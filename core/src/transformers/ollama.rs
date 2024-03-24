use ollama_rs::{Ollama, generation::completion::request::GenerationRequest};

pub struct OllamaInstance{
    model_name: String,
    host_url: String,
    host_port: u16,
    instance: Ollama
}

pub trait LLMFunctions{
    fn new(model_name: String, host_url: String, host_port: u16) -> Self;
    async fn generate_reponse(&self, prompt_text: String) -> Result<String, String>;
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
}
