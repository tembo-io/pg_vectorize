use pgrx::prelude::*;
use serde_json::json;

use anyhow::Result;

#[derive(serde::Deserialize, Debug)]
struct EmbeddingResponse {
    // object: String,
    data: Vec<DataObject>,
}

#[derive(serde::Deserialize, Debug)]
struct DataObject {
    // object: String,
    // index: usize,
    embedding: Vec<f64>,
}

pub async fn get_embeddings(inputs: &Vec<String>, key: &str) -> Result<Vec<Vec<f64>>> {
    // let len = inputs.len();
    // vec![vec![0.0; 1536]; len]
    let url = "https://api.openai.com/v1/embeddings";
    log!("pg-vectorize: openai request size: {}", inputs.len());
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .json(&json!({
            "input": inputs,
            "model": "text-embedding-ada-002"
        }))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .expect("failed calling openai");
    let embedding_resp = handle_response::<EmbeddingResponse>(resp, "embeddings").await?;

    let embeddings = embedding_resp
        .data
        .iter()
        .map(|d| d.embedding.clone())
        .collect();
    Ok(embeddings)
}

// thanks Evan :D
pub async fn handle_response<T: for<'de> serde::Deserialize<'de>>(
    resp: reqwest::Response,
    method: &'static str,
) -> Result<T> {
    if !resp.status().is_success() {
        let errmsg = format!(
            "Failed to call method '{}', received response with status code:{} and body: {}",
            method,
            resp.status(),
            resp.text().await?
        );
        error!("{}", errmsg);
    }
    let value = resp.json::<T>().await?;
    Ok(value)
}
