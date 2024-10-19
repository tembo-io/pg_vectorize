use crate::chat::ops::{call_chat, call_chat_completions};
use crate::chat::types::RenderedPrompt;
use crate::guc::get_guc_configs;
use crate::search::{self, init_table};
use crate::transformers::generic::env_interpolate_string;
use crate::transformers::transform;
use crate::types;

use anyhow::Result;
use pgrx::prelude::*;
use vectorize_core::types::Model;

#[allow(clippy::too_many_arguments)]
#[pg_extern]
fn table(
    table_name: PgOid,
    columns: Vec<String>,
    job_name: &str,
    primary_key: &str,
    update_col: default!(String, "'last_updated_at'"),
    index_dist_type: default!(types::IndexDist, "'pgv_hnsw_cosine'"),
    transformer: default!(&str, "'sentence-transformers/all-MiniLM-L6-v2'"),
    table_method: default!(types::TableMethod, "'join'"),
    // cron-like for a cron based update model, or 'realtime' for a trigger-based
    schedule: default!(&str, "'* * * * *'"),
) -> Result<String> {
    let model = Model::new(transformer)?;

    init_table(
        job_name,
        table_name,
        columns,
        primary_key,
        Some(update_col),
        index_dist_type.into(),
        &model,
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
    model_name: default!(String, "'sentence-transformers/all-MiniLM-L6-v2'"),
    api_key: default!(Option<String>, "NULL"),
) -> Result<Vec<f64>> {
    let model = Model::new(&model_name)?;
    Ok(transform(input, &model, api_key).remove(0))
}

#[pg_extern]
fn encode(
    input: &str,
    model: default!(String, "'sentence-transformers/all-MiniLM-L6-v2'"),
    api_key: default!(Option<String>, "NULL"),
) -> Result<Vec<f64>> {
    let model = Model::new(&model)?;
    Ok(transform(input, &model, api_key).remove(0))
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
    index_dist_type: default!(types::IndexDist, "'pgv_hnsw_cosine'"),
    // transformer model to use in vector-search
    transformer: default!(&str, "'sentence-transformers/all-MiniLM-L6-v2'"),
    table_method: default!(types::TableMethod, "'join'"),
    schedule: default!(&str, "'* * * * *'"),
) -> Result<String> {
    // chat only supports single columns transform
    let columns = vec![column.to_string()];
    let transformer_model = Model::new(transformer)?;
    init_table(
        agent_name,
        table_name,
        columns,
        unique_record_id,
        None,
        index_dist_type.into(),
        &transformer_model,
        table_method.into(),
        schedule,
    )
}

/// creates a table indexed with embeddings for chat completion workloads
#[pg_extern]
fn rag(
    agent_name: &str,
    query: &str,
    chat_model: default!(String, "'tembo/meta-llama/Meta-Llama-3-8B-Instruct'"),
    // points to the type of prompt template to use
    task: default!(String, "'question_answer'"),
    api_key: default!(Option<String>, "NULL"),
    // number of records to include in the context
    num_context: default!(i32, 2),
    // truncates context to fit the model's context window
    force_trim: default!(bool, false),
) -> Result<TableIterator<'static, (name!(chat_results, pgrx::JsonB),)>> {
    let model = Model::new(&chat_model)?;
    let resp = call_chat(
        agent_name,
        query,
        &model,
        &task,
        api_key,
        num_context,
        force_trim,
    )?;
    let iter = vec![(pgrx::JsonB(serde_json::to_value(resp)?),)];
    Ok(TableIterator::new(iter))
}

#[pg_extern]
fn generate(
    input: &str,
    model: default!(String, "'tembo/meta-llama/Meta-Llama-3-8B-Instruct'"),
    api_key: default!(Option<String>, "NULL"),
) -> Result<String> {
    let model = Model::new(&model)?;
    let prompt = RenderedPrompt {
        sys_rendered: "".to_string(),
        user_rendered: input.to_string(),
    };
    let mut guc_configs = get_guc_configs(&model.source);
    if let Some(api_key) = api_key {
        guc_configs.api_key = Some(api_key);
    }
    call_chat_completions(prompt, &model, &guc_configs)
}

#[pg_extern]
fn env_interpolate_guc(guc_name: &str) -> Result<String> {
    let g: String = Spi::get_one_with_args(
        "SELECT current_setting($1)",
        vec![(PgBuiltInOids::TEXTOID.oid(), guc_name.into_datum())],
    )?
    .unwrap_or_else(|| panic!("no value set for guc: {guc_name}"));
    env_interpolate_string(&g)
}
