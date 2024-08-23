DROP FUNCTION vectorize."transform_embeddings";
CREATE  FUNCTION vectorize."transform_embeddings"(
	"input" TEXT, /* &str */
	"model_name" TEXT DEFAULT 'sentence-transformers/all-MiniLM-L6-v2', /* alloc::string::String */
	"api_key" TEXT DEFAULT NULL /* core::option::Option<alloc::string::String> */
) RETURNS double precision[] /* core::result::Result<alloc::vec::Vec<f64>, anyhow::Error> */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'transform_embeddings_wrapper';


DROP FUNCTION vectorize."rag";
CREATE  FUNCTION vectorize."rag"(
	"agent_name" TEXT, /* &str */
	"query" TEXT, /* &str */
	"chat_model" TEXT DEFAULT 'tembo/meta-llama/Meta-Llama-3-8B-Instruct', /* alloc::string::String */
	"task" TEXT DEFAULT 'question_answer', /* alloc::string::String */
	"api_key" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
	"num_context" INT DEFAULT 2, /* i32 */
	"force_trim" bool DEFAULT false /* bool */
) RETURNS TABLE (
	"chat_results" jsonb  /* pgrx::datum::json::JsonB */
)
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'rag_wrapper';


DROP FUNCTION vectorize."generate";
CREATE  FUNCTION vectorize."generate"(
	"input" TEXT, /* &str */
	"model" TEXT DEFAULT 'tembo/meta-llama/Meta-Llama-3-8B-Instruct', /* alloc::string::String */
	"api_key" TEXT DEFAULT NULL /* core::option::Option<alloc::string::String> */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'generate_wrapper';


DROP FUNCTION vectorize."encode";
CREATE  FUNCTION vectorize."encode"(
	"input" TEXT, /* &str */
	"model" TEXT DEFAULT 'sentence-transformers/all-MiniLM-L6-v2', /* alloc::string::String */
	"api_key" TEXT DEFAULT NULL /* core::option::Option<alloc::string::String> */
) RETURNS double precision[] /* core::result::Result<alloc::vec::Vec<f64>, anyhow::Error> */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'encode_wrapper';


DROP FUNCTION vectorize."table";
CREATE  FUNCTION vectorize."table"(
	"table" TEXT, /* &str */
	"columns" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
	"job_name" TEXT, /* &str */
	"primary_key" TEXT, /* &str */
	"schema" TEXT DEFAULT 'public', /* &str */
	"update_col" TEXT DEFAULT 'last_updated_at', /* alloc::string::String */
	"index_dist_type" vectorize.IndexDist DEFAULT 'pgv_hnsw_cosine', /* vectorize::types::IndexDist */
	"transformer" TEXT DEFAULT 'sentence-transformers/all-MiniLM-L6-v2', /* &str */
	"search_alg" vectorize.SimilarityAlg DEFAULT 'pgv_cosine_similarity', /* vectorize::types::SimilarityAlg */
	"table_method" vectorize.TableMethod DEFAULT 'join', /* vectorize::types::TableMethod */
	"schedule" TEXT DEFAULT '* * * * *' /* &str */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'table_wrapper';


DROP FUNCTION vectorize."init_rag";
CREATE  FUNCTION vectorize."init_rag"(
	"agent_name" TEXT, /* &str */
	"table_name" TEXT, /* &str */
	"unique_record_id" TEXT, /* &str */
	"column" TEXT, /* &str */
	"schema" TEXT DEFAULT 'public', /* &str */
	"index_dist_type" vectorize.IndexDist DEFAULT 'pgv_hnsw_cosine', /* vectorize::types::IndexDist */
	"transformer" TEXT DEFAULT 'sentence-transformers/all-MiniLM-L6-v2', /* &str */
	"search_alg" vectorize.SimilarityAlg DEFAULT 'pgv_cosine_similarity', /* vectorize::types::SimilarityAlg */
	"table_method" vectorize.TableMethod DEFAULT 'join', /* vectorize::types::TableMethod */
	"schedule" TEXT DEFAULT '* * * * *' /* &str */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'init_rag_wrapper';