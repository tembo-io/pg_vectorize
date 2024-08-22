pub mod cohere;
pub mod openai;
pub mod vector_serve;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{openai::trim_inputs, types::Inputs};
use crate::errors::VectorizeError;
use crate::transformers::providers;
use crate::types::Model;
use crate::types::ModelSource;

#[async_trait]
pub trait EmbeddingProvider {
    #[allow(async_fn_in_trait)]
    async fn generate_embedding<'a>(
        &self,
        request: &'a GenericEmbeddingRequest,
    ) -> Result<GenericEmbeddingResponse, VectorizeError>;
    #[allow(async_fn_in_trait)]
    async fn model_dim(&self, model_name: &str) -> Result<u32, VectorizeError>;
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct GenericEmbeddingRequest {
    pub input: Vec<String>,
    pub model: String,
}

#[derive(Deserialize, Debug)]
pub struct GenericEmbeddingResponse {
    pub embeddings: Vec<Vec<f64>>,
}

pub fn prepare_generic_embedding_request(
    model: &Model,
    inputs: &[Inputs],
) -> GenericEmbeddingRequest {
    let text_inputs = trim_inputs(inputs);

    GenericEmbeddingRequest {
        input: text_inputs,
        model: model.api_name(),
    }
}

pub fn get_provider(
    model_source: &ModelSource,
    api_key: Option<String>,
    url: Option<String>,
) -> Result<Box<dyn EmbeddingProvider>, VectorizeError> {
    match model_source {
        ModelSource::OpenAI => Ok(Box::new(providers::openai::OpenAIProvider::new(
            url, api_key,
        ))),
        ModelSource::Cohere => Ok(Box::new(providers::cohere::CohereProvider::new(
            url, api_key,
        ))),
        ModelSource::SentenceTransformers => Ok(Box::new(
            providers::vector_serve::VectorServeProvider::new(url, api_key),
        )),
        ModelSource::Ollama | ModelSource::Tembo => Err(anyhow::anyhow!(
            "Ollama/Tembo transformer not implemented yet"
        ))?,
    }
}
