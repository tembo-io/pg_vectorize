use log::error;
use vectorize::workers::base::{work, Config};

#[tokio::main]
async fn main() {
    println!("Starting worker");

    let conn = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:password@localhost:28815/postgres")
        .await
        .unwrap();
    let queue = pgmq::PGMQueueExt::new_with_pool(conn.clone())
        .await
        .unwrap();

    let cfg = Config {
        queue_name: "vectorize_jobs".to_string(),
        embedding_svc_url: "http://0.0.0.0:3000/v1/embeddings".to_string(),
        openai_api_key: Some("".to_owned()),
        embedding_request_timeout: 6,
    };

    loop {
        match work(&conn, &queue, &cfg).await {
            Ok(Some(_)) => {
                // continue processing
            }
            Ok(None) => {
                // no messages, small wait
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
            Err(e) => {
                // error, long wait
                error!("Error processing job: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            }
        }
    }
}
