use sqlx::error::Error as DbError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TemboError {
    #[error("Missing attribute: {0}")]
    ValidationError(String),
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    DbError(#[from] DbError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
}
