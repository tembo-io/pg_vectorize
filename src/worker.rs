use pgrx::prelude::*;
use pgrx::spi::SpiTupleTable;

use crate::errors::DatabaseError;
use crate::init::PGMQ_QUEUE_NAME;
use crate::query::check_input;
use crate::types;
use crate::util::{from_env_default, Config};
use chrono::serde::ts_seconds_option::deserialize as from_tsopt;
use chrono::TimeZone;
use serde::{Deserialize, Serialize};
use sqlx::error::Error;
use sqlx::postgres::PgRow;
use sqlx::types::chrono::Utc;
use sqlx::{FromRow, PgPool, Pool, Postgres, Row};

use pgrx::spi;
use std::env;

use std::time::Duration;

use pgrx::bgworkers::*;

use crate::executor::JobMessage;

#[pg_guard]
pub extern "C" fn _PG_init() {
    BackgroundWorkerBuilder::new("PG Tembo Background Worker")
        .set_function("background_worker_main")
        .set_library("tembo")
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
    let cfg = Config::default();
    let (conn, queue) = runtime.block_on(async {
        let conn = PgPool::connect(&cfg.pg_conn_str)
            .await
            .expect("failed sqlx connection");
        let queue = pgmq::PGMQueueExt::new(cfg.pg_conn_str, 4)
            .await
            .expect("failed to init db connection");
        (conn, queue)
    });

    log!("Starting BG Workers {}", BackgroundWorker::get_name(),);

    // poll at 10s or on a SIGTERM
    while BackgroundWorker::wait_latch(Some(Duration::from_secs(10))) {
        if BackgroundWorker::sighup_received() {
            // on SIGHUP, you might want to reload some external configuration or something
        }
        runtime.block_on(async {
            match queue.read::<JobMessage>(PGMQ_QUEUE_NAME, 300).await {
                Ok(Some(msg)) => {
                    log!("Received message: {:?}", msg);
                    let job_meta = msg.message.job_meta;
                    // let job_type = job_meta.job_type.clone;
                    match job_meta.transformer {
                        types::Transformer::openai => {
                            log!("OpenAI transformer");
                        }
                        _ => {
                            log!("No transformer found");
                        }
                    }
                }
                Ok(None) => {
                    log!("No message received");
                }
                _ => {
                    log!("Error reading message");
                }
            }
        });
    }

    log!(
        "Goodbye from inside the {} BGWorker! ",
        BackgroundWorker::get_name()
    );
}
