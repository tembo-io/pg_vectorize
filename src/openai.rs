use pgrx::prelude::*;
use serde_json::json;

use anyhow::Result;

use crate::{
    executor::Inputs,
    guc::OPENAI_KEY,
    types::{JobParams, PairedEmbeddings},
};

// max token length is 8192
// however, depending on content of text, token count can be higher than
// token count returned by split_whitespace()
// TODO: wrap openai toktoken's tokenizer to estimate token count?
pub const MAX_TOKEN_LEN: usize = 7500;
pub const OPENAI_EMBEDDING_RL: &str = "https://api.openai.com/v1/embeddings";

#[derive(serde::Deserialize, Debug)]
struct EmbeddingResponse {
    // object: String,
    data: Vec<DataObject>,
}

#[derive(serde::Deserialize, Debug)]
struct DataObject {
    // object: String,
    // index: usize,
    embedding: Vec<f64>,
}

// OpenAI embedding model has a limit of 8192 tokens per input
// there can be a number of ways condense the inputs
pub fn trim_inputs(inputs: &[Inputs]) -> Vec<String> {
    inputs
        .iter()
        .map(|input| {
            if input.token_estimate as usize > MAX_TOKEN_LEN {
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

pub async fn openai_embeddings(inputs: &Vec<String>, key: &str) -> Result<Vec<Vec<f64>>> {
    log!("pg-vectorize: openai request size: {}", inputs.len());
    let client = reqwest::Client::new();
    let resp = client
        .post(OPENAI_EMBEDDING_RL)
        .json(&json!({
            "input": inputs,
            "model": "text-embedding-ada-002"
        }))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await?;
    let embedding_resp = handle_response::<EmbeddingResponse>(resp, "embeddings").await?;

    let embeddings = embedding_resp
        .data
        .iter()
        .map(|d| d.embedding.clone())
        .collect();
    Ok(embeddings)
}

pub async fn openai_transform(job_params: JobParams, inputs: &[Inputs]) -> Result<Vec<Vec<f64>>> {
    log!("pg-vectorize: OpenAI transformer");

    // handle retrieval of API key. order of precedence:
    // 1. job parameters
    // 2. GUC
    let apikey = match job_params.api_key {
        Some(k) => k,
        None => {
            let key = match OPENAI_KEY.get() {
                Some(k) => k.to_str()?.to_owned(),
                None => {
                    warning!("pg-vectorize: Error getting API key from GUC");
                    return Err(anyhow::anyhow!("failed to get API key"));
                }
            };
            key
        }
    };

    // trims any inputs that exceed openAIs max token length
    let text_inputs = trim_inputs(inputs);
    let embeddings = match openai_embeddings(&text_inputs, &apikey).await {
        Ok(e) => e,
        Err(e) => {
            warning!("pg-vectorize: Error getting embeddings: {}", e);
            return Err(anyhow::anyhow!("failed to get embeddings"));
        }
    };
    Ok(embeddings)
}

// merges the vec of inputs with the embedding responses
pub fn merge_input_output(inputs: Vec<Inputs>, values: Vec<Vec<f64>>) -> Vec<PairedEmbeddings> {
    inputs
        .into_iter()
        .zip(values)
        .map(|(input, value)| PairedEmbeddings {
            primary_key: input.record_id,
            embeddings: value,
        })
        .collect()
}

pub async fn handle_response<T: for<'de> serde::Deserialize<'de>>(
    resp: reqwest::Response,
    method: &'static str,
) -> Result<T> {
    if !resp.status().is_success() {
        let errmsg = format!(
            "Failed to call method '{}', received response with status code:{} and body: {}",
            method,
            resp.status(),
            resp.text().await?
        );
        warning!("pg-vectorize: error handling response: {}", errmsg);
        return Err(anyhow::anyhow!(errmsg));
    }
    let value = resp.json::<T>().await?;
    Ok(value)
}

pub fn validate_api_key(key: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap_or_else(|e| error!("failed to initialize tokio runtime: {}", e));
    runtime.block_on(async {
        let resp = client
            .get("https://api.openai.com/v1/models")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", key))
            .send()
            .await
            .unwrap_or_else(|e| error!("failed to make Open AI key validation call: {}", e));
        let _ = handle_response::<serde_json::Value>(resp, "models")
            .await
            .unwrap_or_else(|e| error!("failed validate API key: {}", e));
    });
    Ok(())
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
