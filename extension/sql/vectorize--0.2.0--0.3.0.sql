DROP FUNCTION vectorize."table";

CREATE  FUNCTION vectorize."table"(
        "table" TEXT, /* &str */
        "columns" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
        "job_name" TEXT, /* core::option::Option<alloc::string::String> */
        "args" json, /* pgrx::datum::json::Json */
        "primary_key" TEXT, /* alloc::string::String */
        "schema" TEXT DEFAULT 'public', /* alloc::string::String */
        "update_col" TEXT DEFAULT 'last_updated_at', /* alloc::string::String */
        "transformer" vectorize.Transformer DEFAULT 'openai', /* vectorize::types::Transformer */
        "index_dist_type" vectorize.IndexDist DEFAULT 'pgv_hnsw_cosine',
        "table_method" vectorize.TableMethod DEFAULT 'append', /* vectorize::init::TableMethod */
        "schedule" TEXT DEFAULT '* * * * *' /* alloc::string::String */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
LANGUAGE c /* Rust */

AS 'MODULE_PATHNAME', 'table_wrapper';

DROP FUNCTION vectorize."search";

CREATE  FUNCTION vectorize."search"(
        "job_name" TEXT, /* &str */
        "query" TEXT, /* &str */
        "api_key" TEXT, /* &str */
        "return_columns" TEXT[] DEFAULT ARRAY['*']::text[], /* alloc::vec::Vec<alloc::string::String> */
        "num_results" INT DEFAULT 10 /* i32 */
) RETURNS TABLE (
        "search_results" jsonb  /* pgrx::datum::json::JsonB */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_wrapper';