use super::generic::get_env_interpolated_guc;
use crate::guc;
use anyhow::Result;

use pgrx::prelude::*;

use vectorize_core::transformers::providers::vector_serve::VectorServeProvider;
use vectorize_core::transformers::providers::EmbeddingProvider;
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
    let svc_url = get_env_interpolated_guc(guc::VectorizeGuc::EmbeddingServiceUrl)?;
    let provider = VectorServeProvider::new(Some(svc_url.clone()), api_key);
    let dim = provider.model_dim(model_name).await?;
    Ok(TransformerMetadata {
        model: model_name.to_string(),
        max_seq_len: 0,
        embedding_dimension: dim as i32,
    })
}
