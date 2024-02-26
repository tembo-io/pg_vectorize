use super::generic::get_generic_svc_url;
use super::types::TransformerMetadata;
use anyhow::Result;

use crate::guc::EMBEDDING_REQ_TIMEOUT_SEC;
use crate::transformers::types::{
    EmbeddingPayload, EmbeddingRequest, EmbeddingResponse, Inputs, PairedEmbeddings,
};
use pgrx::prelude::*;

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
        warning!("pg-vectorize: error handling response: {}", errmsg);
        return Err(anyhow::anyhow!(errmsg));
    }
    let value = resp.json::<T>().await?;
    Ok(value)
}

// handle an OpenAI compatible embedding transform request
pub async fn openai_embedding_request(request: EmbeddingRequest) -> Result<Vec<Vec<f64>>> {
    log!(
        "pg-vectorize: embedding request size: {}",
        request.payload.input.len()
    );
    let client = reqwest::Client::new();
    let timeout = EMBEDDING_REQ_TIMEOUT_SEC.get();
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

#[pg_extern]
pub fn mod_info(model_name: &str, api_key: default!(Option<String>, "NULL")) -> pgrx::JsonB {
    let meta = sync_get_model_info(model_name, api_key).unwrap();
    pgrx::JsonB(serde_json::to_value(meta).unwrap())
}

pub fn sync_get_model_info(
    model_name: &str,
    api_key: Option<String>,
) -> Result<TransformerMetadata> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap_or_else(|e| error!("failed to initialize tokio runtime: {}", e));
    let meta = match runtime.block_on(async { get_model_info(model_name, api_key).await }) {
        Ok(e) => e,
        Err(e) => {
            error!("error getting embeddings: {}", e);
        }
    };
    Ok(meta)
}

pub async fn get_model_info(
    model_name: &str,
    api_key: Option<String>,
) -> Result<TransformerMetadata> {
    let svc_url = get_generic_svc_url()?;
    let info_url = svc_url.replace("/embeddings", "/info");
    let client = reqwest::Client::new();
    let mut req = client
        .get(info_url)
        .query(&[("model_name", model_name)])
        .timeout(std::time::Duration::from_secs(5)); // model info must always be fast
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req.send().await?;
    let meta_response = handle_response::<TransformerMetadata>(resp, "info").await?;
    Ok(meta_response)
}
