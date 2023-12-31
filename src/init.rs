use crate::{query::check_input, types, types::TableMethod, types::Transformer};
use pgrx::prelude::*;
use std::collections::HashMap;

use anyhow::{Context, Result};
use lazy_static::lazy_static;

lazy_static! {
    // each model has its own job queue
    // maintain the mapping of transformer to queue name here
    pub static ref QUEUE_MAPPING: HashMap<Transformer, &'static str> = {
        let mut m = HashMap::new();
        m.insert(Transformer::text_embedding_ada_002, "v_openai");
        m.insert(Transformer::all_MiniLM_L12_v2, "v_all_MiniLM_L12_v2");
        m
    };
}

pub fn init_pgmq(transformer: &Transformer) -> Result<()> {
    let qname = QUEUE_MAPPING.get(transformer).expect("invalid transformer");
    // check if queue already created:
    let queue_exists: bool = Spi::get_one(&format!(
        "SELECT EXISTS (SELECT 1 FROM pgmq.meta WHERE queue_name = '{qname}');",
    ))?
    .context("error checking if queue exists")?;
    if queue_exists {
        return Ok(());
    } else {
        let ran: Result<_, spi::Error> = Spi::connect(|mut c| {
            let _r = c.update(&format!("SELECT pgmq.create('{qname}');"), None, None)?;
            Ok(())
        });
        if let Err(e) = ran {
            error!("error creating job queue: {}", e);
        }
    }
    Ok(())
}

pub fn init_cron(cron: &str, job_name: &str) -> Result<Option<i64>, spi::Error> {
    let cronjob = format!(
        "
        SELECT cron.schedule(
            '{job_name}',
            '{cron}',
            $$select vectorize.job_execute('{job_name}')$$
        )
        ;"
    );
    Spi::get_one(&cronjob)
}

pub fn init_job_query() -> String {
    format!(
        "
        INSERT INTO {schema}.job (name, job_type, transformer, search_alg, params)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (name) DO UPDATE SET
            job_type = EXCLUDED.job_type,
            transformer = EXCLUDED.transformer,
            search_alg = EXCLUDED.search_alg,
            params = EXCLUDED.params;
        ",
        schema = types::VECTORIZE_SCHEMA
    )
}

pub fn init_embedding_table_query(
    job_name: &str,
    schema: &str,
    table: &str,
    transformer: &types::Transformer,
    search_alg: &types::SimilarityAlg,
    transform_method: &TableMethod,
) -> Vec<String> {
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
        (
            types::Transformer::text_embedding_ada_002,
            types::SimilarityAlg::pgv_cosine_similarity,
        ) => "vector(1536)",
        (types::Transformer::all_MiniLM_L12_v2, types::SimilarityAlg::pgv_cosine_similarity) => {
            "vector(384)"
        }
    };
    match transform_method {
        TableMethod::append => {
            vec![
                append_embedding_column(job_name, schema, table, col_type),
                create_hnsw_cosine_index(job_name, schema, table),
            ]
        }
        TableMethod::join => {
            vec![create_embedding_table(job_name, col_type)]
        }
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

fn create_hnsw_cosine_index(job_name: &str, schema: &str, table: &str) -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS {job_name}_idx ON {schema}.{table} USING hnsw ({job_name}_embeddings vector_cosine_ops);
        ",
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
        DO $$
        BEGIN
           IF NOT EXISTS (
                SELECT 1
                FROM information_schema.columns
                WHERE table_name = '{table}'
                AND table_schema = '{schema}'
                AND column_name = '{job_name}_embeddings'
            )
            THEN ALTER TABLE {schema}.{table}
            ADD COLUMN {job_name}_embeddings {col_type},
            ADD COLUMN {job_name}_updated_at TIMESTAMP WITH TIME ZONE;
           END IF;
        END
        $$;
        ",
    )
}

pub fn get_column_datatype(schema: &str, table: &str, column: &str) -> String {
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
