use pgrx::prelude::*;
use pgrx::spi::SpiTupleTable;

use crate::errors::DatabaseError;
use crate::init::{TableMethod, PGMQ_QUEUE_NAME};
use crate::query::check_input;
use crate::types;
use crate::util::{from_env_default, Config};
use chrono::serde::ts_seconds_option::deserialize as from_tsopt;
use chrono::TimeZone;
use serde::{Deserialize, Serialize};
use sqlx::error::Error;
use sqlx::postgres::PgRow;
use sqlx::types::chrono::Utc;
use sqlx::{FromRow, PgPool, Pool, Postgres, Row};

// schema for every job
// also schema for the vectorize.vectorize_meta table
#[derive(Clone, Debug, Deserialize, FromRow, Serialize)]
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
fn job_execute(job_name: String) -> pgrx::JsonB {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    let cfg = Config::default();
    let db_url = cfg.pg_conn_str;

    runtime.block_on(async {
        let conn = PgPool::connect(&db_url)
            .await
            .expect("failed sqlx connection");
        let queue = pgmq::PGMQueueExt::new(db_url, 2)
            .await
            .expect("failed to init db connection");
        let meta = get_vectorize_meta(&job_name, conn)
            .await
            .expect("failed to get job meta");
        let job_params = serde_json::from_value::<ColumnJobParams>(meta.params.clone())
            .expect("failed to deserialize job params");
        let last_completion = match meta.last_completion {
            Some(t) => t,
            None => Utc.with_ymd_and_hms(970, 1, 1, 0, 0, 0).unwrap(),
        };
        let new_or_updated_rows = get_new_updates(job_params, last_completion)
            .await
            .expect("failed to get new updates");
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
                    .expect("failed to send message");
                log!("message sent: {}", msg_id);
            }
            None => {
                log!("Job -- {} -- no new records", job_name);
            }
        };
        pgrx::JsonB(serde_json::to_value(meta).unwrap())
    })
}

// get job meta
pub async fn get_vectorize_meta(
    job_name: &str,
    conn: Pool<Postgres>,
) -> Result<VectorizeMeta, DatabaseError> {
    let row = sqlx::query_as!(
        VectorizeMeta,
        "
        SELECT *
        FROM vectorize.vectorize_meta
        WHERE name = $1
        ",
        job_name.to_string(),
    )
    .fetch_one(&conn)
    .await?;
    Ok(row)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Inputs {
    pub record_id: String, // the value to join the record
    pub inputs: String,    // concatenation of input columns
}

// queries a table and returns rows that need new embeddings
pub async fn get_new_updates(
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

// gets last processed times
fn get_inputs_query(
    job_name: &str,
    schema: &str,
    table: &str,
    columns: Vec<String>,
    last_updated_col: &str,
) -> String {
    let cols = collapse_to_csv(&columns);

    format!(
        "
    SELECT {cols} as input_text
    FROM {schema}.{table}
    WHERE {last_updated_col} > 
    (
        SELECT last_completion
        FROM vectorize_meta
        WHERE name = '{job_name}'
    )::timestamp
    "
    )
}

// retrieves inputs for embedding model
#[pg_extern]
fn get_inputs(
    job_name: &str,
    schema: &str,
    table: &str,
    columns: Vec<String>,
    updated_at_col: &str,
) -> Vec<String> {
    let mut results: Vec<String> = Vec::new();
    let query = get_inputs_query(job_name, schema, table, columns, updated_at_col);
    let _: Result<(), pgrx::spi::Error> = Spi::connect(|mut client: spi::SpiClient<'_>| {
        let tup_table: SpiTupleTable = client.update(&query, None, None)?;
        for row in tup_table {
            let input = row["input_text"]
                .value::<String>()?
                .expect("input column missing");
            results.push(input);
        }
        Ok(())
    });
    results
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
