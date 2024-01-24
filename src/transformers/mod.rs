pub mod generic;
pub mod http_handler;
pub mod openai;
pub mod tembo;
pub mod types;

use crate::guc;
use generic::get_generic_svc_url;
use http_handler::openai_embedding_request;
use openai::{OPENAI_EMBEDDING_MODEL, OPENAI_EMBEDDING_URL};
use pgrx::prelude::*;
use types::{EmbeddingPayload, EmbeddingRequest};

pub fn transform(input: &str, transformer: &str, api_key: Option<String>) -> Vec<Vec<f64>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap_or_else(|e| error!("failed to initialize tokio runtime: {}", e));

    let embedding_request = match transformer {
        "text-embedding-ada-002" => {
            let openai_key = match api_key {
                Some(k) => k,
                None => match guc::get_guc(guc::VectorizeGuc::OpenAIKey) {
                    Some(k) => k,
                    None => {
                        error!("failed to get API key from GUC");
                    }
                },
            };

            let embedding_request = EmbeddingPayload {
                input: vec![input.to_string()],
                model: OPENAI_EMBEDDING_MODEL.to_string(),
            };
            EmbeddingRequest {
                url: OPENAI_EMBEDDING_URL.to_owned(),
                payload: embedding_request,
                api_key: Some(openai_key),
            }
        }
        _ => {
            let url = get_generic_svc_url().expect("failed to get embedding service url from GUC");
            let embedding_request = EmbeddingPayload {
                input: vec![input.to_string()],
                model: transformer.to_string(),
            };
            EmbeddingRequest {
                url,
                payload: embedding_request,
                api_key: None,
            }
        }
    };
    match runtime.block_on(async { openai_embedding_request(embedding_request).await }) {
        Ok(e) => e,
        Err(e) => {
            error!("error getting embeddings: {}", e);
        }
    }
}
