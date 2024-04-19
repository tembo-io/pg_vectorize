use pgrx::info;
use vectorize_core::transformers::{ollama::{LLMFunctions, OllamaInstance}, types::EmbeddingPayload};
use anyhow::Result;

pub fn init_llm_instance(model_name: &str, host_url: &str) -> OllamaInstance{
    let instance = OllamaInstance::new(
        model_name.to_string(), 
        host_url.to_string(), 
    );
    instance
}

pub async fn ollama_embedding_request(host_url: &str, payload: EmbeddingPayload) -> Result<Vec<Vec<f64>>>{
    let spl: Vec<&str> = host_url.split(":").collect();
    let model_name: Vec<&str> = payload.model.split("/").collect();
    let model = init_llm_instance(model_name[0], host_url);
    let mut embeds: Vec<Vec<f64>> = vec![];
    for input in payload.input{
        let embeddings = model.generate_emebeddings(input).await.unwrap();
        embeds.push(embeddings);
    }
    return Ok(embeds);
}
