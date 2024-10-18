-- changed default table method on table()
DROP FUNCTION vectorize."table";
-- src/api.rs:11
-- vectorize::api::table
CREATE  FUNCTION vectorize."table"(
        "table" TEXT, /* &str */
        "columns" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
        "job_name" TEXT, /* &str */
        "primary_key" TEXT, /* &str */
        "args" json DEFAULT '{}', /* pgrx::datum::json::Json */
        "schema" TEXT DEFAULT 'public', /* &str */
        "update_col" TEXT DEFAULT 'last_updated_at', /* alloc::string::String */
        "transformer" TEXT DEFAULT 'text-embedding-ada-002', /* &str */
        "index_dist_type" vectorize.IndexDist DEFAULT 'pgv_hnsw_cosine',
        "table_method" vectorize.TableMethod DEFAULT 'join', /* vectorize::types::TableMethod */
        "schedule" TEXT DEFAULT '* * * * *' /* &str */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'table_wrapper';

-- changed default table method on init_rag()
DROP FUNCTION vectorize."init_rag";
-- vectorize::api::init_rag
CREATE  FUNCTION vectorize."init_rag"(
        "agent_name" TEXT, /* &str */
        "table_name" TEXT, /* &str */
        "unique_record_id" TEXT, /* &str */
        "column" TEXT, /* &str */
        "schema" TEXT DEFAULT 'public', /* &str */
        "transformer" TEXT DEFAULT 'text-embedding-ada-002', /* &str */
        "index_dist_type" vectorize.IndexDist DEFAULT 'pgv_hnsw_cosine',
        "table_method" vectorize.TableMethod DEFAULT 'join', /* vectorize::types::TableMethod */
        "schedule" TEXT DEFAULT '* * * * *' /* &str */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'init_rag_wrapper';
