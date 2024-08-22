use super::{EmbeddingProvider, GenericEmbeddingRequest, GenericEmbeddingResponse};
use crate::errors::VectorizeError;
use async_trait::async_trait;
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use serde::{Deserialize, Serialize};
use url::Url;

pub const OLLAMA_BASE_URL: &str = "http://localhost:3001";

pub struct OllamaProvider {
    pub instance: Ollama,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ModelInfo {
    embedding_dimension: u32,
    max_seq_len: u32,
}

impl OllamaProvider {
    pub fn new(url: Option<String>) -> Self {
        let url_in = url.unwrap_or_else(|| OLLAMA_BASE_URL.to_string());
        let parsed_url = Url::parse(&url_in).unwrap_or_else(|_| panic!("invalid url: {}", url_in));
        let instance = Ollama::new(
            format!(
                "{}://{}",
                parsed_url.scheme(),
                parsed_url.host_str().expect("parsed url missing")
            ),
            parsed_url.port().expect("parsed port missing"),
        );
        OllamaProvider { instance }
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn generate_embedding<'a>(
        &self,
        request: &'a GenericEmbeddingRequest,
    ) -> Result<GenericEmbeddingResponse, VectorizeError> {
        let mut all_embeddings: Vec<Vec<f64>> = Vec::with_capacity(request.input.len());
        let model_name = request.model.clone();
        for ipt in request.input.iter() {
            let embed = self
                .instance
                .generate_embeddings(model_name.clone(), ipt.clone(), None)
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
    pub async fn generate_response(
        &self,
        model_name: String,
        prompt_text: &str,
    ) -> Result<String, VectorizeError> {
        let req = GenerationRequest::new(model_name, prompt_text.to_owned());
        let res = self.instance.generate(req).await?;
        Ok(res.response)
    }
}

pub fn check_model_host(url: &str) -> Result<String, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap_or_else(|e| panic!("failed to initialize tokio runtime: {}", e));

    runtime.block_on(async {
        let response = reqwest::get(url).await.unwrap();
        match response.status() {
            reqwest::StatusCode::OK => Ok(format!("Success! {:?}", response)),
            _ => Err(format!("Error! {:?}", response)),
        }
    })
}

pub fn ollama_embedding_dim(model_name: &str) -> i32 {
    match model_name {
        "llama2" => 5192,
        _ => 1536,
    }
}
