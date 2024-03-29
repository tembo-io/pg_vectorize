use ollama_rs::{generation::{completion::request::GenerationRequest, options::GenerationOptions}, Ollama};
use anyhow::Result;

use crate::transformers::types::{EmbeddingPayload, EmbeddingRequest, Inputs};
use crate::types;
use crate::transformers::openai::trim_inputs;

pub struct OllamaInstance{
    model_name: String,
    host_url: String,
    host_port: u16,
    instance: Ollama
}

pub trait LLMFunctions{
    fn new(model_name: String, host_url: String, host_port: u16) -> Self;
    async fn generate_reponse(&self, prompt_text: String) -> Result<String, String>;
    async fn generate_emebeddings(&self, input: String) -> Result<Vec<f64>, String>;
}

impl LLMFunctions for OllamaInstance{
    fn new(model_name: String, host_url: String, host_port: u16) -> Self{
        let instance = Ollama::new(host_url.clone(), host_port);
        OllamaInstance{
            model_name,
            host_url,
            host_port,
            instance
        }
    }
    async fn generate_reponse(&self, prompt_text: String) -> Result<String, String>{
        let res = self.instance.generate(GenerationRequest::new(self.model_name.clone(), prompt_text)).await;
        if let Ok(res) = res {
            return Ok(res.response);
        }
        return Err("Unable to generate any response".to_string());
    }
    async fn generate_emebeddings(&self, input: String) -> Result<Vec<f64>, String>{
        let embedding = self.instance.generate_embeddings(self.model_name.clone(), input, None).await;
        if let Ok(embedding) = embedding {
            return Ok(embedding.embeddings);
        }
        return Err("Unable to generate embeddings".to_string());
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
        model: vect_meta.transformer.to_string()
    };

    // TBD
    let _job_params: types::JobParams = serde_json::from_value(vect_meta.params)?;

    Ok(EmbeddingRequest{
        url: model_url,
        payload,
        api_key: None,
    })

}
