use pgrx::prelude::*;

use crate::errors::DatabaseError;
use crate::init::{TableMethod, PGMQ_QUEUE_NAME};
use crate::query::check_input;
use crate::types;
use crate::util::{from_env_default, get_pg_conn};
use chrono::serde::ts_seconds_option::deserialize as from_tsopt;
use chrono::TimeZone;
use serde::{Deserialize, Serialize};
use sqlx::error::Error;
use sqlx::postgres::PgRow;
use sqlx::types::chrono::Utc;
use sqlx::{FromRow, PgPool, Pool, Postgres, Row};

// schema for every job
// also schema for the vectorize.vectorize_meta table
#[derive(Clone, Debug, Deserialize, FromRow, Serialize, PostgresType)]
pub struct VectorizeMeta {
    pub job_id: i64,
    pub name: String,
    pub job_type: types::JobType,
    pub transformer: types::Transformer,
    pub search_alg: types::SimilarityAlg,
    pub params: serde_json::Value,
    #[serde(deserialize_with = "from_tsopt")]
    pub last_completion: Option<chrono::DateTime<Utc>>,
}

// temporary struct for deserializing from db
// not needed when sqlx 0.7.x
#[derive(Clone, Debug, Deserialize, FromRow, Serialize, PostgresType)]
pub struct _VectorizeMeta {
    pub job_id: i64,
    pub name: String,
    pub job_type: String,
    pub transformer: String,
    pub search_alg: String,
    pub params: serde_json::Value,
    #[serde(deserialize_with = "from_tsopt")]
    pub last_completion: Option<chrono::DateTime<Utc>>,
}

impl From<_VectorizeMeta> for VectorizeMeta {
    fn from(val: _VectorizeMeta) -> Self {
        VectorizeMeta {
            job_id: val.job_id,
            name: val.name,
            job_type: types::JobType::from(val.job_type),
            transformer: types::Transformer::from(val.transformer),
            search_alg: types::SimilarityAlg::from(val.search_alg),
            params: val.params,
            last_completion: val.last_completion,
        }
    }
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct ColumnJobParams {
    pub schema: String,
    pub table: String,
    pub columns: Vec<String>,
    pub primary_key: String,
    pub pkey_type: String,
    pub update_time_col: String,
    pub api_key: Option<String>,
    pub table_method: TableMethod,
}

// schema for all messages that hit pgmq
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct JobMessage {
    pub job_name: String,
    pub job_meta: VectorizeMeta,
    pub inputs: Vec<Inputs>,
}

// called by pg_cron on schedule
// identifiers new inputs and enqueues them
#[pg_extern]
#[pg_guard]
fn job_execute(job_name: String) {
    log!("pg-vectorize: refresh job: {}", job_name);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap_or_else(|e| error!("failed to initialize tokio runtime: {}", e));

    runtime.block_on(async {
        let conn = get_pg_conn().await.unwrap_or_else(|e| error!("pg-vectorize: failed to establsh db connection: {}", e));
        let queue = pgmq::PGMQueueExt::new_with_pool(conn.clone())
            .await
            .unwrap_or_else(|e| error!("failed to init db connection: {}", e));
        let meta = get_vectorize_meta(&job_name, &conn)
            .await
            .unwrap_or_else(|e| error!("failed to get job metadata: {}", e));
        let job_params = serde_json::from_value::<ColumnJobParams>(meta.params.clone())
            .unwrap_or_else(|e| error!("failed to deserialize job params: {}", e));
        let _last_completion = match meta.last_completion {
            Some(t) => t,
            None => Utc.with_ymd_and_hms(970, 1, 1, 0, 0, 0).unwrap(),
        };
        let new_or_updated_rows = get_new_updates_append(&conn, &job_name, job_params)
            .await
            .unwrap_or_else(|e| error!("failed to get new updates: {}", e));
        match new_or_updated_rows {
            Some(rows) => {
                log!("num new records: {}", rows.len());
                let msg = JobMessage {
                    job_name: job_name.clone(),
                    job_meta: meta.clone(),
                    inputs: rows,
                };
                let msg_id = queue
                    .send(PGMQ_QUEUE_NAME, &msg)
                    .await
                    .unwrap_or_else(|e| error!("failed to send message updates: {}", e));
                log!("message sent: {}", msg_id);
            }
            None => {
                log!("pg-vectorize: job: {}, no new records", job_name);
            }
        };
    })
}

// get job meta
pub async fn get_vectorize_meta(
    job_name: &str,
    conn: &Pool<Postgres>,
) -> Result<VectorizeMeta, DatabaseError> {
    log!("fetching job: {}", job_name);
    let row = sqlx::query_as!(
        _VectorizeMeta,
        "
        SELECT *
        FROM vectorize.vectorize_meta
        WHERE name = $1
        ",
        job_name.to_string(),
    )
    .fetch_one(conn)
    .await?;
    Ok(row.into())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Inputs {
    pub record_id: String, // the value to join the record
    pub inputs: String,    // concatenation of input columns
}

// queries a table and returns rows that need new embeddings
// used for the TableMethod::append, which has source and embedding on the same table
pub async fn get_new_updates_append(
    pool: &Pool<Postgres>,
    job_name: &str,
    job_params: ColumnJobParams,
) -> Result<Option<Vec<Inputs>>, DatabaseError> {
    let cols = collapse_to_csv(&job_params.columns);

    // query source and return any new rows that need transformation
    // return any row where last updated embedding is also null (never populated)
    let new_rows_query = format!(
        "
        SELECT 
            {record_id}::text as record_id,
            {cols} as input_text
        FROM {schema}.{table}
        WHERE {updated_at_col} > COALESCE
            (
                {job_name}_updated_at::timestamp,
                '0001-01-01 00:00:00'::timestamp
            );
    ",
        record_id = job_params.primary_key,
        schema = job_params.schema,
        table = job_params.table,
        updated_at_col = job_params.update_time_col,
    );

    let rows: Result<Vec<PgRow>, Error> = sqlx::query(&new_rows_query).fetch_all(pool).await;
    match rows {
        Ok(rows) => {
            if !rows.is_empty() {
                let mut new_inputs: Vec<Inputs> = Vec::new();
                for r in rows {
                    new_inputs.push(Inputs {
                        record_id: r.get("record_id"),
                        inputs: r.get("input_text"),
                    })
                }
                log!("pg-vectorize: num new inputs: {}", new_inputs.len());
                Ok(Some(new_inputs))
            } else {
                Ok(None)
            }
        }
        Err(sqlx::error::Error::RowNotFound) => Ok(None),
        Err(e) => Err(e)?,
    }
}

// queries a table and returns rows that need new embeddings
#[allow(dead_code)]
pub async fn get_new_updates_shared(
    job_params: ColumnJobParams,
    last_completion: chrono::DateTime<Utc>,
) -> Result<Option<Vec<Inputs>>, DatabaseError> {
    let pool = PgPool::connect(&from_env_default(
        "DATABASE_URL",
        "postgres:://postgres:postgres@localhost:5432/",
    ))
    .await?;

    let cols = collapse_to_csv(&job_params.columns);

    // query source and return any new rows that need transformation
    let new_rows_query = format!(
        "
        SELECT 
            {record_id}::text as record_id,
            {cols} as input_text
        FROM {schema}.{table}
        WHERE {updated_at_col} > '{last_completion}'::timestamp;
    ",
        record_id = job_params.primary_key,
        schema = job_params.schema,
        table = job_params.table,
        updated_at_col = job_params.update_time_col,
    );

    let mut new_inputs: Vec<Inputs> = Vec::new();

    let rows: Result<Vec<PgRow>, Error> = sqlx::query(&new_rows_query).fetch_all(&pool).await;
    match rows {
        Ok(rows) => {
            for r in rows {
                new_inputs.push(Inputs {
                    record_id: r.get("record_id"),
                    inputs: r.get("input_text"),
                })
            }
            Ok(Some(new_inputs))
        }
        Err(sqlx::error::Error::RowNotFound) => Ok(None),
        Err(e) => Err(e)?,
    }
}

fn collapse_to_csv(strings: &[String]) -> String {
    strings
        .iter()
        .map(|s| {
            check_input(s).expect("Failed to validate input");
            s.as_str()
        })
        .collect::<Vec<_>>()
        .join("|| ', ' ||")
}
