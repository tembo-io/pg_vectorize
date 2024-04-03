use chrono::serde::ts_seconds_option::deserialize as from_tsopt;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::Utc;
use sqlx::FromRow;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

pub const VECTORIZE_SCHEMA: &str = "vectorize";

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimilarityAlg {
    pgv_cosine_similarity,
}

impl Display for SimilarityAlg {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            SimilarityAlg::pgv_cosine_similarity => write!(f, "pgv_cosine_similarity"),
        }
    }
}

impl FromStr for SimilarityAlg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pgv_cosine_similarity" => Ok(SimilarityAlg::pgv_cosine_similarity),
            _ => Err(format!("Invalid value: {}", s)),
        }
    }
}

impl From<String> for SimilarityAlg {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pgv_cosine_similarity" => SimilarityAlg::pgv_cosine_similarity, // ... handle other variants ...
            _ => panic!("Invalid value for SimilarityAlg: {}", s), // or handle this case differently
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JobType {
    Columns,
    // row,
    // url,
}

impl FromStr for JobType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Columns" => Ok(JobType::Columns),
            _ => Err(format!("Invalid value: {}", s)),
        }
    }
}

impl From<String> for JobType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Columns" => JobType::Columns,
            _ => panic!("Invalid value for JobType: {}", s), // or handle this case differently
        }
    }
}

impl Display for JobType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            JobType::Columns => write!(f, "Columns"),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum TableMethod {
    // append a new column to the existing table
    append,
    // join existing table to a new table with embeddings
    #[default]
    join,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, FromRow)]
pub struct JobParams {
    pub schema: String,
    pub table: String,
    pub columns: Vec<String>,
    pub update_time_col: Option<String>,
    pub table_method: TableMethod,
    pub primary_key: String,
    pub pkey_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default = "default_schedule")]
    pub schedule: String,
}

fn default_schedule() -> String {
    "realtime".to_string()
}

// schema for all messages that hit pgmq
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct JobMessage {
    pub job_name: String,
    pub job_meta: VectorizeMeta,
    pub inputs: Vec<crate::transformers::types::Inputs>,
}

// schema for every job
// also schema for the vectorize.vectorize_meta table
#[derive(Clone, Debug, Deserialize, FromRow, Serialize)]
pub struct VectorizeMeta {
    pub job_id: i64,
    pub name: String,
    pub job_type: JobType,
    pub transformer: String,
    pub search_alg: SimilarityAlg,
    pub params: serde_json::Value,
    #[serde(deserialize_with = "from_tsopt")]
    pub last_completion: Option<chrono::DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Model {
    pub source: ModelSource,
    pub name: String,
}

#[derive(Debug, Error, PartialEq)]
pub enum ModelError {
    #[error("Database error")]
    InvalidSource,
    #[error("Invalid model format: {0}")]
    InvalidFormat(String),
}

impl Model {
    pub fn new(input: &str) -> Result<Self, ModelError> {
        let mut parts: Vec<&str> = input.split('/').collect();
        let missing_source = parts.len() != 2;
        if missing_source && parts[0] == "text-embedding-ada-002" {
            // for backwards compatibility, prepend "openai" to text-embedding-ada-2
            parts.insert(0, "openai");
        } else if missing_source && parts[0] == "all-MiniLM-L12-v2" {
            parts.insert(0, "sentence-transformers");
        } else if missing_source {
            return Err(ModelError::InvalidFormat(input.to_string()));
        }

        let source = parts[0]
            .parse::<ModelSource>()
            .map_err(|_| ModelError::InvalidSource)?;

        Ok(Self {
            source,
            name: parts[1].to_string(),
        })
    }
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.source, self.name)
    }
}

// model sources are places that serve models
// each source can have its own API schema
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ModelSource {
    OpenAI,
    SentenceTransformers,
}

impl FromStr for ModelSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ModelSource::OpenAI),
            "sentence-transformers" => Ok(ModelSource::SentenceTransformers),
            _ => Err(format!("Invalid value: {}", s)),
        }
    }
}

impl Display for ModelSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ModelSource::OpenAI => write!(f, "openai"),
            ModelSource::SentenceTransformers => write!(f, "sentence-transformers"),
        }
    }
}

impl From<String> for ModelSource {
    fn from(s: String) -> Self {
        match s.as_str() {
            "openai" => ModelSource::OpenAI,
            "sentence-transformers" => ModelSource::SentenceTransformers,
            _ => panic!("Invalid value for ModelSource: {}", s),
        }
    }
}

// test
#[cfg(test)]
mod model_tests {
    use super::*;

    #[test]
    fn test_valid_model_openai() {
        let model = Model::new("openai/model-name").unwrap();
        assert_eq!(model.source, ModelSource::OpenAI);
        assert_eq!(model.name, "model-name");
    }

    #[test]
    fn test_valid_model_sentencetransformers() {
        let model = Model::new("sentence-transformers/model-name").unwrap();
        assert_eq!(model.source, ModelSource::SentenceTransformers);
        assert_eq!(model.name, "model-name");
    }

    #[test]
    fn test_invalid_model_source() {
        assert!(Model::new("invalidsource/model-name").is_err());
    }

    #[test]
    fn test_invalid_format_no_slash() {
        assert!(Model::new("openaimodel-name").is_err());
    }

    #[test]
    fn test_invalid_format_extra_slash() {
        assert!(Model::new("openai/model/name").is_err());
    }

    #[test]
    fn test_backwards_compatibility() {
        let model = Model::new("text-embedding-ada-002").unwrap();
        assert_eq!(model.source, ModelSource::OpenAI);
        assert_eq!(model.name, "text-embedding-ada-002");
    }
}
