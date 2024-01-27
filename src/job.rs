use anyhow::Result;

use crate::init::VECTORIZE_QUEUE;
use pgrx::prelude::*;

use crate::executor::{JobMessage, VectorizeMeta};
use crate::transformers::types::Inputs;
use crate::util;
use tiktoken_rs::cl100k_base;

/// called by the trigger function when a table is updated
/// handles enqueueing the embedding transform jobs
#[pg_extern]
fn handle_table_update(job_name: &str, record_ids: Vec<String>, inputs: Vec<String>) {
    // get the job metadata
    let project_meta: VectorizeMeta = if let Ok(Some(js)) = util::get_vectorize_meta_spi(job_name) {
        js
    } else {
        error!("failed to get project metadata");
    };

    // create Input objects
    let bpe = cl100k_base().unwrap();
    let mut new_inputs: Vec<Inputs> = Vec::new();
    for (record_id, input) in record_ids.into_iter().zip(inputs.into_iter()) {
        let token_estimate = bpe.encode_with_special_tokens(&input).len() as i32;
        new_inputs.push(Inputs {
            record_id,
            inputs: input,
            token_estimate,
        })
    }

    // create the job message
    let job_message = JobMessage {
        job_name: job_name.to_string(),
        job_meta: project_meta,
        inputs: new_inputs,
    };

    // send the job message to the queue
    let query = format!(
        "select pgmq.send('{VECTORIZE_QUEUE}', '{}');",
        serde_json::to_string(&job_message).unwrap()
    );
    let _ran: Result<_, spi::Error> = Spi::connect(|mut c| {
        let _r = c.update(&query, None, None)?;
        Ok(())
    });
}
