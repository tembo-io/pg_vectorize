use pgrx::prelude::*;

use crate::{
    query::check_input,
    types::{self, Transformer},
};

pub const PGMQ_QUEUE_NAME: &str = "vectorize_queue";

#[pg_extern]
fn init_table(
    schema: &str,
    table: &str,
    join_key: &str,
    columns: Vec<String>,
    transformer: types::Transformer,
    search_alg: types::SimilarityAlg,
    alias: Option<String>,
    api_key: &str,
    update_col: default!(String, "'updated_at'"),
) -> String {
    // initialize pgmq
    init_pgmq().expect("error initializing pgmq");
    let job_type = types::JobType::Columns;
    // write job to table
    let init_job_q = init_job_query();

    let job_name = match alias {
        Some(a) => a,
        None => format!("{}_{}_{}", schema, table, columns.join("_")),
    };

    // TODO: implement a struct for these params
    let params = pgrx::JsonB(serde_json::json!({
        "schema": schema,
        "table": table,
        "columns": columns,
        "update_time_col": update_col,
        "join_key": join_key,
        "api_key": api_key
    }));

    let ran: Result<_, spi::Error> = Spi::connect(|mut c| {
        let _ = c
            .update(
                &init_job_q,
                None,
                Some(vec![
                    (PgBuiltInOids::TEXTOID.oid(), job_name.clone().into_datum()),
                    (
                        PgBuiltInOids::TEXTOID.oid(),
                        job_type.to_string().into_datum(),
                    ),
                    (
                        PgBuiltInOids::TEXTOID.oid(),
                        transformer.to_string().into_datum(),
                    ),
                    (
                        PgBuiltInOids::TEXTOID.oid(),
                        search_alg.to_string().into_datum(),
                    ),
                    (PgBuiltInOids::JSONBOID.oid(), params.into_datum()),
                ]),
            )
            .expect("error exec query");
        Ok(())
    });
    ran.expect("error creating job");
    let init_embed_q = init_embedding_table(&job_name);

    let ran: Result<_, spi::Error> = Spi::connect(|mut c| {
        let _ = c.update(&init_embed_q, None, None);
        Ok(())
    });
    ran.expect("error creating embedding table");
    // do first batch update
    // setup recurring cron job
    format!("{schema}.{table}.{columns:?}.{transformer}.{search_alg}")
}

fn init_pgmq() -> Result<(), spi::Error> {
    Spi::connect(|mut c| {
        let _ = c.update(
            &format!("SELECT pgmq_create('{PGMQ_QUEUE_NAME}');"),
            None,
            None,
        );
        Ok(())
    })
}

fn init_job_query() -> String {
    format!(
        "
        INSERT INTO {schema}.vectorize_meta (name, job_type, transformer, search_alg, params)
        VALUES ($1, $2, $3, $4, $5);
        ",
        schema = types::VECTORIZE_SCHEMA
    )
}

fn init_embedding_table(job_name: &str) -> String {
    check_input(job_name).expect("invalid job name");
    // TODO: change from jsonb to specified vector type
    format!(
        "CREATE TABLE IF NOT EXISTS {schema}.{job_name}_embeddings (
            record_id text,
            embeddings jsonb,
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT (now() at time zone 'utc') not null
        );
        ",
        schema = types::VECTORIZE_SCHEMA
    )
}
