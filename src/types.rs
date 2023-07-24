use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub const TEMBO_SCHEMA: &str = "tembo";

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize, PostgresEnum)]
pub enum Transformer {
    openai,
    // bert,
}

impl FromStr for Transformer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "openai" => Ok(Transformer::openai),
            _ => Err(format!("Invalid value: {}", s)),
        }
    }
}

impl From<String> for Transformer {
    fn from(s: String) -> Self {
        match s.as_str() {
            "openai" => Transformer::openai,
            _ => panic!("Invalid value for Transformer: {}", s), // or handle this case differently
        }
    }
}

impl Display for Transformer {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Transformer::openai => write!(f, "openai"),
        }
    }
}

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
