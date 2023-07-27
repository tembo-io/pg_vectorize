use pgrx::prelude::*;
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub pg_conn_str: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pg_conn_str: from_env_default(
                "DATABASE_URL",
                "postgresql://postgres:postgres@localhost:5432/postgres",
            ),
        }
    }
}

/// source a variable from environment - use default if not exists
pub fn from_env_default(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}

pub fn get_vectorize_meta_spi(job_name: &str) -> Option<pgrx::JsonB> {
    // TODO: change to bind param
    let query = "
        SELECT params::jsonb
        FROM vectorize.vectorize_meta
        WHERE name = $1
    ";
    let r: Result<Option<pgrx::JsonB>, spi::Error> = Spi::get_one_with_args(
        &query,
        vec![(PgBuiltInOids::TEXTOID.oid(), job_name.into_datum())],
    );
    r.expect("failed to query vectorizie metadata table")
}
