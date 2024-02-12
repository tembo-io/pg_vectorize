use crate::chat::call_chat;
use crate::executor::VectorizeMeta;
use crate::search::{cosine_similarity_search, init_table};
use crate::transformers::transform;
use crate::types;
use crate::util;

use anyhow::Result;
use pgrx::prelude::*;

#[allow(clippy::too_many_arguments)]
#[pg_extern]
fn table(
    table: &str,
    columns: Vec<String>,
    job_name: &str,
    primary_key: &str,
    args: default!(pgrx::Json, "'{}'"),
    schema: default!(&str, "'public'"),
    update_col: default!(String, "'last_updated_at'"),
    transformer: default!(&str, "'text-embedding-ada-002'"),
    search_alg: default!(types::SimilarityAlg, "'pgv_cosine_similarity'"),
    table_method: default!(types::TableMethod, "'append'"),
    // cron-like for a cron based update model, or 'realtime' for a trigger-based
    schedule: default!(&str, "'realtime'"),
) -> Result<String> {
    init_table(
        job_name,
        schema,
        table,
        columns,
        primary_key,
        Some(serde_json::to_value(args).expect("failed to parse args")),
        Some(update_col),
        transformer,
        search_alg,
        table_method,
        schedule,
    )
}

#[pg_extern]
fn search(
    job_name: &str,
    query: &str,
    api_key: default!(Option<String>, "NULL"),
    return_columns: default!(Vec<String>, "ARRAY['*']::text[]"),
    num_results: default!(i32, 10),
) -> Result<TableIterator<'static, (name!(search_results, pgrx::JsonB),)>> {
    let project_meta: VectorizeMeta = if let Ok(Some(js)) = util::get_vectorize_meta_spi(job_name) {
        js
    } else {
        error!("failed to get project metadata");
    };
    let proj_params: types::JobParams = serde_json::from_value(
        serde_json::to_value(project_meta.params).unwrap_or_else(|e| {
            error!("failed to serialize metadata: {}", e);
        }),
    )
    .unwrap_or_else(|e| error!("failed to deserialize metadata: {}", e));

    let schema = proj_params.schema;
    let table = proj_params.table;

    let embeddings = transform(query, &project_meta.transformer, api_key);

    let search_results = match project_meta.search_alg {
        types::SimilarityAlg::pgv_cosine_similarity => cosine_similarity_search(
            job_name,
            &schema,
            &table,
            &return_columns,
            num_results,
            &embeddings[0],
        )?,
    };

    Ok(TableIterator::new(search_results))
}

#[pg_extern]
fn transform_embeddings(
    input: &str,
    model_name: default!(String, "'text-embedding-ada-002'"),
    api_key: default!(Option<String>, "NULL"),
) -> Result<Vec<f64>> {
    Ok(transform(input, &model_name, api_key).remove(0))
}

#[allow(clippy::too_many_arguments)]
#[pg_extern]
fn chat_table(
    agent_name: &str,
    table_name: &str,
    unique_record_id: &str,
    // column that have data we want to be able to chat with
    column: &str,
    schema: default!(&str, "'public'"),
    // transformer model to use in vector-search
    transformer: default!(&str, "'text-embedding-ada-002'"),
    // similarity algorithm to use in vector-search
    search_alg: default!(types::SimilarityAlg, "'pgv_cosine_similarity'"),
    table_method: default!(types::TableMethod, "'append'"),
) -> Result<String> {
    // chat only supports single columns transform
    let columns = vec![column.to_string()];
    init_table(
        agent_name,
        schema,
        table_name,
        columns,
        unique_record_id,
        None,
        None,
        transformer,
        search_alg,
        table_method,
        "realtime",
    )
}

/// creates an table indexed with embeddings for chat completion workloads
#[pg_extern]
fn chat(
    agent_name: &str,
    query: &str,
    chat_model: default!(&str, "'gpt-3.5-turbo'"),
    task: default!(&str, "'question_answer'"),
    api_key: default!(Option<&str>, "NULL"),
) -> Result<TableIterator<'static, (name!(chat_results, pgrx::JsonB),)>> {
    let resp = call_chat(agent_name, query, chat_model, task, api_key)?;
    let iter = vec![(pgrx::JsonB(serde_json::to_value(resp)?),)];
    Ok(TableIterator::new(iter))
}
