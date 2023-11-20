DROP function vectorize."table";

-- vectorize::api::table
CREATE  FUNCTION vectorize."table"(
        "table" TEXT, /* &str */
        "columns" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
        "job_name" TEXT, /* alloc::string::String */
        "primary_key" TEXT, /* alloc::string::String */
        "args" json DEFAULT '{}', /* pgrx::datum::json::Json */
        "schema" TEXT DEFAULT 'public', /* alloc::string::String */
        "update_col" TEXT DEFAULT 'last_updated_at', /* alloc::string::String */
        "transformer" vectorize.Transformer DEFAULT 'openai', /* vectorize::types::Transformer */
        "search_alg" vectorize.SimilarityAlg DEFAULT 'pgv_cosine_similarity', /* vectorize::types::SimilarityAlg */
        "table_method" vectorize.TableMethod DEFAULT 'append', /* vectorize::types::TableMethod */
        "schedule" TEXT DEFAULT '* * * * *' /* alloc::string::String */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'table_wrapper';

ALTER TABLE vectorize.vectorize_meta RENAME TO vectorize.job;
