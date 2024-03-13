use crate::transformers::{generic, http_handler, openai};
use crate::types::{JobMessage, JobParams};
use anyhow::Result;
use pgmq::{Message, PGMQueueExt};
use sqlx::{Pool, Postgres};

pub async fn work(
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
    if read_ct <= 9999 {
        execute_job(conn, msg, config).await?;
    } else {
        println!("message exceed read count")
    }

    queue.archive(&config.queue_name, msg_id).await?;

    Ok(Some(()))
}

pub struct Config {
    pub queue_name: String,
    pub embedding_svc_url: String,
    pub openai_api_key: Option<String>,
    pub embedding_request_timeout: i32,
}

/// processes a single job from the queue
async fn execute_job(
    dbclient: &Pool<Postgres>,
    msg: Message<JobMessage>,
    cfg: &Config,
) -> Result<()> {
    let job_meta = msg.message.job_meta;
    let job_params: JobParams = serde_json::from_value(job_meta.params.clone())?;

    let embedding_request = match job_meta.transformer.as_ref() {
        "text-embedding-ada-002" => openai::prepare_openai_request_no_guc(
            job_meta.clone(),
            &msg.message.inputs,
            cfg.openai_api_key.clone(),
        ),
        _ => {
            println!("other req");

            generic::prepare_generic_embedding_request_no_guc(
                job_meta.clone(),
                &msg.message.inputs,
                cfg.embedding_svc_url.clone(),
            )
        }
    };
    let embeddings =
        http_handler::openai_embedding_request(embedding_request?, cfg.embedding_request_timeout)
            .await?;
    let paired_embeddings = http_handler::merge_input_output(msg.message.inputs, embeddings);

    match job_params.clone().table_method {
        crate::types::TableMethod::append => {
            crate::workers::update_embeddings(
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
            crate::workers::upsert_embedding_table(
                dbclient,
                &job_meta.name,
                &job_params,
                paired_embeddings,
            )
            .await?
        }
    }
    Ok(())
}
