use super::generic::get_generic_svc_url;
use crate::guc::EMBEDDING_REQ_TIMEOUT_SEC;
use anyhow::Result;

use pgrx::prelude::*;

use vectorize_core::transformers::http_handler::handle_response;
use vectorize_core::transformers::types::TransformerMetadata;

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
            error!("error getting model info: {}", e);
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
    let timeout = EMBEDDING_REQ_TIMEOUT_SEC.get();
    let client = reqwest::Client::new();
    let mut req = client
        .get(info_url)
        .query(&[("model_name", model_name)])
        .timeout(std::time::Duration::from_secs(timeout as u64));
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req.send().await?;
    let meta_response = handle_response::<TransformerMetadata>(resp, "info").await?;
    Ok(meta_response)
}
