use crate::{query::check_input, types};
use pgrx::prelude::*;
use serde::{Deserialize, Serialize};

pub const PGMQ_QUEUE_NAME: &str = "vectorize_queue";

#[derive(Clone, Debug, Serialize, Deserialize, PostgresEnum)]
pub enum TableMethod {
    // append a new column to the existing table
    append,
    // join existing table to a new table with embeddings
    join,
}

// TODO: move this to api.rs
#[pg_extern]
fn table(
    table: &str,
    columns: Vec<String>,
    job_name: Option<String>,
    args: pgrx::Json,
    primary_key: String,
    schema: default!(String, "'public'"),
    update_col: default!(String, "'last_updated_at'"),
    transformer: default!(types::Transformer, "'openai'"),
    search_alg: default!(types::SimilarityAlg, "'pgv_cosine_similarity'"),
    table_method: default!(TableMethod, "'append'"),
) -> String {
    // initialize pgmq
    init_pgmq().expect("error initializing pgmq");
    let job_type = types::JobType::Columns;

    // write job to table
    let init_job_q = init_job_query();
    let job_name = match job_name {
        Some(a) => a,
        None => format!("{}_{}_{}", schema, table, columns.join("_")),
    };
    let arguments = serde_json::to_value(args).expect("invalid json for argument `args`");
    let api_key = arguments.get("api_key");

    // get prim key type
    let pkey_type = get_column_datatype(&schema, table, &primary_key);
    // TODO: implement a struct for these params
    let params = pgrx::JsonB(serde_json::json!({
        "schema": schema,
        "table": table,
        "columns": columns,
        "update_time_col": update_col,
        "table_method": table_method,
        "primary_key": primary_key,
        "pkey_type": pkey_type,
        "api_key": api_key
    }));

    // using SPI here because it is unlikely that this code will be run anywhere but inside the extension
    // background worker will likely be moved to an external container or service in near future
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
    let init_embed_q = init_embedding_table_query(
        &job_name,
        &schema,
        table,
        &transformer,
        &search_alg,
        &table_method,
    );

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

fn init_embedding_table_query(
    job_name: &str,
    schema: &str,
    table: &str,
    transformer: &types::Transformer,
    search_alg: &types::SimilarityAlg,
    transform_method: &TableMethod,
) -> String {
    // TODO: when adding support for other models, add the output dimension to the transformer attributes
    // so that they can be read here, not hard-coded here below
    // currently only supports the text-embedding-ada-002 embedding model - output dim 1536
    // https://platform.openai.com/docs/guides/embeddings/what-are-embeddings

    check_input(job_name).expect("invalid job name");
    let col_type = match (transformer, search_alg) {
        // TODO: when adding support for other models, add the output dimension to the transformer attributes
        // so that they can be read here, not hard-coded here below
        // currently only supports the text-embedding-ada-002 embedding model - output dim 1536
        // https://platform.openai.com/docs/guides/embeddings/what-are-embeddings
        (types::Transformer::openai, types::SimilarityAlg::pgv_cosine_similarity) => "vector(1536)",
    };
    match transform_method {
        TableMethod::append => append_embedding_column(job_name, schema, table, col_type),
        TableMethod::join => create_embedding_table(job_name, col_type),
    }
}

fn create_embedding_table(job_name: &str, col_type: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {schema}.{job_name}_embeddings (
            record_id text unique,
            embeddings {col_type},
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT (now() at time zone 'utc') not null
        );
        ",
        schema = types::VECTORIZE_SCHEMA
    )
}

fn append_embedding_column(job_name: &str, schema: &str, table: &str, col_type: &str) -> String {
    // TODO: when adding support for other models, add the output dimension to the transformer attributes
    // so that they can be read here, not hard-coded here below
    // currently only supports the text-embedding-ada-002 embedding model - output dim 1536
    // https://platform.openai.com/docs/guides/embeddings/what-are-embeddings

    check_input(job_name).expect("invalid job name");
    format!(
        "
        ALTER TABLE {schema}.{table}
        ADD COLUMN {job_name}_embeddings {col_type},
        ADD COLUMN {job_name}_updated_at TIMESTAMP WITH TIME ZONE;
        ",
    )
}

fn get_column_datatype(schema: &str, table: &str, column: &str) -> String {
    Spi::get_one_with_args(
        "
        SELECT data_type
        FROM information_schema.columns
        WHERE
            table_schema = $1
            AND table_name = $2
            AND column_name = $3    
    ",
        vec![
            (PgBuiltInOids::TEXTOID.oid(), schema.into_datum()),
            (PgBuiltInOids::TEXTOID.oid(), table.into_datum()),
            (PgBuiltInOids::TEXTOID.oid(), column.into_datum()),
        ],
    )
    .expect("error getting column datatype")
    .expect("no resultset for column datatype")
}
