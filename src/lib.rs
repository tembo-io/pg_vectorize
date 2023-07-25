use pgrx::prelude::*;

mod errors;
mod executor;
mod init;
mod openai;
mod query;
mod types;
mod util;
mod worker;

pgrx::pg_module_magic!();

// example data
extension_sql!(
    r#"

CREATE TABLE extensions (
    ext_id INT,
    ext_name TEXT,
    summary TEXT,
    last_updated_at TIMESTAMP WITH TIME ZONE DEFAULT (now() at time zone 'utc') NOT NULL
);

INSERT INTO extensions(ext_id, ext_name, summary) VALUES (1, 'pg_jsonschema ', 'pg_jsonschema is a PostgreSQL extension adding support for JSON schema validation on json and jsonb data types.');
INSERT INTO extensions(ext_id, ext_name, summary) VALUES (2, 'pgmq ', 'A lightweight distributed message queue. Like AWS SQS and RSMQ but on Postgres. Features
Lightweight - Built with Rust and Postgres only
Guaranteed "exactly once" delivery of messages consumer within a visibility timeout
API parity with AWS SQS and RSMQ
Messages stay in the queue until deleted
Messages can be archived, instead of deleted, for long-term retention and replayability
Table (bloat) maintenance automated with pg_partman
High performance operations with index-only scans.');
INSERT INTO extensions(ext_id, ext_name, summary) VALUES (3, 'pg_cron ', 'pg_cron is a simple cron-based job scheduler for PostgreSQL (10 or higher) that runs inside the database as an extension. It uses the same syntax as regular cron, but it allows you to schedule PostgreSQL commands directly from the database. You can also use [1-59] seconds to schedule a job based on an interval.'
);
"#,
    name = "init",
);

extension_sql_file!("../sql/meta.sql");

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    // #[pg_test]
    // fn test_hello_tembo() {
    //     assert_eq!("Hello, tembo", crate::hello_tembo());
    // }
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
