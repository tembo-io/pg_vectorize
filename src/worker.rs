use crate::executor::{ColumnJobParams, JobMessage};
use crate::init::PGMQ_QUEUE_NAME;
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

    log!("Starting BG Workers {}", BackgroundWorker::get_name(),);

    // poll at 10s or on a SIGTERM
    while BackgroundWorker::wait_latch(Some(Duration::from_secs(10))) {
        if BackgroundWorker::sighup_received() {
            // on SIGHUP, you might want to reload some external configuration or something
        }
        runtime.block_on(async {
            match queue.read::<JobMessage>(PGMQ_QUEUE_NAME, 300).await {
                Ok(Some(msg)) => {
                    let msg_id = msg.msg_id;
                    log!("Received message: {:?}", msg);
                    let job_meta = msg.message.job_meta;
                    let job_params: ColumnJobParams =
                        serde_json::from_value(job_meta.params).expect("invalid job parameters");
                    let embeddings = match job_meta.transformer {
                        types::Transformer::openai => {
                            log!("OpenAI transformer");
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
                            log!("No transformer found");
                            Err(anyhow::anyhow!("Unsupported transformer"))
                        }
                    };
                    // write embeddings to result table
                    upsert_embedding_table(
                        &conn,
                        &job_params.schema,
                        &job_meta.name,
                        embeddings.expect("failed to get embeddings"),
                    )
                    .await
                    .expect("failed to write embeddings");
                    // delete message from queue
                    queue
                        .delete(PGMQ_QUEUE_NAME, msg_id)
                        .await
                        .expect("failed to delete message");
                    // TODO: update job meta updated_timestamp
                }
                Ok(None) => {
                    log!("No messages in queue");
                }
                _ => {
                    log!("Error reading message");
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
    join_key: String,
    embeddings: Vec<f64>,
}

use crate::executor::Inputs;

// merges the vec of inputs with the embedding responses
fn merge_input_output(inputs: Vec<Inputs>, values: Vec<Vec<f64>>) -> Vec<PairedEmbeddings> {
    inputs
        .into_iter()
        .zip(values.into_iter())
        .map(|(input, value)| PairedEmbeddings {
            join_key: input.record_id,
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
    // // TODO: batch insert
    // let upsert_stmt = format!("
    //     INSERT INTO {schema}.{table}_embeddings
    //     VALUES (record_id, embeddings) values ($1, $2)
    //     ON CONFLICT (record_id)
    //     DO UPDATE SET embeddings = $2;
    //     ;");
    // query.execute(conn).await?;
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
fn build_upsert_query(
    schema: &str,
    project: &str,
    embeddings: Vec<PairedEmbeddings>,
) -> (String, Vec<(String, serde_json::Value)>) {
    let mut query = format!(
        "
        INSERT INTO {schema}.{project}_embeddings (record_id, embeddings) VALUES"
    );
    let mut bindings: Vec<(String, serde_json::Value)> = Vec::new();

    for (index, pair) in embeddings.into_iter().enumerate() {
        if index > 0 {
            query.push(',');
        }
        query.push_str(&format!(" (${}, ${}::jsonb)", 2 * index + 1, 2 * index + 2));

        let embedding =
            serde_json::to_value(&pair.embeddings).expect("failed to serialize embedding");
        bindings.push((pair.join_key, embedding));
    }

    query.push_str(" ON CONFLICT (record_id) DO UPDATE SET embeddings = EXCLUDED.embeddings");
    (query, bindings)
}
