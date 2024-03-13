use anyhow::Result;

use crate::transformers::types::{
    EmbeddingPayload, EmbeddingRequest, EmbeddingResponse, Inputs, PairedEmbeddings,
};
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
        return Err(anyhow::anyhow!(errmsg));
    }
    let value = resp.json::<T>().await?;
    Ok(value)
}

// handle an OpenAI compatible embedding transform request
pub async fn openai_embedding_request(
    request: EmbeddingRequest,
    timeout: i32,
) -> Result<Vec<Vec<f64>>> {
    let client = reqwest::Client::new();
    let mut req = client
        .post(request.url)
        .timeout(std::time::Duration::from_secs(timeout as u64))
        .json::<EmbeddingPayload>(&request.payload)
        .header("Content-Type", "application/json");
    if let Some(key) = request.api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req.send().await?;
    let embedding_resp = handle_response::<EmbeddingResponse>(resp, "embeddings").await?;
    let embeddings = embedding_resp
        .data
        .iter()
        .map(|d| d.embedding.clone())
        .collect();
    Ok(embeddings)
}

// merges the vec of inputs with the embedding responses
pub fn merge_input_output(inputs: Vec<Inputs>, values: Vec<Vec<f64>>) -> Vec<PairedEmbeddings> {
    inputs
        .into_iter()
        .zip(values)
        .map(|(input, value)| PairedEmbeddings {
            primary_key: input.record_id,
            embeddings: value,
        })
        .collect()
}
