use crate::chat::ops::{call_chat, call_chat_completions};
use crate::chat::types::RenderedPrompt;
use crate::guc::get_guc_configs;
use crate::search::{self, init_table};
use crate::transformers::generic::env_interpolate_string;
use crate::transformers::transform;
use crate::types;
use text_splitter::TextSplitter;
use crate::init::VECTORIZE_QUEUE;
use vectorize_core::transformers::providers::get_provider;

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

/// EXPERIMENTAL: Hybrid search
///
/// This function is experimental and may change in future versions.
#[pg_extern]
fn hybrid_search(
    job_name: String,
    query: String,
    api_key: default!(Option<String>, "NULL"),
    return_columns: default!(Vec<String>, "ARRAY['*']::text[]"),
    num_results: default!(i32, 10),
    where_sql: default!(Option<String>, "NULL"),
) -> Result<TableIterator<'static, (name!(search_results, pgrx::JsonB),)>> {
    let search_results = search::hybrid_search(
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

#[pg_extern]
fn import_embeddings(
    job_name: &str,
    src_table: &str,
    src_primary_key: &str,
    src_embeddings_col: &str,
) -> Result<String> {
    // Get project metadata
    let meta = get_vectorize_meta_spi(job_name)?;
    let job_params: types::JobParams = serde_json::from_value(meta.params.clone())?;

    // Get model dimension
    let guc_configs = get_guc_configs(&meta.transformer.source);
    let provider = get_provider(
        &meta.transformer.source,
        guc_configs.api_key.clone(),
        guc_configs.service_url.clone(),
        guc_configs.virtual_key.clone(),
    )?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap_or_else(|e| error!("failed to initialize tokio runtime: {}", e));

    let expected_dim = runtime
        .block_on(async { provider.model_dim(&meta.transformer.api_name()).await })?;

    let mut count = 0;

    // Process rows based on table method
    if job_params.table_method == types::TableMethod::join {
        // For join method, insert into dedicated embeddings table
        let select_q = format!(
            "SELECT {}, {} FROM {}",
            src_primary_key, src_embeddings_col, src_table
        );
        
        let rows = Spi::get_all(&select_q)?;
        for row in rows {
            let original_id = row
                .get_by_name::<String>(src_primary_key)?
                .ok_or_else(|| anyhow::anyhow!("Missing primary key value"))?;
            
            let embeddings: Vec<f64> = row
                .get_by_name::<Vec<f64>>(src_embeddings_col)?
                .ok_or_else(|| anyhow::anyhow!("Missing embeddings value"))?;

            // Validate dimensions
            if embeddings.len() != expected_dim as usize {
                return Err(anyhow::anyhow!(
                    "Row {} has embedding length {} but expected {}",
                    original_id,
                    embeddings.len(),
                    expected_dim
                ));
            }

            let insert_q = format!(
                "INSERT INTO vectorize._embeddings_{} ({}, embeddings, updated_at)
                 VALUES ($1, $2, NOW())
                 ON CONFLICT ({}) DO UPDATE SET embeddings = $2, updated_at = NOW()",
                job_name, job_params.primary_key, job_params.primary_key
            );

            Spi::run_with_args(
                &insert_q,
                Some(vec![
                    (PgBuiltInOids::TEXTOID.oid(), original_id.into_datum()),
                    (PgBuiltInOids::FLOAT8ARRAYOID.oid(), embeddings.into_datum()),
                ]),
            )?;
            count += 1;
        }
    } else {
        // For append method, update the source table's embeddings column
        let update_q = format!(
            "UPDATE {}.{} t0 
             SET {}_embeddings = src.{},
                 {}_updated_at = NOW()
             FROM {} src
             WHERE t0.{} = src.{}",
            job_params.schema,
            job_params.table,
            job_name,
            src_embeddings_col,
            job_name,
            src_table,
            job_params.primary_key,
            src_primary_key
        );
        
        Spi::run(&update_q)?;
        count = Spi::get_one::<i64>("SELECT count(*) FROM last_query")?
            .unwrap_or(0) as i32;
    }

    // Clean up realtime jobs if necessary
    if job_params.schedule == "realtime" {
        let delete_q = format!(
            "DELETE FROM pgmq.q_{} WHERE message->>'job_name' = $1",
            VECTORIZE_QUEUE
        );
        Spi::run_with_args(
            &delete_q,
            Some(vec![(PgBuiltInOids::TEXTOID.oid(), job_name.into_datum())]),
        )?;
    }

    Ok(format!("Successfully imported embeddings for {} row(s)", count))
}

#[allow(clippy::too_many_arguments)]
#[pg_extern]
fn table_from(
    table: &str,
    columns: Vec<String>,
    job_name: &str,
    primary_key: &str,
    src_table: &str,
    src_primary_key: &str,
    src_embeddings_col: &str,
    schema: default!(&str, "'public'"),
    update_col: default!(String, "'last_updated_at'"),
    index_dist_type: default!(types::IndexDist, "'pgv_hnsw_cosine'"),
    transformer: default!(&str, "'sentence-transformers/all-MiniLM-L6-v2'"),
    table_method: default!(types::TableMethod, "'join'"),
    schedule: default!(&str, "'* * * * *'"),
) -> Result<String> {
    let model = Model::new(transformer)?;
    
    // First initialize the table structure without triggers/cron
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
        "manual", // Use manual schedule initially to prevent immediate job creation
    )?;

    // Import the embeddings
    import_embeddings(
        job_name,
        src_table,
        src_primary_key,
        src_embeddings_col,
    )?;

    // Now set up the triggers or cron job based on the desired schedule
    if schedule == "realtime" {
        // Create triggers for realtime updates
        let trigger_handler = create_trigger_handler(job_name, &columns, primary_key);
        Spi::run(&trigger_handler)?;
        
        let insert_trigger = create_event_trigger(job_name, schema, table, "INSERT");
        let update_trigger = create_event_trigger(job_name, schema, table, "UPDATE");
        
        Spi::run(&insert_trigger)?;
        Spi::run(&update_trigger)?;
    } else if schedule != "manual" {
        // Set up cron job for scheduled updates
        init_cron(schedule, job_name)?;
    }

    Ok(format!(
        "Successfully created table from existing embeddings with schedule: {}",
        schedule
    ))
}
