-- transformer names are now all namespaced
UPDATE vectorize.job
SET transformer = CASE
    WHEN transformer = 'text-embedding-ada-002' THEN 'openai/text-embedding-ada-002'
    WHEN transformer LIKE 'sentence-transformers/%' THEN transformer
    ELSE 'sentence-transformers/' || transformer
END;

DROP FUNCTION vectorize."transform_embeddings";
-- src/api.rs:63
-- vectorize::api::transform_embeddings
CREATE  FUNCTION vectorize."transform_embeddings"(
	"input" TEXT, /* &str */
	"model_name" TEXT DEFAULT 'openai/text-embedding-ada-002', /* alloc::string::String */
	"api_key" TEXT DEFAULT NULL /* core::option::Option<alloc::string::String> */
) RETURNS double precision[] /* core::result::Result<alloc::vec::Vec<f64>, anyhow::Error> */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'transform_embeddings_wrapper';

DROP FUNCTION vectorize."rag";
-- src/api.rs:108
-- vectorize::api::rag
CREATE  FUNCTION vectorize."rag"(
	"agent_name" TEXT, /* &str */
	"query" TEXT, /* &str */
	"chat_model" TEXT DEFAULT 'openai/gpt-3.5-turbo', /* alloc::string::String */
	"task" TEXT DEFAULT 'question_answer', /* alloc::string::String */
	"api_key" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
	"num_context" INT DEFAULT 2, /* i32 */
	"force_trim" bool DEFAULT false /* bool */
) RETURNS TABLE (
	"chat_results" jsonb  /* pgrx::datum::json::JsonB */
)
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'rag_wrapper';

DROP FUNCTION vectorize."init_rag";
-- src/api.rs:74
-- vectorize::api::init_rag
CREATE  FUNCTION vectorize."init_rag"(
	"agent_name" TEXT, /* &str */
	"table_name" TEXT, /* &str */
	"unique_record_id" TEXT, /* &str */
	"column" TEXT, /* &str */
	"schema" TEXT DEFAULT 'public', /* &str */
  "index_dist_type" TEXT DEFAULT 'pgv_hsnw_cosine', /* vectorize::types::IndexDist */
	"transformer" TEXT DEFAULT 'openai/text-embedding-ada-002', /* &str */
	"search_alg" vectorize.SimilarityAlg DEFAULT 'pgv_cosine_similarity', /* vectorize::types::SimilarityAlg */
	"table_method" vectorize.TableMethod DEFAULT 'join', /* vectorize::types::TableMethod */
	"schedule" TEXT DEFAULT '* * * * *' /* &str */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'init_rag_wrapper';

DROP FUNCTION VECTORIZE."table";
-- src/api.rs:12
-- vectorize::api::table
CREATE  FUNCTION vectorize."table"(
	"table" TEXT, /* &str */
	"columns" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
	"job_name" TEXT, /* &str */
	"primary_key" TEXT, /* &str */
	"args" json DEFAULT '{}', /* pgrx::datum::json::Json */
	"schema" TEXT DEFAULT 'public', /* &str */
	"update_col" TEXT DEFAULT 'last_updated_at', /* alloc::string::String */
	"index_dist_type" TEXT DEFAULT 'pgv_hsnw_cosine', /* vectorize::types::IndexDist */
  "transformer" TEXT DEFAULT 'openai/text-embedding-ada-002', /* &str */
	"search_alg" vectorize.SimilarityAlg DEFAULT 'pgv_cosine_similarity', /* vectorize::types::SimilarityAlg */
	"table_method" vectorize.TableMethod DEFAULT 'join', /* vectorize::types::TableMethod */
	"schedule" TEXT DEFAULT '* * * * *' /* &str */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'table_wrapper';