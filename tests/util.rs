pub mod common {
    use log::LevelFilter;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use sqlx::ConnectOptions;
    use sqlx::{Pool, Postgres, Row};
    use url::{ParseError, Url};

    pub async fn connect(url: &str) -> Pool<Postgres> {
        let options = conn_options(url).expect("failed to parse url");
        println!("URL: {}", url);
        PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(10))
            .max_connections(5)
            .connect_with(options)
            .await
            .expect("failed to connect to pg")
    }

    pub async fn init_database() -> Pool<Postgres> {
        let username = whoami::username();
        let database_port = database_port();

        let conn = connect(&format!(
            "postgres://{username}:postgres@localhost:{database_port}/postgres"
        ))
        .await;

        let _ = sqlx::query("DROP EXTENSION IF EXISTS vectorize CASCADE")
        .execute(&conn)
        .await
        .expect("failed to create extension");

        // CREATE EXTENSION
        let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS vectorize CASCADE")
            .execute(&conn)
            .await
            .expect("failed to create extension");

        conn
    }

    pub fn database_port() -> usize {
        if cfg!(feature = "pg16") {
            28816
        } else if cfg!(feature = "pg15") {
            28815
        } else if cfg!(feature = "pg14") {
            28814
        } else if cfg!(feature = "pg13") {
            28813
        } else if cfg!(feature = "pg12") {
            28812
        } else {
            5432
        }
    }

    pub fn conn_options(url: &str) -> Result<PgConnectOptions, ParseError> {
        // Parse url
        let parsed = Url::parse(url)?;
        let options = PgConnectOptions::new()
            .host(parsed.host_str().ok_or(ParseError::EmptyHost)?)
            .port(parsed.port().ok_or(ParseError::InvalidPort)?)
            .username(parsed.username())
            .password(parsed.password().ok_or(ParseError::IdnaError)?)
            .database(parsed.path().trim_start_matches('/'))
            .log_statements(LevelFilter::Debug);
        Ok(options)
    }

    pub async fn init_test_table(test_num: i32, conn: &Pool<Postgres>) -> String {
        let table = format!("product_{test_num}");
        let q = format!("CREATE TABLE {table} as SELECT * from vectorize.example_products;");
        let _ = sqlx::query(&q)
            .execute(conn)
            .await
            .expect("failed to create test table");
        table
    }

    pub async fn row_count(fq_table_name: &str, conn: &Pool<Postgres>) -> i64 {
        let q = format!(
            "SELECT COUNT(*) FROM {fq_table_name};",
            fq_table_name = fq_table_name
        );
        sqlx::query(&q)
            .fetch_one(conn)
            .await
            .expect("failed to select from test_table")
            .get::<i64, usize>(0)
    }

    pub async fn init_embedding_svc_url(conn: &Pool<Postgres>) {
        let set_guc = format!(
            "ALTER SYSTEM SET vectorize.embedding_service_url to 'http://0.0.0.0:3000/v1/embeddings';");
        let reload = "SELECT pg_reload_conf();".to_string();
        for q in vec![set_guc, reload] {
            sqlx::query(&q)
                .execute(conn)
                .await
                .expect("failed to init embedding svc url");
        }
    }
}
