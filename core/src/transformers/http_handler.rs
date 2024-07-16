use crate::transformers::types::{
    EmbeddingPayload, EmbeddingRequest, EmbeddingResponse, Inputs, PairedEmbeddings,
};
use anyhow::Result;
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
        return Err(anyhow::anyhow!(errmsg));
    }
    let value = resp.json::<T>().await?;
    Ok(value)
}

// handle an OpenAI compatible embedding transform request
pub async fn openai_embedding_request(
    request: EmbeddingRequest,
    timeout: i32,
) -> Result<Vec<Vec<f64>>> {
    let client = reqwest::Client::new();

    // openai request size limit is 2048 inputs
    let number_inputs = request.payload.input.len();
    let todo_requests: Vec<EmbeddingPayload> = if number_inputs > 2048 {
        split_vector(request.payload.input, 2048)
            .iter()
            .map(|chunk| EmbeddingPayload {
                input: chunk.clone(),
                model: request.payload.model.clone(),
            })
            .collect()
    } else {
        vec![request.payload]
    };

    let mut all_embeddings: Vec<Vec<f64>> = Vec::with_capacity(number_inputs);

    for request_payload in todo_requests.iter() {
        let mut req = client
            .post(&request.url)
            .timeout(std::time::Duration::from_secs(timeout as u64))
            .json::<EmbeddingPayload>(request_payload)
            .header("Content-Type", "application/json");
        if let Some(key) = request.api_key.as_ref() {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let resp = req.send().await?;
        let embedding_resp: EmbeddingResponse =
            handle_response::<EmbeddingResponse>(resp, "embeddings").await?;
        let embeddings: Vec<Vec<f64>> = embedding_resp
            .data
            .iter()
            .map(|d| d.embedding.clone())
            .collect();
        all_embeddings.extend(embeddings);
    }
    Ok(all_embeddings)
}

fn split_vector(vec: Vec<String>, chunk_size: usize) -> Vec<Vec<String>> {
    vec.chunks(chunk_size).map(|chunk| chunk.to_vec()).collect()
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
