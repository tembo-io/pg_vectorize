use pgrx::prelude::*;

#[derive(serde::Deserialize, Debug)]
struct EmbeddingResponse {
    object: String,
    data: Vec<DataObject>,
}

#[derive(serde::Deserialize, Debug)]
struct DataObject {
    object: String,
    index: usize,
    embedding: Vec<f64>,
}

pub async fn get_embeddings(inputs: &Vec<String>, key: &str) -> Vec<Vec<f64>> {
    use serde_json::json;
    let url = "https://api.openai.com/v1/embeddings";
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .json(&json!({
            "input": inputs,
            "model": "text-embedding-ada-002"
        }))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .expect("failed calling openai");
    let embedding_resp = handle_response::<EmbeddingResponse>(resp, "embeddings")
        .await
        .unwrap();
    let embeddings = embedding_resp
        .data
        .iter()
        .map(|d| d.embedding.clone())
        .collect();
    embeddings
}

// thanks Evan :D
pub async fn handle_response<T: for<'de> serde::Deserialize<'de>>(
    resp: reqwest::Response,
    method: &'static str,
) -> Result<T, Box<dyn std::error::Error>> {
    if !resp.status().is_success() {
        let errmsg = format!(
            "Failed to call method '{}', received response with status code:{} and body: {}",
            method,
            resp.status(),
            resp.text().await?
        );
        error!("{}", errmsg);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            errmsg,
        )));
    }
    let value = resp.json::<T>().await?;
    Ok(value)
}
