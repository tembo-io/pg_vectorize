use actix_web::{post, web, App, HttpServer, Responder, Result};
use actix_web::error::ErrorInternalServerError;
use anyhow::Context;
use rust_bert::pipelines::sentence_embeddings::{Embedding, SentenceEmbeddingsBuilder};
use serde::{Deserialize, Serialize};


#[derive(Deserialize)]
struct EmbeddingGenerationInput {
    texts_to_embed: Vec<String>,
}

#[derive(Serialize)]
struct GeneratedEmbeddings {
    embeddings: Vec<Vec<Embedding>>
}

fn get_embeddings(vec: &Vec<String>) -> anyhow::Result<Vec<Embedding>> {
    let model = SentenceEmbeddingsBuilder::local("resources/all-MiniLM-L12-v2")
        .with_device(tch::Device::cuda_if_available())
        .create_model().context("Unable to create an instant of bert model")?;
    let embeddings = model.encode(&vec)?;
    Ok(embeddings)
}

#[post("/generate-embeddings")]
async fn generate_embeddings(input: web::Json<EmbeddingGenerationInput>) -> Result<impl Responder> {
    match get_embeddings(&input.texts_to_embed) {
        Ok(embeddings) => Ok(web::Json(embeddings)),
        Err(error) => Err(ErrorInternalServerError(error)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(generate_embeddings))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}