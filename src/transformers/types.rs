use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct EmbeddingResponse {
    pub data: Vec<DataObject>,
}

#[derive(serde::Deserialize, Debug)]
pub struct DataObject {
    // object: String,
    // index: usize,
    pub embedding: Vec<f64>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct EmbeddingRequest {
    pub input: Vec<String>,
    pub model: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Inputs {
    pub record_id: String,   // the value to join the record
    pub inputs: String,      // concatenation of input columns
    pub token_estimate: i32, // estimated token count
}

pub struct PairedEmbeddings {
    pub primary_key: String,
    pub embeddings: Vec<f64>,
}
