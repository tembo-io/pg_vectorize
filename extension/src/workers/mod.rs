pub mod pg_bgw;

use crate::guc::{get_guc_configs, ModelGucConfig};

use anyhow::Result;
use pgmq::{Message, PGMQueueExt};
use pgrx::*;
use sqlx::{Pool, Postgres};
use vectorize_core::transformers::http_handler;
use vectorize_core::transformers::providers;
use vectorize_core::transformers::types::PairedEmbeddings;
use vectorize_core::types;
use vectorize_core::worker::ops;

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
        match queue.delete(queue_name, msg_id).await {
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
    let mut job_params: types::JobParams = serde_json::from_value(job_meta.params.clone())?;

    let embedding_request =
        providers::prepare_generic_embedding_request(&job_meta.transformer, &msg.message.inputs);

    let guc_configs: ModelGucConfig = get_guc_configs(&job_meta.transformer.source);

    // if api_key found in GUC, then use that and re-assign
    if let Some(k) = guc_configs.api_key {
        job_params.api_key = Some(k);
    }

    let provider = providers::get_provider(
        &job_meta.transformer.source,
        job_params.api_key.clone(),
        guc_configs.service_url,
    )?;

    let embedding_response = provider.generate_embedding(&embedding_request).await?;
    let paired_embeddings: Vec<PairedEmbeddings> =
        http_handler::merge_input_output(msg.message.inputs, embedding_response.embeddings);

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
