use pgrx::prelude::*;

mod api;
mod errors;
mod executor;
mod guc;
mod init;
mod query;
mod search;
mod transformers;
mod types;
mod util;
mod workers;

pgrx::pg_module_magic!();

extension_sql_file!("../sql/meta.sql");

// example dataset
extension_sql_file!("../sql/example.sql");

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
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
