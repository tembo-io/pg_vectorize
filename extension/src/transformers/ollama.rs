use log::info;
use vectorize_core::transformers::{ollama::{LLMFunctions, OllamaInstance}, types::EmbeddingPayload};
use anyhow::Result;

pub fn init_llm_instance(model_name: &str, host_url: &str, model_port: u16) -> OllamaInstance{
    OllamaInstance::new(
        model_name.to_string(), 
        host_url.to_string(), 
        model_port
    )
}

pub async fn ollama_embedding_request(host_url: &str, model_port: u16, payload: EmbeddingPayload) -> Result<Vec<Vec<f64>>>{
    let model_name: Vec<&str> = payload.model.split("/").collect();
    info!("{:?}", model_name);
    let model = init_llm_instance(model_name[0], host_url, model_port);
    let mut embeds: Vec<Vec<f64>> = vec![];
    for input in payload.input{
        info!("{:?}", input);
        let embeddings = model.generate_emebeddings(input).await.unwrap();
        embeds.push(embeddings);
    }
    return Ok(embeds);
}
