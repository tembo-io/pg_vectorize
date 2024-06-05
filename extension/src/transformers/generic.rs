use anyhow::Result;

use crate::guc;
use vectorize_core::transformers::generic::{find_placeholders, interpolate};

pub fn get_env_interpolated_guc(requested: guc::VectorizeGuc) -> Result<String> {
    if let Some(url) = guc::get_guc(requested.clone()) {
        env_interpolate_string(&url)
    } else {
        match requested {
            guc::VectorizeGuc::EmbeddingServiceUrl => {
                return Err(anyhow::anyhow!("vectorize.embedding_service_url not set"))
            }
            guc::VectorizeGuc::OpenAIServiceUrl => {
                return Err(anyhow::anyhow!("vectorize.openai_service_url not set"))
            }
            _ => return Err(anyhow::anyhow!("GUC not found")),
        }
    }
}

/// Interpolates environment variables into a string
/// if env var is missing, the placeholder is left as a raw string
pub fn env_interpolate_string(input: &str) -> Result<String> {
    if let Some(phs) = find_placeholders(input) {
        let interpolated = interpolate(input, phs)?;
        Ok(interpolated)
    } else {
        Ok(input.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_interpolate_string() {
        let input = "http://${HOST}:8000";
        // set env var
        std::env::set_var("HOST", "localhost");
        let result = env_interpolate_string(input).unwrap();
        assert_eq!(result, "http://localhost:8000");
    }

    #[test]
    fn test_env_interpolate_string_with_placeholder() {
        // a missing env var results in the placeholder being left as a raw string
        let input = "http://localhost:8000/embeddings/{model_name}";
        let result = env_interpolate_string(input);
        assert_eq!(result.unwrap(), input.to_string());
    }
}
