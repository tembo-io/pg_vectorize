pub mod pg_bgw;

use crate::executor::JobMessage;
use crate::init::PGMQ_QUEUE_NAME;
use crate::openai;
use crate::types;
use anyhow::Result;
use pgmq::{Message, PGMQueueExt};
use pgrx::*;
use sqlx::{Pool, Postgres};

pub async fn run_worker(queue: PGMQueueExt, conn: Pool<Postgres>) -> Result<()> {
    let msg: Message<JobMessage> = match queue.pop::<JobMessage>(PGMQ_QUEUE_NAME).await {
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
    Ok(())
}

async fn upsert_embedding_table(
    conn: &Pool<Postgres>,
    schema: &str,
    project: &str,
    embeddings: Vec<types::PairedEmbeddings>,
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
    embeddings: Vec<types::PairedEmbeddings>,
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
    embeddings: Vec<types::PairedEmbeddings>,
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
    let job_params: types::JobParams = serde_json::from_value(job_meta.params)?;
    let embeddings: Result<Vec<types::PairedEmbeddings>> = match job_meta.transformer {
        types::Transformer::openai => {
            log!("pg-vectorize: OpenAI transformer");

            let embeddings =
                openai::openai_transform(job_params.clone(), &msg.message.inputs).await?;
            // TODO: validate returned embeddings order is same as the input order
            let emb: Vec<types::PairedEmbeddings> =
                openai::merge_input_output(msg.message.inputs, embeddings);
            Ok(emb)
        }
        types::Transformer::allMiniLML12v2 => {
            log!("pg-vectorize: allMiniLML12v2 transformer");
            todo!()
        }
    };
    // write embeddings to result table
    match job_params.table_method {
        types::TableMethod::append => {
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
        types::TableMethod::join => {
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
