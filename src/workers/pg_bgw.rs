use crate::guc::init_guc;
use crate::init::QUEUE_MAPPING;
use crate::types::Transformer;
use crate::util::get_pg_conn;
use anyhow::Result;
use pgrx::bgworkers::*;
use pgrx::*;
use std::time::Duration;

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

    // bgw only supports the OpenAI transformer case
    let oai_q = QUEUE_MAPPING
        .get(&Transformer::openai)
        .expect("invalid transformer");
    let aux_q = QUEUE_MAPPING
        .get(&Transformer::all_MiniLM_L12_v2)
        .expect("invalid transformer");
    while BackgroundWorker::wait_latch(Some(Duration::from_secs(5))) {
        if BackgroundWorker::sighup_received() {
            // on SIGHUP, you might want to reload configurations and env vars
        }
        let _: Result<()> =
            runtime.block_on(async { run_worker(queue.clone(), conn.clone(), oai_q).await });
        let _: Result<()> =
            runtime.block_on(async { run_worker(queue.clone(), conn.clone(), aux_q).await });
    }
    log!("pg-vectorize: shutting down");
}
