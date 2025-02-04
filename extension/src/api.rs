use crate::chat::ops::{call_chat, call_chat_completions};
use crate::chat::types::RenderedPrompt;
use crate::guc::get_guc_configs;
use crate::search::{self, init_table};
use crate::transformers::generic::env_interpolate_string;
use crate::transformers::transform;
use crate::types;
use text_splitter::TextSplitter;

use anyhow::Result;
use pgrx::prelude::*;
use vectorize_core::types::Model;

#[pg_extern]
fn chunk_table(
    input_table: &str,
    column_name: &str,
    primary_key: &str, // Add primary_key parameter
    max_chunk_size: default!(i32, 1000),
    output_table: default!(&str, "'chunked_data'"),
) -> Result<String> {
    let max_chunk_size = max_chunk_size as usize;

    // Retrieve rows from the input table, ensuring column existence
    let query = format!(
        "SELECT {}, {} FROM {}",
        primary_key, column_name, input_table
    ); // Use primary_key instead of hardcoding "id"

    // Reverting back to use get_two
    let (id_opt, text_opt): (Option<i32>, Option<String>) = Spi::get_two(&query)?;
    let rows = vec![(id_opt, text_opt)]; // Wrap in a vector if needed

    // Prepare to hold chunked rows
    let mut chunked_rows: Vec<(i32, i32, String)> = Vec::new(); // (original_id, chunk_index, chunk)

    // Chunk the data and keep track of the original id and chunk index
    for (id_opt, text_opt) in rows {
        // Only process rows where both id and text exist
        if let (Some(id), Some(text)) = (id_opt, text_opt.map(|s| s.to_string())) {
            let chunks = chunk_text(
                &text,
                max_chunk_size.try_into().expect("failed usize conversion"),
            );
            for (index, chunk) in chunks.iter().enumerate() {
                chunked_rows.push((id, index as i32, chunk.clone())); // Add chunk index
            }
        }
    }

    // Create output table with an additional column for chunk index
    let create_table_query = format!(
        "CREATE TABLE IF NOT EXISTS {} (id SERIAL PRIMARY KEY, original_id INT, chunk_index INT, chunk TEXT)",
        output_table
    );
    Spi::run(&create_table_query)
        .map_err(|e| anyhow::anyhow!("Failed to create table {}: {}", output_table, e))?;

    // Insert chunked rows into output table
    for (original_id, chunk_index, chunk) in chunked_rows {
        let insert_query = format!(
            "INSERT INTO {} (original_id, chunk_index, chunk) VALUES ($1, $2, $3)",
            output_table
        );
        Spi::run_with_args(
            &insert_query,
            Some(vec![
                (
                    pgrx::PgOid::Custom(pgrx::pg_sys::INT4OID),
                    original_id.into_datum(),
                ), // OID for integer
                (
                    pgrx::PgOid::Custom(pgrx::pg_sys::INT4OID),
                    chunk_index.into_datum(),
                ), // OID for integer
                (
                    pgrx::PgOid::Custom(pgrx::pg_sys::TEXTOID),
                    chunk.into_datum(),
                ), // OID for text
            ]),
        )?;
    }

    Ok(format!(
        "Chunked data inserted into table: {}",
        output_table
    ))
}

#[allow(clippy::too_many_arguments)]
#[pg_extern]
fn table(
    table: &str,
    columns: Vec<String>,
    job_name: &str,
    primary_key: &str,
    schema: default!(&str, "'public'"),
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
        schema,
        table,
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
        schema,
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

/// Splits a document into smaller chunks of text based on a maximum characters
///
/// # Example
///
/// ```sql
/// -- Example usage in PostgreSQL after creating the function:
/// SELECT vectorize.chunk_text('This is a sample text to demonstrate chunking.', 20);
///
/// -- Expected output:
/// -- ["This is a sample tex", "t to demonstrate ch", "unking."]
/// ```
#[pg_extern]
fn chunk_text(document: &str, max_characters: i64) -> Vec<String> {
    let max_chars_usize = max_characters as usize;
    let splitter = TextSplitter::new(max_chars_usize);
    splitter.chunks(document).map(|s| s.to_string()).collect()
}
