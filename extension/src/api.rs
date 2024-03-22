use crate::chat::ops::call_chat;
use crate::search::{self, init_table};
use crate::transformers::transform;

use crate::types;
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
    index_type: default!(&str, "'hnsw'"),
    distance_function: default!(&str, "'cosine'"),
    transformer: default!(&str, "'text-embedding-ada-002'"),
    search_alg: default!(types::SimilarityAlg, "'pgv_cosine_similarity'"),
    table_method: default!(types::TableMethod, "'join'"),
    // cron-like for a cron based update model, or 'realtime' for a trigger-based
    schedule: default!(&str, "'* * * * *'"),
) -> Result<String> {
    init_table(
        job_name,
        schema,
        table,
        columns,
        primary_key,
        Some(serde_json::to_value(args).expect("failed to parse args")),
        Some(update_col),
        index_type,
        distance_function,
        transformer,
        search_alg.into(),
        table_method.into(),
        schedule,
    )
}

#[pg_extern]
fn search(
    job_name: String,
    query: String,
    api_key: default!(Option<String>, "NULL"),
    return_columns: default!(Vec<String>, "ARRAY['*']::text[]"),
    num_results: default!(i32, 10),
    where_sql: default!(Option<String>, "NULL"),
) -> Result<TableIterator<'static, (name!(search_results, pgrx::JsonB),)>> {
    let search_results = search::search(
        &job_name,
        &query,
        api_key,
        return_columns,
        num_results,
        where_sql,
    )?;
    Ok(TableIterator::new(search_results.into_iter().map(|r| (r,))))
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
fn init_rag(
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
    table_method: default!(types::TableMethod, "'join'"),
    schedule: default!(&str, "'* * * * *'"),
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
        search_alg.into(),
        table_method.into(),
        schedule,
    )
}

/// creates an table indexed with embeddings for chat completion workloads
#[pg_extern]
fn rag(
    agent_name: &str,
    query: &str,
    // chat models: currently only supports gpt 3.5 and 4
    // https://platform.openai.com/docs/models/gpt-3-5-turbo
    // https://platform.openai.com/docs/models/gpt-4-and-gpt-4-turbo
    chat_model: default!(String, "'gpt-3.5-turbo'"),
    // points to the type of prompt template to use
    task: default!(String, "'question_answer'"),
    api_key: default!(Option<String>, "NULL"),
    // number of records to include in the context
    num_context: default!(i32, 2),
    // truncates context to fit the model's context window
    force_trim: default!(bool, false),
) -> Result<TableIterator<'static, (name!(chat_results, pgrx::JsonB),)>> {
    let resp = call_chat(
        agent_name,
        query,
        &chat_model,
        &task,
        api_key,
        num_context,
        force_trim,
    )?;
    let iter = vec![(pgrx::JsonB(serde_json::to_value(resp)?),)];
    Ok(TableIterator::new(iter))
}
