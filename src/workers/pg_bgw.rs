use crate::guc::init_guc;
use crate::init::VECTORIZE_QUEUE;
use crate::util::get_pg_conn;
use anyhow::Result;
use pgrx::bgworkers::*;
use pgrx::*;
use std::time::{Duration, Instant};

use crate::workers::run_worker;

#[pg_guard]
pub extern "C" fn _PG_init() {
    init_guc();
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
    let (conn, queue) = runtime.block_on(async {
        let con = get_pg_conn().await.expect("failed to connect to database");
        let queue = pgmq::PGMQueueExt::new_with_pool(con.clone())
            .await
            .expect("failed to init db connection");
        (con, queue)
    });

    log!("Starting BG Workers {}", BackgroundWorker::get_name(),);

    while BackgroundWorker::wait_latch(Some(Duration::from_millis(10))) {
        if BackgroundWorker::sighup_received() {
            // on SIGHUP, you might want to reload configurations and env vars
        }
        let _worker_ran: Result<()> = runtime.block_on(async {
            // continue to poll without pauses
            let start = Instant::now();
            let duration = Duration::from_secs(6);
            // return control to wait_latch() after `duration` has elapsed
            while start.elapsed() < duration {
                match run_worker(queue.clone(), &conn, VECTORIZE_QUEUE).await {
                    // sleep for 2 seconds when no messages in the queue
                    Ok(None) => tokio::time::sleep(Duration::from_secs(2)).await,
                    // sleep for 6 seconds when there is an error reading messages
                    Err(_) => tokio::time::sleep(Duration::from_secs(6)).await,
                    // continue to poll where there was a message consumed
                    Ok(Some(_)) => continue,
                }
            }
            Ok(())
        });
    }
    log!("pg-vectorize: shutting down");
}
