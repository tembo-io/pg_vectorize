use pgrx::prelude::*;
use sqlx::{Pool, Postgres};
use std::env;

use anyhow::Result;

// use log::info as log;

#[derive(Clone, Debug)]
pub struct Config {
    pub pg_conn_str: String,
    pub vectorize_socket_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pg_conn_str: from_env_default(
                "DATABASE_URL",
                "postgresql://postgres:postgres@localhost:5432/postgres",
            ),
            vectorize_socket_url: env::var("VECTORIZE_SOCKET_URL").ok(),
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct PostgresSocketConnection {
    user: Option<String>,
    dbname: Option<String>,
    host: Option<String>,
    password: Option<String>,
    port: Option<u16>, // Add other potential query parameters as needed
}

impl PostgresSocketConnection {
    fn from_unix_socket_string(s: &str) -> Option<Self> {
        let parsed_url = url::Url::parse(s).ok()?;
        let mut connection = PostgresSocketConnection::default();

        for (key, value) in parsed_url.query_pairs() {
            match key.as_ref() {
                "user" => connection.user = Some(value.into_owned()),
                "dbname" => connection.dbname = Some(value.into_owned()),
                "host" => connection.host = Some(value.into_owned()),
                "password" => connection.password = Some(value.into_owned()),
                "port" => connection.port = Some(value.parse::<u16>().expect("invalid port")),
                // Add other potential query parameters as needed
                _ => {} // Ignoring unknown parameters
            }
        }

        Some(connection)
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
        query,
        vec![(PgBuiltInOids::TEXTOID.oid(), job_name.into_datum())],
    );
    r.expect("failed to query vectorize metadata table")
}

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use url::{ParseError, Url};

pub async fn get_pg_conn() -> Result<Pool<Postgres>> {
    let opts = get_pg_options()?;
    let pgp = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(4))
        .max_connections(4)
        .connect_with(opts)
        .await?;
    Ok(pgp)
}

pub fn get_pgc_socket_opt(socket_conn: PostgresSocketConnection) -> Result<PgConnectOptions> {
    let mut opts = PgConnectOptions::new();
    opts = opts.socket(socket_conn.host.expect("missing socket host"));
    if socket_conn.port.is_some() {
        opts = opts.port(socket_conn.port.expect("missing socket port"));
    } else {
        opts = opts.port(5432);
    }
    if socket_conn.dbname.is_some() {
        opts = opts.database(&socket_conn.dbname.expect("missing socket dbname"));
    } else {
        opts = opts.database("postgres");
    }
    if socket_conn.user.is_some() {
        opts = opts.username(&socket_conn.user.expect("missing socket user"));
    } else {
        opts = opts.username("postgres");
    }
    if socket_conn.password.is_some() {
        opts = opts.password(&socket_conn.password.expect("missing socket password"));
    } else {
    }
    Ok(opts)
}

fn get_pgc_tcp_opt(url: Url) -> Result<PgConnectOptions> {
    let options = PgConnectOptions::new()
        .host(url.host_str().ok_or(ParseError::EmptyHost)?)
        .port(url.port().ok_or(ParseError::InvalidPort)?)
        .username(url.username())
        .password(url.password().ok_or(ParseError::IdnaError)?)
        .database(url.path().trim_start_matches('/'));
    log::info!("tcp options: {:?}", options);
    Ok(options)
}

pub fn get_pg_options() -> Result<PgConnectOptions> {
    let cfg = Config::default();
    log!("CONFIG!: {:?}", cfg);

    let opts = match cfg.vectorize_socket_url {
        Some(socket_url) => {
            log!("VECTORIZE_SOCKET_URL={:?}", socket_url);
            let socket_conn = PostgresSocketConnection::from_unix_socket_string(&socket_url)
                .expect("failed to parse socket url");
            get_pgc_socket_opt(socket_conn)
        }
        None => {
            log!("DATABASE_URL={}", cfg.pg_conn_str);
            let url = Url::parse(&cfg.pg_conn_str)?;
            get_pgc_tcp_opt(url)
        }
    };
    opts
}
