use vectorize_core::transformers::ollama::{OllamaInstance, LLMFunctions};

pub fn init_llm_instance(model_name: &str, host_url: &str, model_port: u16) -> OllamaInstance{
    OllamaInstance::new(
        model_name.to_string(), 
        host_url.to_string(), 
        model_port
    )
}
