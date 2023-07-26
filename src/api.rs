use crate::openai::get_embeddings;
use crate::{query::check_input, types};
use pgrx::prelude::*;

#[pg_extern]
fn search(
    job_name: &str,
    schema: &str,
    table: &str,
    return_col: &str,
    query: &str,
    api_key: &str,
    num_results: default!(i32, 10),
) -> Result<Vec<pgrx::JsonB>, spi::Error> {
    // TODO: simplify api signature
    // TODO: user should not have to provide schema and table again
    // TODO: this needs to introspect the project type to figure out where to get embeddings from
    // assuming default openai API for now
    // get embeddings
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    let embeddings =
        runtime.block_on(async { get_embeddings(&vec![query.to_string()], api_key).await });
    cosine_similarity_search(
        job_name,
        schema,
        table,
        return_col,
        num_results,
        &embeddings[0],
    )
}

fn cosine_similarity_search(
    project: &str,
    schema: &str,
    table: &str,
    return_col: &str,
    num_results: i32,
    embeddings: &[f64],
) -> Result<Vec<pgrx::JsonB>, spi::Error> {
    let emb = serde_json::to_string(&embeddings).expect("failed to serialize embeddings");
    let query = format!(
        "
    SELECT 
        1 - ({project}_embeddings <=> '{emb}'::vector) AS cosine_similarity,
        *
    FROM {schema}.{table}
    ORDER BY cosine_similarity DESC
    LIMIT {num_results};
    "
    );
    log!("query: {}", query);
    Spi::connect(|client| {
        let mut results: Vec<pgrx::JsonB> = Vec::new();
        let tup_table = client.select(&query, None, None)?;

        for row in tup_table {
            let v = row[return_col]
                .value::<String>()
                .expect("failed to get value");
            let score = row["cosine_similarity"]
                .value::<f64>()
                .expect("failed to get value");

            let r = serde_json::json!({
                "column": return_col,
                "value": v,
                "similarity_score": score
            });
            results.push(pgrx::JsonB(r));
        }

        Ok(results)
    })
}
