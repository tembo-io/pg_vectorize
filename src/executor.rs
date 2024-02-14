use pgrx::prelude::*;

use crate::errors::DatabaseError;
use crate::guc::BATCH_SIZE;
use crate::init::VECTORIZE_QUEUE;
use crate::query::check_input;
use crate::transformers::types::Inputs;
use crate::types;
use crate::util::get_pg_conn;
use chrono::serde::ts_seconds_option::deserialize as from_tsopt;
use chrono::TimeZone;
use serde::{Deserialize, Serialize};
use sqlx::error::Error;
use sqlx::postgres::PgRow;
use sqlx::types::chrono::Utc;
use sqlx::{FromRow, Pool, Postgres, Row};
use tiktoken_rs::cl100k_base;

// schema for every job
// also schema for the vectorize.vectorize_meta table
#[derive(Clone, Debug, Deserialize, FromRow, Serialize)]
pub struct VectorizeMeta {
    pub job_id: i64,
    pub name: String,
    pub job_type: types::JobType,
    pub transformer: String,
    pub search_alg: types::SimilarityAlg,
    pub params: serde_json::Value,
    #[serde(deserialize_with = "from_tsopt")]
    pub last_completion: Option<chrono::DateTime<Utc>>,
}

// creates batches based on total token count
// batch_size is the max token count per batch
pub fn create_batches(data: Vec<Inputs>, batch_size: i32) -> Vec<Vec<Inputs>> {
    let mut groups: Vec<Vec<Inputs>> = Vec::new();
    let mut current_group: Vec<Inputs> = Vec::new();
    let mut current_token_count = 0;

    for input in data {
        if current_token_count + input.token_estimate > batch_size {
            // Create a new group
            groups.push(current_group);
            current_group = Vec::new();
            current_token_count = 0;
        }
        current_token_count += input.token_estimate;
        current_group.push(input);
    }

    // Add any remaining inputs to the groups
    if !current_group.is_empty() {
        groups.push(current_group);
    }
    groups
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

    let max_batch_size = BATCH_SIZE.get();

    runtime.block_on(async {
        let conn = get_pg_conn()
            .await
            .unwrap_or_else(|e| error!("pg-vectorize: failed to establish db connection: {}", e));
        let queue = pgmq::PGMQueueExt::new_with_pool(conn.clone())
            .await
            .unwrap_or_else(|e| error!("failed to init db connection: {}", e));
        let meta = get_vectorize_meta(&job_name, &conn)
            .await
            .unwrap_or_else(|e| error!("failed to get job metadata: {}", e));
        let job_params = serde_json::from_value::<types::JobParams>(meta.params.clone())
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
                info!("num new records: {}", rows.len());
                let batches = create_batches(rows, max_batch_size);
                info!(
                    "total batches: {}, max_batch_size: {}",
                    batches.len(),
                    max_batch_size
                );
                for b in batches {
                    let msg = JobMessage {
                        job_name: job_name.clone(),
                        job_meta: meta.clone(),
                        inputs: b,
                    };
                    let msg_id = queue
                        .send(VECTORIZE_QUEUE, &msg)
                        .await
                        .unwrap_or_else(|e| error!("failed to send message updates: {}", e));
                    log!("message sent: {}", msg_id);
                }
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
    let row = sqlx::query_as!(
        VectorizeMeta,
        "
        SELECT *
        FROM vectorize.job
        WHERE name = $1
        ",
        job_name.to_string(),
    )
    .fetch_one(conn)
    .await?;
    Ok(row)
}

pub fn new_rows_query(job_name: &str, job_params: &types::JobParams) -> String {
    let cols = collapse_to_csv(&job_params.columns);

    // query source and return any new rows that need transformation
    // return any row where last updated embedding is also null (never populated)
    let base_query = format!(
        "
        SELECT 
        {record_id}::text as record_id,
        {cols} as input_text
        FROM {schema}.{table}
        ",
        record_id = job_params.primary_key,
        schema = job_params.schema,
        table = job_params.table,
    );
    if let Some(updated_at_col) = &job_params.update_time_col {
        // updated_at_column is not required when `schedule` is realtime
        let where_clause = format!(
            "
            WHERE {updated_at_col} > COALESCE
            (
                {job_name}_updated_at::timestamp,
                '0001-01-01 00:00:00'::timestamp
            )",
        );
        format!(
            "
            {base_query}
            {where_clause}
        "
        )
    } else {
        base_query
    }
}

// queries a table and returns rows that need new embeddings
// used for the TableMethod::append, which has source and embedding on the same table
pub async fn get_new_updates_append(
    pool: &Pool<Postgres>,
    job_name: &str,
    job_params: types::JobParams,
) -> Result<Option<Vec<Inputs>>, DatabaseError> {
    let query = new_rows_query(job_name, &job_params);

    let rows: Result<Vec<PgRow>, Error> = sqlx::query(&query).fetch_all(pool).await;
    match rows {
        Ok(rows) => {
            if !rows.is_empty() {
                let bpe = cl100k_base().unwrap();
                let mut new_inputs: Vec<Inputs> = Vec::new();
                for r in rows {
                    let ipt: String = r.get("input_text");
                    let token_estimate = bpe.encode_with_special_tokens(&ipt).len() as i32;
                    new_inputs.push(Inputs {
                        record_id: r.get("record_id"),
                        inputs: ipt,
                        token_estimate,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_batches_normal() {
        let data = vec![
            Inputs {
                record_id: "1".to_string(),
                inputs: "Test 1.".to_string(),
                token_estimate: 2,
            },
            Inputs {
                record_id: "2".to_string(),
                inputs: "Test 2.".to_string(),
                token_estimate: 2,
            },
            Inputs {
                record_id: "3".to_string(),
                inputs: "Test 3.".to_string(),
                token_estimate: 3,
            },
        ];

        let batches = create_batches(data, 4);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[1].len(), 1);
    }

    #[test]
    fn test_create_batches_empty() {
        let data: Vec<Inputs> = Vec::new();
        let batches = create_batches(data, 4);
        assert_eq!(batches.len(), 0);
    }

    #[test]
    fn test_create_batches_large() {
        let data = vec![
            Inputs {
                record_id: "1".to_string(),
                inputs: "Test 1.".to_string(),
                token_estimate: 2,
            },
            Inputs {
                record_id: "2".to_string(),
                inputs: "Test 2.".to_string(),
                token_estimate: 2,
            },
            Inputs {
                record_id: "3".to_string(),
                inputs: "Test 3.".to_string(),
                token_estimate: 100,
            },
        ];
        let batches = create_batches(data, 5);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[1].len(), 1);
        assert_eq!(batches[1][0].token_estimate, 100);
    }
}
