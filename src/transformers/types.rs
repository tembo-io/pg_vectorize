#[derive(serde::Deserialize, Debug)]
pub struct EmbeddingResponse {
    pub data: Vec<DataObject>,
}

#[derive(serde::Deserialize, Debug)]
pub struct DataObject {
    // object: String,
    // index: usize,
    pub embedding: Vec<f64>,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct EmbeddingRequest {
    pub input: Vec<String>,
    pub model: String,
}
