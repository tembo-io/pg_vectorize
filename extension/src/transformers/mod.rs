pub mod generic;
pub mod http_handler;
pub mod openai;

use crate::guc::{self, EMBEDDING_REQ_TIMEOUT_SEC};
use generic::get_generic_svc_url;
use pgrx::prelude::*;

use vectorize_core::transformers::http_handler::openai_embedding_request;
use vectorize_core::transformers::openai::OPENAI_EMBEDDING_URL;
use vectorize_core::transformers::types::{EmbeddingPayload, EmbeddingRequest};
use vectorize_core::types::{Model, ModelSource};

pub fn transform(input: &str, transformer: &Model, api_key: Option<String>) -> Vec<Vec<f64>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap_or_else(|e| error!("failed to initialize tokio runtime: {}", e));

    let embedding_request = match transformer.source {
        ModelSource::OpenAI => {
            let openai_key = match api_key {
                Some(k) => k.to_owned(),
                None => match guc::get_guc(guc::VectorizeGuc::OpenAIKey) {
                    Some(k) => k,
                    None => {
                        error!("failed to get API key from GUC");
                    }
                },
            };

            let embedding_request = EmbeddingPayload {
                input: vec![input.to_string()],
                model: transformer.name.to_string(),
            };
            EmbeddingRequest {
                url: OPENAI_EMBEDDING_URL.to_owned(),
                payload: embedding_request,
                api_key: Some(openai_key.to_string()),
            }
        }
        ModelSource::SentenceTransformers => {
            let url = get_generic_svc_url().expect("failed to get embedding service url from GUC");
            let embedding_request = EmbeddingPayload {
                input: vec![input.to_string()],
                model: transformer.fullname.to_string(),
            };
            EmbeddingRequest {
                url,
                payload: embedding_request,
                api_key: api_key.map(|s| s.to_string()),
            }
        }
        ModelSource::Ollama => error!("Ollama transformer not implemented yet"),
    };
    let timeout = EMBEDDING_REQ_TIMEOUT_SEC.get();

    match transformer.source {
        ModelSource::Ollama => error!("Ollama transformer not implemented yet"),
        ModelSource::OpenAI | ModelSource::SentenceTransformers => {
            match runtime
                .block_on(async { openai_embedding_request(embedding_request, timeout).await })
            {
                Ok(e) => e,
                Err(e) => {
                    error!("error getting embeddings: {}", e);
                }
            }
        }
    }
}
