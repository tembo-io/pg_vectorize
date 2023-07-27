use crate::executor::{ColumnJobParams, JobMessage};
use crate::init::{TableMethod, PGMQ_QUEUE_NAME};
use crate::openai;
use crate::types;
use crate::util::Config;
use pgrx::bgworkers::*;
use pgrx::prelude::*;
use sqlx::{PgPool, Pool, Postgres};
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
    let cfg = Config::default();
    log!("database url: {}", cfg.pg_conn_str);
    let (conn, queue) = runtime.block_on(async {
        let conn = PgPool::connect(&cfg.pg_conn_str)
            .await
            .expect("failed sqlx connection");
        let queue = pgmq::PGMQueueExt::new(cfg.pg_conn_str, 4)
            .await
            .expect("failed to init db connection");
        (conn, queue)
    });

    log!(
        "pg-vectorize: starting bg workers: {}",
        BackgroundWorker::get_name(),
    );

    // poll at 10s or on a SIGTERM
    while BackgroundWorker::wait_latch(Some(Duration::from_secs(5))) {
        if BackgroundWorker::sighup_received() {
            // on SIGHUP, you might want to reload some external configuration or something
        }
        runtime.block_on(async {
            match queue.read::<JobMessage>(PGMQ_QUEUE_NAME, 300).await {
                Ok(Some(msg)) => {
                    let msg_id = msg.msg_id;
                    log!(
                        "pg-vectorize: received message for job: {:?}",
                        msg.message.job_name
                    );
                    let job_meta = msg.message.job_meta;
                    let job_params: ColumnJobParams =
                        serde_json::from_value(job_meta.params).expect("invalid job parameters");
                    let embeddings = match job_meta.transformer {
                        types::Transformer::openai => {
                            log!("pg-vectorize: OpenAI transformer");
                            let text_inputs: Vec<String> = msg
                                .message
                                .inputs
                                .clone()
                                .into_iter()
                                .map(|v| v.inputs)
                                .collect();
                            let embeddings = openai::get_embeddings(
                                &text_inputs,
                                &job_params.api_key.expect("missing api key"),
                            )
                            .await;
                            // TODO: validate returned embeddings order is same as the input order
                            Ok(merge_input_output(msg.message.inputs, embeddings))
                        }
                        _ => {
                            log!("pg-vectorize: No transformer found");
                            Err(anyhow::anyhow!("Unsupported transformer"))
                        }
                    };
                    // write embeddings to result table
                    match job_params.table_method {
                        TableMethod::append => {
                            log!("Append method");
                            update_append_table(
                                &conn,
                                embeddings.expect("failed to get embeddings"),
                                &job_params.schema,
                                &job_params.table,
                                &job_meta.name,
                                &job_params.primary_key,
                                &job_params.pkey_type,
                            )
                            .await
                            .expect("failed to write embeddings");
                        }
                        TableMethod::join => {
                            log!("Join method");
                            upsert_embedding_table(
                                &conn,
                                &job_params.schema,
                                &job_meta.name,
                                embeddings.expect("failed to get embeddings"),
                            )
                            .await
                            .expect("failed to write embeddings");
                        }
                    };
                    // delete message from queue
                    queue
                        .delete(PGMQ_QUEUE_NAME, msg_id)
                        .await
                        .expect("failed to delete message");
                    // TODO: update job meta updated_timestamp
                }
                Ok(None) => {
                    log!("pg-vectorize: No messages in queue");
                }
                _ => {
                    log!("pg-vectorize: Error reading message");
                }
            }
        });
    }

    log!(
        "Goodbye from inside the {} BGWorker! ",
        BackgroundWorker::get_name()
    );
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
) -> anyhow::Result<()> {
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
