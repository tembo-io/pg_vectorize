pub mod pg_bgw;

use crate::guc::{EMBEDDING_REQ_TIMEOUT_SEC, OPENAI_KEY};
use crate::transformers::generic::get_generic_svc_url;

use vectorize_core::transformers::types::PairedEmbeddings;
use vectorize_core::transformers::{generic, http_handler, ollama, openai};
use vectorize_core::types::{self, ModelSource};
use vectorize_core::worker::ops;

use anyhow::{Context, Result};
use pgmq::{Message, PGMQueueExt};
use pgrx::*;
use sqlx::{Pool, Postgres};

pub async fn run_worker(
    queue: PGMQueueExt,
    conn: &Pool<Postgres>,
    queue_name: &str,
) -> Result<Option<()>> {
    let msg: Message<types::JobMessage> =
        match queue.read::<types::JobMessage>(queue_name, 180_i32).await {
            Ok(Some(msg)) => msg,
            Ok(None) => {
                info!("pg-vectorize: No messages in queue");
                return Ok(None);
            }
            Err(e) => {
                warning!("pg-vectorize: Error reading message: {e}");
                return Err(anyhow::anyhow!("failed to read message"));
            }
        };

    let msg_id: i64 = msg.msg_id;
    let read_ct: i32 = msg.read_ct;
    info!(
        "pg-vectorize: received message for job: {:?}",
        msg.message.job_name
    );
    let job_success = execute_job(conn.clone(), msg).await;
    let delete_it = match job_success {
        Ok(_) => {
            info!("pg-vectorize: job success");
            true
        }
        Err(e) => {
            warning!("pg-vectorize: job failed: {:?}", e);
            read_ct > 2
        }
    };

    // delete message from queue
    if delete_it {
        match queue.archive(queue_name, msg_id).await {
            Ok(_) => {
                info!("pg-vectorize: deleted message: {}", msg_id);
            }
            Err(e) => {
                warning!("pg-vectorize: Error deleting message: {}", e);
            }
        }
    }
    // return Some(), indicating that worker consumed some message
    // any possibly more messages on queue
    Ok(Some(()))
}

async fn execute_job(dbclient: Pool<Postgres>, msg: Message<types::JobMessage>) -> Result<()> {
    let job_meta = msg.message.job_meta;
    let job_params: types::JobParams = serde_json::from_value(job_meta.params.clone())?;

    let embedding_request = match job_meta.transformer.source {
        ModelSource::OpenAI => {
            info!("pg-vectorize: OpenAI transformer");
            let apikey = match job_params.api_key.clone() {
                Some(k) => k,
                None => {
                    let key = match OPENAI_KEY.get() {
                        Some(k) => k.to_str()?.to_owned(),
                        None => {
                            warning!("pg-vectorize: Error getting API key from GUC");
                            return Err(anyhow::anyhow!("failed to get API key"));
                        }
                    };
                    key
                }
            };
            openai::prepare_openai_request(job_meta.clone(), &msg.message.inputs, Some(apikey))
        }
        ModelSource::SentenceTransformers => {
            let svc_host =
                get_generic_svc_url().context("failed to get embedding service url from GUC")?;
            generic::prepare_generic_embedding_request(
                job_meta.clone(),
                &msg.message.inputs,
                svc_host,
            )
        }
        ModelSource::Ollama => error!("pg-vectorize: Ollama transformer not implemented yet"),
    }?;

    let timeout = EMBEDDING_REQ_TIMEOUT_SEC.get();
    let embeddings = http_handler::openai_embedding_request(embedding_request, timeout).await?;
    // TODO: validate returned embeddings order is same as the input order
    let paired_embeddings: Vec<PairedEmbeddings> =
        http_handler::merge_input_output(msg.message.inputs, embeddings);

    log!("pg-vectorize: embeddings size: {}", paired_embeddings.len());
    // write embeddings to result table
    match job_params.clone().table_method {
        types::TableMethod::append => {
            ops::update_embeddings(
                &dbclient,
                &job_params.schema,
                &job_params.table,
                &job_meta.clone().name,
                &job_params.primary_key,
                &job_params.pkey_type,
                paired_embeddings,
            )
            .await?;
        }
        types::TableMethod::join => {
            ops::upsert_embedding_table(&dbclient, &job_meta.name, &job_params, paired_embeddings)
                .await?
        }
    };
    Ok(())
}
