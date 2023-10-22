use crate::executor::{ColumnJobParams, JobMessage};
use crate::init::{TableMethod, PGMQ_QUEUE_NAME};
use crate::openai;
use crate::types;
use crate::util::get_pg_conn;
use anyhow::Result;
use pgmq::Message;
use pgrx::bgworkers::*;
use pgrx::prelude::*;
use sqlx::{Pool, Postgres};
use std::time::Duration;

#[pg_guard]
pub extern "C" fn _PG_init() {
    BackgroundWorkerBuilder::new("PG Vectorize Background Worker")
        .set_function("background_worker_main")
        .set_library("vectorize")
        .enable_spi_access()
        .load();
}

#[pg_guard]
#[no_mangle]
pub extern "C" fn background_worker_main(_arg: pg_sys::Datum) {
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP | SignalWakeFlags::SIGTERM);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    // specify database
    let (conn, queue) = runtime.block_on(async {
        let con = get_pg_conn().await.expect("failed to connect to database");
        let queue = pgmq::PGMQueueExt::new_with_pool(con.clone())
            .await
            .expect("failed to init db connection");
        (con, queue)
    });

    log!("Starting BG Workers {}", BackgroundWorker::get_name(),);

    while BackgroundWorker::wait_latch(Some(Duration::from_secs(5))) {
        if BackgroundWorker::sighup_received() {
            // on SIGHUP, you might want to reload configurations and env vars
        }
        let _: Result<()> = runtime.block_on(async {
            let msg: Message<JobMessage> =
                match queue.read::<JobMessage>(PGMQ_QUEUE_NAME, 300).await {
                    Ok(Some(msg)) => msg,
                    Ok(None) => {
                        log!("pg-vectorize: No messages in queue");
                        return Ok(());
                    }
                    Err(e) => {
                        warning!("pg-vectorize: Error reading message: {e}");
                        return Ok(());
                    }
                };

            let msg_id = msg.msg_id;
            let read_ct = msg.read_ct;
            log!(
                "pg-vectorize: received message for job: {:?}",
                msg.message.job_name
            );
            let job_success = execute_job(conn.clone(), msg).await;
            let delete_it = if job_success.is_ok() {
                true
            } else {
                read_ct > 2
            };

            // delete message from queue
            if delete_it {
                match queue.delete(PGMQ_QUEUE_NAME, msg_id).await {
                    Ok(_) => {
                        log!("pg-vectorize: deleted message: {}", msg_id);
                    }
                    Err(e) => {
                        warning!("pg-vectorize: Error deleting message: {}", e);
                    }
                }
            }
            // TODO: update job meta updated_timestamp
            Ok(())
        });
    }
    log!("pg-vectorize: shutting down");
}

struct PairedEmbeddings {
    primary_key: String,
    embeddings: Vec<f64>,
}

use crate::executor::Inputs;

// merges the vec of inputs with the embedding responses
fn merge_input_output(inputs: Vec<Inputs>, values: Vec<Vec<f64>>) -> Vec<PairedEmbeddings> {
    inputs
        .into_iter()
        .zip(values.into_iter())
        .map(|(input, value)| PairedEmbeddings {
            primary_key: input.record_id,
            embeddings: value,
        })
        .collect()
}

async fn upsert_embedding_table(
    conn: &Pool<Postgres>,
    schema: &str,
    project: &str,
    embeddings: Vec<PairedEmbeddings>,
) -> Result<()> {
    let (query, bindings) = build_upsert_query(schema, project, embeddings);
    let mut q = sqlx::query(&query);
    for (record_id, embeddings) in bindings {
        q = q.bind(record_id).bind(embeddings);
    }
    match q.execute(conn).await {
        Ok(_) => Ok(()),
        Err(e) => {
            log!("Error: {}", e);
            Err(anyhow::anyhow!("failed to execute query"))
        }
    }
}

// returns query and bindings
// only compatible with pg-vector data types
fn build_upsert_query(
    schema: &str,
    project: &str,
    embeddings: Vec<PairedEmbeddings>,
) -> (String, Vec<(String, String)>) {
    let mut query = format!(
        "
        INSERT INTO {schema}.{project}_embeddings (record_id, embeddings) VALUES"
    );
    let mut bindings: Vec<(String, String)> = Vec::new();

    for (index, pair) in embeddings.into_iter().enumerate() {
        if index > 0 {
            query.push(',');
        }
        query.push_str(&format!(
            " (${}, ${}::vector)",
            2 * index + 1,
            2 * index + 2
        ));

        let embedding =
            serde_json::to_string(&pair.embeddings).expect("failed to serialize embedding");
        bindings.push((pair.primary_key, embedding));
    }

    query.push_str(" ON CONFLICT (record_id) DO UPDATE SET embeddings = EXCLUDED.embeddings");
    (query, bindings)
}

use serde_json::to_string;

async fn update_append_table(
    pool: &Pool<Postgres>,
    embeddings: Vec<PairedEmbeddings>,
    schema: &str,
    table: &str,
    project: &str,
    pkey: &str,
    pkey_type: &str,
) -> anyhow::Result<()> {
    for embed in embeddings {
        // Serialize the Vec<f64> to a JSON string
        let embedding = to_string(&embed.embeddings).expect("failed to serialize embedding");

        // TODO: pkey might not always be integer type
        let update_query = format!(
            "
            UPDATE {schema}.{table}
            SET 
                {project}_embeddings = $1::vector,
                {project}_updated_at = (NOW() at time zone 'utc')
            WHERE {pkey} = $2::{pkey_type}
        "
        );
        // Prepare and execute the update statement for this pair within the transaction
        sqlx::query(&update_query)
            .bind(embedding)
            .bind(embed.primary_key)
            .execute(pool)
            .await?;
    }
    Ok(())
}

async fn execute_job(dbclient: Pool<Postgres>, msg: Message<JobMessage>) -> Result<()> {
    let job_meta = msg.message.job_meta;
    let job_params: ColumnJobParams =
        serde_json::from_value(job_meta.params).expect("invalid job parameters");
    let embeddings: Result<Vec<PairedEmbeddings>> = match job_meta.transformer {
        types::Transformer::openai => {
            log!("pg-vectorize: OpenAI transformer");
            let text_inputs: Vec<String> = msg
                .message
                .inputs
                .clone()
                .into_iter()
                .map(|v| v.inputs)
                .collect();
            let apikey = match job_params
                .api_key
                .ok_or_else(|| anyhow::anyhow!("missing api key"))
            {
                Ok(k) => k,
                Err(e) => {
                    warning!("pg-vectorize: Error getting api key: {}", e);
                    return Err(anyhow::anyhow!("failed to get api key"));
                }
            };

            let embeddings = match openai::get_embeddings(&text_inputs, &apikey).await {
                Ok(e) => e,
                Err(e) => {
                    warning!("pg-vectorize: Error getting embeddings: {}", e);
                    return Err(anyhow::anyhow!("failed to get embeddings"));
                }
            };
            // TODO: validate returned embeddings order is same as the input order
            let emb: Vec<PairedEmbeddings> = merge_input_output(msg.message.inputs, embeddings);
            Ok(emb)
        }
    };
    // write embeddings to result table
    match job_params.table_method {
        TableMethod::append => {
            update_append_table(
                &dbclient,
                embeddings.expect("failed to get embeddings"),
                &job_params.schema,
                &job_params.table,
                &job_meta.name,
                &job_params.primary_key,
                &job_params.pkey_type,
            )
            .await?;
        }
        TableMethod::join => {
            upsert_embedding_table(
                &dbclient,
                &job_params.schema,
                &job_meta.name,
                embeddings.expect("failed to get embeddings"),
            )
            .await?
        }
    };
    Ok(())
}
