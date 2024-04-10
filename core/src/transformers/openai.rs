use anyhow::Result;

use crate::plugin::{map_http_transform, EmbeddingRequest};
use crate::transformers::types::{
    EmbeddingPayload,
    // EmbeddingRequest,
    Inputs,
};
use crate::types::{JobParams, ModelSource, VectorizeMeta};

pub const MAX_TOKEN_LEN: usize = 8192;
pub const OPENAI_EMBEDDING_URL: &str = "https://api.openai.com/v1/embeddings";

pub fn prepare_openai_request(
    vect_meta: VectorizeMeta,
    inputs: &[Inputs],
    api_key: Option<String>,
) -> Result<EmbeddingRequest> {
    let text_inputs = trim_inputs(inputs);
    let job_params: JobParams = serde_json::from_value(vect_meta.params.clone())?;
    let payload = EmbeddingPayload {
        input: text_inputs,
        model: vect_meta.transformer.to_string(),
    };

    let apikey = match job_params.api_key {
        Some(k) => k,
        None => match api_key {
            Some(k) => k.to_owned(),
            None => {
                return Err(anyhow::anyhow!("failed to get API key"));
            }
        },
    };

    let trans = map_http_transform(ModelSource::OpenAI);

    Ok(EmbeddingRequest {
        url: OPENAI_EMBEDDING_URL.to_owned(),
        payload,
        api_key: Some(apikey),
        json_transform: trans,
    })
}

// OpenAI embedding model has a limit of 8192 tokens per input
// there can be a number of ways condense the inputs
pub fn trim_inputs(inputs: &[Inputs]) -> Vec<String> {
    inputs
        .iter()
        .map(|input| {
            if input.token_estimate as usize > MAX_TOKEN_LEN {
                // not example taking tokens, but naive way to trim input
                let tokens: Vec<&str> = input.inputs.split_whitespace().collect();
                tokens
                    .into_iter()
                    .take(MAX_TOKEN_LEN)
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                input.inputs.clone()
            }
        })
        .collect()
}

pub fn openai_embedding_dim(model_name: &str) -> i32 {
    match model_name {
        "text-embedding-3-large" => 3072,
        "text-embedding-3-small" => 1536,
        "text-embedding-ada-002" => 1536,
        _ => 1536,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_inputs_no_trimming_required() {
        let data = vec![
            Inputs {
                record_id: "1".to_string(),
                inputs: "token1 token2".to_string(),
                token_estimate: 2,
            },
            Inputs {
                record_id: "2".to_string(),
                inputs: "token3 token4".to_string(),
                token_estimate: 2,
            },
        ];

        let trimmed = trim_inputs(&data);
        assert_eq!(trimmed, vec!["token1 token2", "token3 token4"]);
    }

    #[test]
    fn test_trim_inputs_trimming_required() {
        let token_len = 1000000;
        let long_input = (0..token_len)
            .map(|i| format!("token{}", i))
            .collect::<Vec<_>>()
            .join(" ");

        let num_tokens = long_input.split_whitespace().count();
        assert_eq!(num_tokens, token_len);

        let data = vec![Inputs {
            record_id: "1".to_string(),
            inputs: long_input.clone(),
            token_estimate: token_len as i32,
        }];

        let trimmed = trim_inputs(&data);
        let trimmed_input = trimmed[0].clone();
        let trimmed_length = trimmed_input.split_whitespace().count();
        assert_eq!(trimmed_length, MAX_TOKEN_LEN);
    }

    #[test]
    fn test_trim_inputs_mixed_cases() {
        let num_tokens_in = 1000000;
        let long_input = (0..num_tokens_in)
            .map(|i| format!("token{}", i))
            .collect::<Vec<_>>()
            .join(" ");
        let data = vec![
            Inputs {
                record_id: "1".to_string(),
                inputs: "token1 token2".to_string(),
                token_estimate: 2,
            },
            Inputs {
                record_id: "2".to_string(),
                inputs: long_input.clone(),
                token_estimate: num_tokens_in,
            },
        ];

        let trimmed = trim_inputs(&data);
        assert_eq!(trimmed[0].split_whitespace().count(), 2);
        assert_eq!(trimmed[1].split_whitespace().count(), MAX_TOKEN_LEN);
    }
}
