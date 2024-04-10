use crate::{
    transformers::types::{EmbeddingPayload, EmbeddingResponse},
    types::ModelSource,
};
use fluvio_jolt::{transform, TransformSpec};
use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json::json;

// mapping of the EmbeddingPayload -> destination
lazy_static! {
    pub static ref TEMBO_VECTOR_SERVE_REQ: TransformSpec = serde_json::from_value(json!([
        {
          "operation": "shift",
          "spec": {
            "input": "input",
            "model": "model"
          }
        }
      ])).unwrap();
      pub static ref OPENAI_REQ: TransformSpec = serde_json::from_value(json!([
        {
          "operation": "shift",
          "spec": {
            "input": "input",
            "model": "model"
          }
        }
      ])).unwrap();
      /// Hugging Face Sentence Embedding Endpoints include model name in the endpoint url
      pub static ref HUGGING_FACE_REQ: TransformSpec = serde_json::from_value(json!([
        {
          "operation": "shift",
          "spec": {
            "input": "inputs",
          }
        }
      ])).unwrap();
}

// mapping of destination response -> EmbeddingResponse
lazy_static! {
    pub static ref TEMBO_VECTOR_SERVE_RESPONSE: TransformSpec = serde_json::from_value(json!([
    {
        "operation": "shift",
        "spec": {
          "data": {
              "*": {
                  "@(embedding)": "embeddings[]",
              }
          },
        }
      }
    ]))
    .unwrap();
    pub static ref OPENAI_RESPONSE: TransformSpec = serde_json::from_value(json!([
      {
        "operation": "shift",
        "spec": {
          "data": {
              "*": {
                  "@(embedding)": "embeddings[]",
              }
          },
        }
      }
    ]))
    .unwrap();
    pub static ref HUGGING_FACE_RESPONSE: TransformSpec = serde_json::from_value(json!([
      {
        "operation": "shift",
        "spec": {
          "*": "embeddings",
        }
      }
    ]))
    .unwrap();
}

pub struct EmbeddingRequest {
    pub url: String,
    pub payload: EmbeddingPayload,
    pub api_key: Option<String>,
    pub json_transform: HttpTransform,
}

use anyhow::Result;

pub async fn embedding_request(
    request: EmbeddingRequest,
    timeout: u64,
) -> Result<EmbeddingResponse> {
    let client = reqwest::Client::new();

    let mapped_request = transform(
        serde_json::to_value(request.payload)?,
        &request.json_transform.request,
    )?;
    let mut req = client
        .post(&request.url)
        .timeout(std::time::Duration::from_secs(timeout))
        .json(&mapped_request)
        .header("Content-Type", "application/json");
    if let Some(key) = request.api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req.send().await?;
    if !resp.status().is_success() {
        let errmsg = format!(
            "Failed embedding request to {}, received response with status code:{} and body: {}",
            request.url,
            resp.status(),
            resp.text().await?
        );
        return Err(anyhow::anyhow!(errmsg));
    }
    let resp_value = resp.json().await?;
    let transformed = transform(resp_value, &request.json_transform.response).unwrap();
    let embedding_response = serde_json::from_value(transformed)?;
    Ok(embedding_response)
}

#[derive(Clone, Debug, Deserialize)]
pub struct HttpTransform {
    pub request: TransformSpec,
    pub response: TransformSpec,
}

// maps model to request/response transformations
pub fn map_http_transform(model: ModelSource) -> HttpTransform {
    match model {
        ModelSource::OpenAI => HttpTransform {
            request: OPENAI_REQ.clone(),
            response: OPENAI_RESPONSE.clone(),
        },
        ModelSource::SentenceTransformers => HttpTransform {
            request: TEMBO_VECTOR_SERVE_REQ.clone(),
            response: TEMBO_VECTOR_SERVE_RESPONSE.clone(),
        },
    }
}
