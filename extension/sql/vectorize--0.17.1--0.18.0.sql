
-- src/api.rs:14
-- vectorize::api::table
DROP FUNCTION vectorize."table";
CREATE  FUNCTION vectorize."table"(
        "table" TEXT, /* &str */
        "columns" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
        "job_name" TEXT, /* &str */
        "primary_key" TEXT, /* &str */
        "schema" TEXT DEFAULT 'public', /* &str */
        "update_col" TEXT DEFAULT 'last_updated_at', /* alloc::string::String */
        "index_dist_type" vectorize.IndexDist DEFAULT 'pgv_hnsw_cosine', /* vectorize::types::IndexDist */
        "transformer" TEXT DEFAULT 'openai/text-embedding-ada-002', /* &str */
        "search_alg" vectorize.SimilarityAlg DEFAULT 'pgv_cosine_similarity', /* vectorize::types::SimilarityAlg */
        "table_method" vectorize.TableMethod DEFAULT 'join', /* vectorize::types::TableMethod */
        "schedule" TEXT DEFAULT '* * * * *' /* &str */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'table_wrapper';