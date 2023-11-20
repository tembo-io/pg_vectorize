use anyhow::{Context, Result};

use crate::{
    executor::VectorizeMeta,
    guc,
    transformers::types::{EmbeddingPayload, EmbeddingRequest, Inputs},
};

use super::openai::trim_inputs;

pub fn prepare_generic_embedding_request(
    job_meta: VectorizeMeta,
    inputs: &[Inputs],
) -> Result<EmbeddingRequest> {
    let text_inputs = trim_inputs(inputs);
    let payload = EmbeddingPayload {
        input: text_inputs,
        model: job_meta.transformer.to_string(),
    };

    let svc_host = guc::get_guc(guc::VectorizeGuc::EmbeddingServiceUrl)
        .context("vectorize.embedding_Service_url is not set")?;

    Ok(EmbeddingRequest {
        url: svc_host,
        payload,
        api_key: None,
    })
}
