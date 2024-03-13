use log::{error, info};
use vectorize::workers::base::{work, Config};

#[tokio::main]
async fn main() {
    info!("starting pg-vectorize remote-worker");

    let conn = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:password@localhost:28815/postgres")
        .await
        .unwrap();
    let queue = pgmq::PGMQueueExt::new_with_pool(conn.clone())
        .await
        .unwrap();

    let cfg = Config::from_env();

    loop {
        match work(&conn, &queue, &cfg).await {
            Ok(Some(_)) => {
                // continue processing
            }
            Ok(None) => {
                // no messages, small wait
                info!(
                    "No messages in queue, waiting for {} seconds",
                    cfg.poll_interval
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(cfg.poll_interval)).await;
            }
            Err(e) => {
                // error, long wait
                error!("Error processing job: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(cfg.poll_interval)).await;
            }
        }
    }
}
