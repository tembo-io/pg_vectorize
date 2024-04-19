use anyhow::Result;
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use url::Url;

pub struct OllamaInstance {
    pub model_name: String,
    pub instance: Ollama,
}

pub trait LLMFunctions {
    fn new(model_name: String, url: String) -> Self;
    #[allow(async_fn_in_trait)]
    async fn generate_reponse(&self, prompt_text: String) -> Result<String, String>;
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
}

pub fn ollama_embedding_dim(model_name: &str) -> i32 {
    match model_name {
        "llama2" => 5192,
        _ => 1536,
    }
}
