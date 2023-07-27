use pgrx::prelude::*;

pub fn cosine_similarity_search(
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
