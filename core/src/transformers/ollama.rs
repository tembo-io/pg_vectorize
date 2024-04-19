use anyhow::Result;
use ollama_rs::{
    generation::{completion::request::GenerationRequest, options::GenerationOptions},
    Ollama,
};
use url::Url;

use crate::transformers::openai::trim_inputs;
use crate::transformers::types::{EmbeddingPayload, EmbeddingRequest, Inputs};
use crate::types;

pub struct OllamaInstance {
    pub model_name: String,
    pub instance: Ollama,
}

pub trait LLMFunctions {
    fn new(model_name: String, url: String) -> Self;
    async fn generate_reponse(&self, prompt_text: String) -> Result<String, String>;
}

impl LLMFunctions for OllamaInstance {
    fn new(model_name: String, url: String) -> Self {
        let parsed_url = Url::parse(&url).expect(format!("invalid url: {}", url).as_str());
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
}

pub fn prepare_ollama_embedding_request(
    vect_meta: types::VectorizeMeta,
    inputs: &[Inputs],
    model_url: String,
) -> Result<EmbeddingRequest> {
    let text_inputs = trim_inputs(inputs);
    let payload = EmbeddingPayload {
        input: text_inputs,
        model: vect_meta.transformer.to_string(),
    };

    // TBD
    let _job_params: types::JobParams = serde_json::from_value(vect_meta.params)?;

    Ok(EmbeddingRequest {
        url: model_url,
        payload,
        api_key: None,
    })
}

pub fn ollama_embedding_dim(model_name: &str) -> i32 {
    match model_name {
        "llama2" => 5192,
        _ => 1536,
    }
}
