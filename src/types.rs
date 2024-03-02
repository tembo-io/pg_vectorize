use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::default;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub const VECTORIZE_SCHEMA: &str = "vectorize";

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize, PostgresEnum)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PostgresEnum)]
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
#[derive(Clone, Debug, Default, Serialize, Deserialize, PostgresEnum, PartialEq, Eq)]
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
    pub api_key: Option<String>,
    #[serde(default = "default_schedule")]
    pub schedule: String,
}

fn default_schedule() -> String {
    "realtime".to_string()
}
