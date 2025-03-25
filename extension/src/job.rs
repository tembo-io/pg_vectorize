use anyhow::Result;

use crate::executor::{create_batches, new_rows_query, new_rows_query_join};
use crate::guc::BATCH_SIZE;
use crate::init::VECTORIZE_QUEUE;
use pgrx::prelude::*;
use tiktoken_rs::cl100k_base;
use vectorize_core::transformers::types::Inputs;
use vectorize_core::types::{IndexDist, JobMessage, JobParams, Model, TableMethod, VectorizeMeta};

static TRIGGER_FN_PREFIX: &str = "vectorize.handle_update_";

/// creates a function that can be called by trigger
pub fn create_trigger_handler(job_name: &str, pkey: &str) -> String {
    format!(
        "
CREATE OR REPLACE FUNCTION {TRIGGER_FN_PREFIX}{job_name}()
RETURNS TRIGGER AS $$
DECLARE
BEGIN
    PERFORM vectorize._handle_table_update(
        '{job_name}'::text,
       (SELECT array_agg({pkey}::text) FROM new_table)::TEXT[]
    );
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;    
"
    )
}

// creates the trigger for a row update
// these triggers use transition tables
// transition tables cannot be specified for triggers with more than one event
// so we create two triggers instead
pub fn create_event_trigger(job_name: &str, schema: &str, table_name: &str, event: &str) -> String {
    format!(
        "
CREATE OR REPLACE TRIGGER vectorize_{event_name}_trigger_{job_name}
AFTER {event} ON {schema}.{table_name}
REFERENCING NEW TABLE AS new_table
FOR EACH STATEMENT
EXECUTE FUNCTION vectorize.handle_update_{job_name}();",
        event_name = event.to_lowercase()
    )
}

// creates batches of embedding jobs
// typically used on table init
pub fn initalize_table_job(
    job_name: &str,
    job_params: &JobParams,
    index_dist_type: IndexDist,
    transformer: &Model,
) -> Result<()> {
    // start with initial batch load
    let rows_need_update_query: String = match job_params.table_method {
        TableMethod::append => new_rows_query(job_name, job_params),
        TableMethod::join => new_rows_query_join(job_name, job_params),
    };
    let mut inputs: Vec<Inputs> = Vec::new();
    let bpe = cl100k_base().unwrap();
    let _: Result<_, spi::Error> = Spi::connect(|c| {
        let rows = c.select(&rows_need_update_query, None, &[])?;
        for row in rows {
            let ipt = row["input_text"]
                .value::<String>()?
                .expect("input_text is null");
            let token_estimate = bpe.encode_with_special_tokens(&ipt).len() as i32;
            inputs.push(Inputs {
                record_id: row["record_id"]
                    .value::<String>()?
                    .expect("record_id is null"),
                inputs: ipt.trim().to_owned(),
                token_estimate,
            });
        }
        Ok(())
    });

    let max_batch_size = BATCH_SIZE.get();
    let batches = create_batches(inputs, max_batch_size);
    let vectorize_meta = VectorizeMeta {
        name: job_name.to_string(),
        // TODO: in future, lookup job id once this gets put into use
        // job_id is currently not used, job_name is unique
        job_id: 0,
        params: serde_json::to_value(job_params.clone()).unwrap(),
        index_dist_type: index_dist_type.clone(),
        transformer: transformer.clone(),
    };
    for b in batches {
        let job_message = JobMessage {
            job_name: job_name.to_string(),
            job_meta: vectorize_meta.clone(),
            record_ids: b.iter().map(|i| i.record_id.clone()).collect(),
        };
        let query = "select pgmq.send($1, $2::jsonb);";
        let _ran: Result<_, spi::Error> = Spi::connect_mut(|c| {
            let _r = c.update(
                query,
                None,
                &[
                    VECTORIZE_QUEUE.into(),
                    pgrx::JsonB(
                        serde_json::to_value(job_message).expect("failed parsing job message"),
                    )
                    .into(),
                ],
            )?;
            Ok(())
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_update_trigger_single() {
        let job_name = "another_job";
        let table_name = "another_table";

        let expected = format!(
            "
CREATE OR REPLACE TRIGGER vectorize_update_trigger_another_job
AFTER UPDATE ON myschema.another_table
REFERENCING NEW TABLE AS new_table
FOR EACH STATEMENT
EXECUTE FUNCTION vectorize.handle_update_another_job();"
        );
        let result = create_event_trigger(job_name, "myschema", table_name, "UPDATE");
        assert_eq!(expected, result);
    }

    #[test]
    fn test_create_insert_trigger_single() {
        let job_name = "another_job";
        let table_name = "another_table";

        let expected = format!(
            "
CREATE OR REPLACE TRIGGER vectorize_insert_trigger_another_job
AFTER INSERT ON myschema.another_table
REFERENCING NEW TABLE AS new_table
FOR EACH STATEMENT
EXECUTE FUNCTION vectorize.handle_update_another_job();"
        );
        let result = create_event_trigger(job_name, "myschema", table_name, "INSERT");
        assert_eq!(expected, result);
    }
}
