use reqwest;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
struct ChatData {
    output: String,
}

// benchmark to evaluate latency overhead from the insert trigger
async fn bench_insert_triggers() {
    let database_url = std::env::var("DATABASE_URL").unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();

    sqlx::query("DROP TABLE IF EXISTS nemotron_chat CASCADE;")
        .execute(&pool)
        .await
        .unwrap();

    // Create table if it doesn't exist
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS nemotron_chat (
            id SERIAL PRIMARY KEY,
            output TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    // initialize vectorize job on the empty table
    sqlx::query(
        "SELECT vectorize.table(
        job_name => 'nemotron_chat',
        relation => 'nemotron_chat',
        primary_key => 'id',
        columns => ARRAY['output'],
        transformer => 'sentence-transformers/all-MiniLM-L6-v2',
        schedule => 'realtime'
    );",
    )
    .execute(&pool)
    .await
    .expect("failed to init job");

    let bench_data = download_dataset().await;

    insert_data(&pool, bench_data).await;

    println!("Data loaded successfully!");
}

fn read_jsonl_file(path: &str) -> Vec<ChatData> {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let mut items = Vec::new();

    for line in reader.lines() {
        let line = line.unwrap();
        match serde_json::from_str::<ChatData>(&line) {
            Ok(item) => items.push(item),
            Err(e) => eprintln!("Failed to parse line: {}", e),
        }
    }
    items
}

async fn download_dataset() -> Vec<ChatData> {
    let file_path = "chat.jsonl";
    let url = "https://huggingface.co/datasets/nvidia/Llama-Nemotron-Post-Training-Dataset-v1/resolve/main/SFT/chat/chat.jsonl";

    // check if the file exists locally, download if not found
    if !Path::new(file_path).exists() {
        println!("File not found locally. Downloading from {url}");
        let client = reqwest::Client::new();
        let response = client.get(url).send().await.unwrap();
        let content = response.text().await.unwrap();

        // Save the downloaded content to local file
        fs::write(file_path, &content).unwrap();
    } else {
        println!("File found locally.");
    }
    let data: Vec<ChatData> = read_jsonl_file(file_path);

    data
}

async fn insert_data(pool: &Pool<Postgres>, data: Vec<ChatData>) {
    let start = std::time::Instant::now();
    let num_rows = data.len();

    let mut query_builder = String::from("INSERT INTO nemotron_chat (output) VALUES ");
    let mut param_index = 1;

    for (i, _) in data.iter().enumerate() {
        if i > 0 {
            query_builder.push_str(", ");
        }
        query_builder.push_str(&format!("(${})", param_index));
        param_index += 1;
    }
    query_builder.push(';');
    let mut query = sqlx::query(&query_builder);

    // Bind all parameters
    for example in data.iter() {
        query = query.bind(&example.output);
    }
    query.execute(pool).await.unwrap();

    let duration = start.elapsed();
    println!("Time elapsed: {:?}, num records: {}", duration, num_rows);
}

#[tokio::main]
async fn main() {
    bench_insert_triggers().await;
}
