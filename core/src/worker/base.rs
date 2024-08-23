use crate::transformers::{http_handler, providers};
use crate::types::{JobMessage, JobParams};
use crate::worker::ops;
use anyhow::Result;
use log::error;
use pgmq::{Message, PGMQueueExt};
use sqlx::{Pool, Postgres};
use std::env;

use crate::types::VectorizeMeta;

pub async fn poll_job(
    conn: &Pool<Postgres>,
    queue: &PGMQueueExt,
    config: &Config,
) -> Result<Option<()>> {
    let msg: Message<JobMessage> = match queue.read::<JobMessage>(&config.queue_name, 1_i32).await {
        Ok(Some(msg)) => msg,
        Ok(None) => {
            return Ok(None);
        }
        Err(e) => {
            return Err(anyhow::anyhow!("failed reading message: {}", e));
        }
    };

    let read_ct: i32 = msg.read_ct;
    let msg_id: i64 = msg.msg_id;
    if read_ct <= config.max_retries {
        execute_job(conn, msg, config).await?;
    } else {
        error!(
            "message exceeds max retry of {}, archiving msg_id: {}",
            config.max_retries, msg_id
        );
    }

    queue.archive(&config.queue_name, msg_id).await?;

    Ok(Some(()))
}

pub struct Config {
    pub database_url: String,
    pub queue_name: String,
    pub embedding_svc_url: String,
    pub openai_api_key: Option<String>,
    pub ollama_svc_url: String,
    pub embedding_request_timeout: i32,
    pub poll_interval: u64,
    pub poll_interval_error: u64,
    pub max_retries: i32,
}

impl Config {
    pub fn from_env() -> Config {
        Config {
            database_url: from_env_default(
                "DATABASE_URL",
                "postgres://postgres:postgres@localhost:5432/postgres",
            ),
            queue_name: from_env_default("VECTORIZE_QUEUE", "vectorize_jobs"),
            embedding_svc_url: from_env_default(
                "EMBEDDING_SVC_URL",
                "http://localhost:3000/v1/embeddings",
            ),
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            ollama_svc_url: from_env_default("OLLAMA_SVC_URL", "http://localhost:3001"),
            embedding_request_timeout: from_env_default("EMBEDDING_REQUEST_TIMEOUT", "6")
                .parse()
                .unwrap(),
            // time to wait between polling for job when there are no messages in queue
            poll_interval: from_env_default("POLL_INTERVAL", "2").parse().unwrap(),
            // time to wait between polling for job when there has been an error in processing
            poll_interval_error: from_env_default("POLL_INTERVAL_ERROR", "10")
                .parse()
                .unwrap(),
            max_retries: from_env_default("MAX_RETRIES", "2").parse().unwrap(),
        }
    }
}

/// source a variable from environment - use default if not exists
pub fn from_env_default(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// processes a single job from the queue
async fn execute_job(
    dbclient: &Pool<Postgres>,
    msg: Message<JobMessage>,
    _cfg: &Config,
) -> Result<()> {
    let job_meta: VectorizeMeta = msg.message.job_meta;
    let job_params: JobParams = serde_json::from_value(job_meta.params.clone())?;

    let provider = providers::get_provider(
        &job_meta.transformer.source,
        job_params.api_key.clone(),
        None,
        None,
    )?;

    let embedding_request =
        providers::prepare_generic_embedding_request(&job_meta.transformer, &msg.message.inputs);

    let embeddings = provider.generate_embedding(&embedding_request).await?;

    let paired_embeddings =
        http_handler::merge_input_output(msg.message.inputs, embeddings.embeddings);
    match job_params.clone().table_method {
        crate::types::TableMethod::append => {
            ops::update_embeddings(
                dbclient,
                &job_params.schema,
                &job_params.table,
                &job_meta.clone().name,
                &job_params.primary_key,
                &job_params.pkey_type,
                paired_embeddings,
            )
            .await?;
        }
        crate::types::TableMethod::join => {
            ops::upsert_embedding_table(dbclient, &job_meta.name, &job_params, paired_embeddings)
                .await?
        }
    }
    Ok(())
}
