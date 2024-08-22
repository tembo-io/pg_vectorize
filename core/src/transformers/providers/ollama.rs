use super::{EmbeddingProvider, GenericEmbeddingRequest, GenericEmbeddingResponse};
use crate::errors::VectorizeError;
use async_trait::async_trait;
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use serde::{Deserialize, Serialize};
use url::Url;

pub const OLLAMA_BASE_URL: &str = "http://localhost:3001";

pub struct OllamaProvider {
    pub model_name: String,
    pub instance: Ollama,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ModelInfo {
    model: String,
    embedding_dimension: u32,
    max_seq_len: u32,
}

impl OllamaProvider {
    pub fn new(model_name: String, url: String) -> Self {
        let parsed_url = Url::parse(&url).unwrap_or_else(|_| panic!("invalid url: {}", url));
        let instance = Ollama::new(
            format!(
                "{}://{}",
                parsed_url.scheme(),
                parsed_url.host_str().expect("parsed url missing")
            ),
            parsed_url.port().expect("parsed port missing"),
        );
        OllamaProvider {
            model_name,
            instance,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn generate_embedding<'a>(
        &self,
        request: &'a GenericEmbeddingRequest,
    ) -> Result<GenericEmbeddingResponse, VectorizeError> {
        let mut all_embeddings: Vec<Vec<f64>> = Vec::with_capacity(request.input.len());
        for ipt in request.input.iter() {
            let embed = self
                .instance
                .generate_embeddings(self.model_name.clone(), ipt.clone(), None)
                .await?;
            all_embeddings.push(embed.embeddings);
        }
        Ok(GenericEmbeddingResponse {
            embeddings: all_embeddings,
        })
    }

    async fn model_dim(&self, model_name: &str) -> Result<u32, VectorizeError> {
        let dim = match model_name {
            "llama2" => 5192,
            _ => 1536,
        };
        Ok(dim)
    }
}

impl OllamaProvider {
    pub async fn generate_response(&self, prompt_text: String) -> Result<String, VectorizeError> {
        let req = GenerationRequest::new(self.model_name.clone(), prompt_text);
        let res = self.instance.generate(req).await?;
        Ok(res.response)
    }
}
