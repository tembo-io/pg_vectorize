use pgrx::prelude::*;
use pgrx::spi::SpiTupleTable;

mod errors;
mod executor;
mod init;
mod query;
mod types;
mod util;
mod worker;

pgrx::pg_module_magic!();

extension_sql!(
    r#"

CREATE TABLE extensions (
    ext_id INT,
    ext_name TEXT,
    summary TEXT,
    last_updated_at TIMESTAMP WITH TIME ZONE DEFAULT (now() at time zone 'utc') NOT NULL
);

INSERT INTO extensions(ext_id, ext_name, summary) VALUES (1, 'pg_jsonschema ', 'pg_jsonschema is a PostgreSQL extension adding support for JSON schema validation on json and jsonb data types.');
INSERT INTO extensions(ext_id, ext_name, summary) VALUES (2, 'pgmq ', 'A lightweight distributed message queue. Like AWS SQS and RSMQ but on Postgres. Features
Lightweight - Built with Rust and Postgres only
Guaranteed "exactly once" delivery of messages consumer within a visibility timeout
API parity with AWS SQS and RSMQ
Messages stay in the queue until deleted
Messages can be archived, instead of deleted, for long-term retention and replayability
Table (bloat) maintenance automated with pg_partman
High performance operations with index-only scans.');
INSERT INTO extensions(ext_id, ext_name, summary) VALUES (3, 'pg_cron ', 'pg_cron is a simple cron-based job scheduler for PostgreSQL (10 or higher) that runs inside the database as an extension. It uses the same syntax as regular cron, but it allows you to schedule PostgreSQL commands directly from the database. You can also use [1-59] seconds to schedule a job based on an interval.'
);
"#,
    name = "init",
);

extension_sql_file!("../sql/meta.sql");

#[pg_extern]
fn enqueue_event(job_name: &str, event_type: &str) {
    // queries the meta table to get the job_type for this name
    // given this job type, create the appropriate message to send to pgmq
    // send the message to pgmq
}

#[pg_extern]
fn pg_openai_embed(key: &str) -> bool {
    // fn pg_openai_embed(schema: &str, table: &str, column: &str, key: &str) -> String {
    let schema = "public";
    let table = "extensions";
    let column = "summary";
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    let inputs = get_inputs(schema, table, column);
    let embeddings = runtime.block_on(async {
        let embeddings = get_embeddings(&inputs, &key).await;
        log!("embeddings: {:?}", embeddings);
        embeddings
    });
    upsert_embedding_table(schema, table, embeddings).unwrap();
    true
}

fn get_inputs(schema: &str, table: &str, column: &str) -> Vec<String> {
    let mut results: Vec<String> = Vec::new();
    let query = format!("select {column} from {schema}.{table}",);
    let _: Result<(), pgrx::spi::Error> = Spi::connect(|mut client| {
        let mut tup_table: SpiTupleTable = client.update(&query, None, None)?;
        while let Some(row) = tup_table.next() {
            let input = row[column]
                .value::<String>()?
                .expect("input column missing");
            results.push(input);
        }
        Ok(())
    });
    results
}

#[derive(serde::Deserialize, Debug)]
struct EmbeddingResponse {
    object: String,
    data: Vec<DataObject>,
}

#[derive(serde::Deserialize, Debug)]
struct DataObject {
    object: String,
    index: usize,
    embedding: Vec<f64>,
}

async fn get_embeddings(inputs: &Vec<String>, key: &str) -> Vec<Vec<f64>> {
    use serde_json::json;
    let url = "https://api.openai.com/v1/embeddings";
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .json(&json!({
            "input": inputs,
            "model": "text-embedding-ada-002"
        }))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .expect("failed calling openai");
    let embedding_resp = handle_response::<EmbeddingResponse>(resp, "embeddings")
        .await
        .unwrap();
    let embeddings = embedding_resp
        .data
        .iter()
        .map(|d| d.embedding.clone())
        .collect();
    embeddings
}

// thanks Evan :D
pub async fn handle_response<T: for<'de> serde::Deserialize<'de>>(
    resp: reqwest::Response,
    method: &'static str,
) -> Result<T, Box<dyn std::error::Error>> {
    if !resp.status().is_success() {
        let errmsg = format!(
            "Failed to call method '{}', received response with status code:{} and body: {}",
            method,
            resp.status(),
            resp.text().await?
        );
        error!("{}", errmsg);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            errmsg,
        )));
    }
    let value = resp.json::<T>().await?;
    Ok(value)
}

fn upsert_embedding_table(
    schema: &str,
    table: &str,
    embeddings: Vec<Vec<f64>>,
) -> Result<(), spi::Error> {
    // TODO: write to pgvector column instead of jsonb
    let create = format!(
        "create table if not exists {schema}.{table}_embeddings (
            record_id bigserial,
            embeddings jsonb
        );
        "
    );

    // TODO: batch insert
    let insert = format!("insert into {schema}.{table}_embeddings (embeddings) values ($1);");

    let ran: Result<_, spi::Error> = Spi::connect(|mut c| {
        let _ = c.update(&create, None, None)?;
        Ok(())
    });
    let ran: Result<_, spi::Error> = Spi::connect(|mut c| {
        for d in embeddings {
            let jsb = vec_to_jsonb(d);
            let _ = c.update(
                &insert,
                None,
                Some(vec![(
                    PgBuiltInOids::JSONBOID.oid(),
                    Some(jsb.into_datum().expect("error")),
                )]),
            );
        }
        Ok(())
    });
    Ok(ran?)
}

fn vec_to_jsonb(data: Vec<f64>) -> pgrx::JsonB {
    pgrx::JsonB(serde_json::Value::from(data))
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    // #[pg_test]
    // fn test_hello_tembo() {
    //     assert_eq!("Hello, tembo", crate::hello_tembo());
    // }
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
