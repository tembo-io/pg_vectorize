use chrono::serde::ts_seconds_option::deserialize as from_tsopt;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::Utc;
use sqlx::FromRow;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub const VECTORIZE_SCHEMA: &str = "vectorize";

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize)]
// SimilarityAlg is now deprecated
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

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IndexDist {
    pgv_hnsw_l2,
    pgv_hnsw_ip,
    pgv_hnsw_cosine,
}

impl Display for IndexDist {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            IndexDist::pgv_hnsw_l2 => write!(f, "pgv_hnsw_l2"),
            IndexDist::pgv_hnsw_ip => write!(f, "pgv_hnsw_ip"),
            IndexDist::pgv_hnsw_cosine => write!(f, "pgv_hnsw_cosine"),
        }
    }
}

impl FromStr for IndexDist {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pgv_hnsw_l2" => Ok(IndexDist::pgv_hnsw_l2),
            "pgv_hnsw_ip" => Ok(IndexDist::pgv_hnsw_ip),
            "pgv_hnsw_cosine" => Ok(IndexDist::pgv_hnsw_cosine),
            _ => Err(format!("Invalid value for IndexDist: {}", s)),
        }
    }
}

impl From<String> for IndexDist {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pgv_hnsw_l2" => IndexDist::pgv_hnsw_l2,
            "pgv_hnsw_ip" => IndexDist::pgv_hnsw_ip,
            "pgv_hnsw_cosine" => IndexDist::pgv_hnsw_cosine,
            _ => panic!("Invalid value for IndexDist: {}", s),
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
    pub index_dist_type: IndexDist,
    pub transformer: String,
    // search_alg and SimilarityAlg are now deprecated
    pub search_alg: SimilarityAlg,
    pub params: serde_json::Value,
    #[serde(deserialize_with = "from_tsopt")]
    pub last_completion: Option<chrono::DateTime<Utc>>,
}
