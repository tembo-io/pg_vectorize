use anyhow::Result;
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use url::Url;

use super::types::EmbeddingRequest;

pub struct OllamaInstance {
    pub model_name: String,
    pub instance: Ollama,
}

pub trait LLMFunctions {
    fn new(model_name: String, url: String) -> Self;
    #[allow(async_fn_in_trait)]
    async fn generate_reponse(&self, prompt_text: String) -> Result<String, String>;
    #[allow(async_fn_in_trait)]
    async fn generate_embedding(&self, inputs: String) -> Result<Vec<f64>, String>;
}

impl LLMFunctions for OllamaInstance {
    fn new(model_name: String, url: String) -> Self {
        let parsed_url = Url::parse(&url).unwrap_or_else(|_| panic!("invalid url: {}", url));
        let instance = Ollama::new(
            format!(
                "{}://{}",
                parsed_url.scheme(),
                parsed_url.host_str().expect("parsed url missing")
            ),
            parsed_url.port().expect("parsed port missing"),
        );
        OllamaInstance {
            model_name,
            instance,
        }
    }
    async fn generate_reponse(&self, prompt_text: String) -> Result<String, String> {
        let req = GenerationRequest::new(self.model_name.clone(), prompt_text);
        println!("ollama instance: {:?}", self.instance);
        let res = self.instance.generate(req).await;
        match res {
            Ok(res) => Ok(res.response),
            Err(e) => Err(e.to_string()),
        }
    }
    async fn generate_embedding(&self, input: String) -> Result<Vec<f64>, String> {
        let embed = self
            .instance
            .generate_embeddings(self.model_name.clone(), input, None)
            .await;
        match embed {
            Ok(res) => Ok(res.embeddings),
            Err(e) => Err(e.to_string()),
        }
    }
}

pub fn ollama_embedding_dim(model_name: &str) -> i32 {
    match model_name {
        "llama2" => 5192,
        _ => 1536,
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

pub fn generate_embeddings(request: EmbeddingRequest) -> Result<Vec<Vec<f64>>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap_or_else(|e| panic!("failed to initialize tokio runtime: {}", e));

    runtime.block_on(async {
        let instance = OllamaInstance::new(request.payload.model, request.url);
        let mut embeddings: Vec<Vec<f64>> = vec![];
        for input in request.payload.input {
            let response = instance.generate_embedding(input).await;
            let embedding = match response {
                Ok(embed) => embed,
                Err(e) => panic!("Unable to generate embeddings.\nError: {:?}", e),
            };
            embeddings.push(embedding);
        }
        Ok(embeddings)
    })
}
