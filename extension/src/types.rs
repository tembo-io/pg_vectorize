use pgrx::*;
use vectorize_core::types::{SimilarityAlg as CoreSimilarityAlg, TableMethod as CoreTableMethod};

use serde::{Deserialize, Serialize};
pub const VECTORIZE_SCHEMA: &str = "vectorize";

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Default, Serialize, Deserialize, PostgresEnum, PartialEq, Eq)]
pub enum TableMethod {
    append,
    #[default]
    join,
}

impl From<TableMethod> for CoreTableMethod {
    fn from(my_method: TableMethod) -> Self {
        match my_method {
            TableMethod::append => CoreTableMethod::append,
            TableMethod::join => CoreTableMethod::join,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize, PostgresEnum)]
pub enum SimilarityAlg {
    pgv_cosine_similarity,
}

impl From<SimilarityAlg> for CoreSimilarityAlg {
    fn from(mysim: SimilarityAlg) -> Self {
        match mysim {
            SimilarityAlg::pgv_cosine_similarity => CoreSimilarityAlg::pgv_cosine_similarity,
        }
    }
}
