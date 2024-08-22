use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{EmbeddingProvider, GenericEmbeddingRequest, GenericEmbeddingResponse};
use crate::errors::VectorizeError;
use crate::transformers::http_handler::handle_response;
use async_trait::async_trait;
use std::env;

pub const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

pub struct OpenAIProvider {
    pub url: String,
    pub api_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenAIEmbeddingBody {
    pub model: String,
    pub input: Vec<String>,
}

impl From<GenericEmbeddingRequest> for OpenAIEmbeddingBody {
    fn from(request: GenericEmbeddingRequest) -> Self {
        OpenAIEmbeddingBody {
            model: request.model,
            input: request.input,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenAIEmbeddingResponse {
    pub model: String,
    pub data: Vec<EmbeddingObject>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EmbeddingObject {
    pub index: usize,
    pub embedding: Vec<f64>,
}

impl From<OpenAIEmbeddingResponse> for GenericEmbeddingResponse {
    fn from(response: OpenAIEmbeddingResponse) -> Self {
        GenericEmbeddingResponse {
            embeddings: response.data.iter().map(|x| x.embedding.clone()).collect(),
        }
    }
}

impl OpenAIProvider {
    pub fn new(url: Option<String>, api_key: Option<String>) -> Self {
        let final_url = match url {
            Some(url) => url,
            None => OPENAI_BASE_URL.to_string(),
        };
        let final_api_key = match api_key {
            Some(api_key) => api_key,
            None => env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set"),
        };
        OpenAIProvider {
            url: final_url,
            api_key: final_api_key,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIProvider {
    async fn generate_embedding<'a>(
        &self,
        request: &'a GenericEmbeddingRequest,
    ) -> Result<GenericEmbeddingResponse, VectorizeError> {
        let client = Client::new();

        let req = OpenAIEmbeddingBody::from(request.clone());
        let num_inputs = request.input.len();
        let todo_requests: Vec<OpenAIEmbeddingBody> = if num_inputs > 2048 {
            split_vector(req.input, 2048)
                .iter()
                .map(|chunk| OpenAIEmbeddingBody {
                    input: chunk.clone(),
                    model: request.model.clone(),
                })
                .collect()
        } else {
            vec![req]
        };

        let mut all_embeddings: Vec<Vec<f64>> = Vec::with_capacity(num_inputs);

        for request_payload in todo_requests.iter() {
            let payload_val = serde_json::to_value(request_payload)?;
            let embeddings_url = format!("{}/embeddings", self.url);
            let response = client
                .post(&embeddings_url)
                .timeout(std::time::Duration::from_secs(120_u64))
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&payload_val)
                .send()
                .await?;

            let embeddings =
                handle_response::<OpenAIEmbeddingResponse>(response, "embeddings").await?;
            all_embeddings.extend(embeddings.data.iter().map(|x| x.embedding.clone()));
        }
        Ok(GenericEmbeddingResponse {
            embeddings: all_embeddings,
        })
    }

    async fn model_dim(&self, model_name: &str) -> Result<u32, VectorizeError> {
        Ok(openai_embedding_dim(model_name) as u32)
    }
}

pub fn openai_embedding_dim(model_name: &str) -> i32 {
    match model_name {
        "text-embedding-3-large" => 3072,
        "text-embedding-3-small" => 1536,
        "text-embedding-ada-002" => 1536,
        _ => 1536,
    }
}

fn split_vector(vec: Vec<String>, chunk_size: usize) -> Vec<Vec<String>> {
    vec.chunks(chunk_size).map(|chunk| chunk.to_vec()).collect()
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio::test as async_test;

    #[async_test]
    async fn test_generate_embedding() {
        let provider = OpenAIProvider::new(Some(OPENAI_BASE_URL.to_string()), None);
        let request = GenericEmbeddingRequest {
            model: "text-embedding-ada-002".to_string(),
            input: vec!["hello world".to_string()],
        };

        let embeddings = provider.generate_embedding(&request).await.unwrap();
        assert!(
            !embeddings.embeddings.is_empty(),
            "Embeddings should not be empty"
        );
        assert!(
            embeddings.embeddings.len() == 1,
            "Embeddings should have length 1"
        );
        assert!(
            embeddings.embeddings[0].len() == 1536,
            "Embeddings should have length 1536"
        );
    }
}
