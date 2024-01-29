-- src/api.rs:15
-- vectorize::api::table
DROP FUNCTION IF EXISTS vectorize."table"(TEXT, TEXT[], TEXT, TEXT, json, TEXT, TEXT, TEXT, vectorize.SimilarityAlg, vectorize.TableMethod, TEXT);
CREATE  FUNCTION vectorize."table"(
        "table" TEXT, /* &str */
        "columns" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
        "job_name" TEXT, /* alloc::string::String */
        "primary_key" TEXT, /* alloc::string::String */
        "args" json DEFAULT '{}', /* pgrx::datum::json::Json */
        "schema" TEXT DEFAULT 'public', /* alloc::string::String */
        "update_col" TEXT DEFAULT 'last_updated_at', /* alloc::string::String */
        "transformer" TEXT DEFAULT 'text-embedding-ada-002', /* alloc::string::String */
        "search_alg" vectorize.SimilarityAlg DEFAULT 'pgv_cosine_similarity', /* vectorize::types::SimilarityAlg */
        "table_method" vectorize.TableMethod DEFAULT 'append', /* vectorize::types::TableMethod */
        "schedule" TEXT DEFAULT 'realtime' /* alloc::string::String */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'table_wrapper';