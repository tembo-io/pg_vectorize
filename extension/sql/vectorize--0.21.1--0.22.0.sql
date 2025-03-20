
DROP FUNCTION IF EXISTS vectorize."rag";

CREATE  FUNCTION vectorize."rag"(
	"job_name" TEXT, /* &str */
	"query" TEXT, /* &str */
	"chat_model" TEXT DEFAULT 'openai/gpt-4o-mini', /* alloc::string::String */
	"task" TEXT DEFAULT 'question_answer', /* alloc::string::String */
	"api_key" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
	"num_context" INT DEFAULT 2, /* i32 */
	"force_trim" bool DEFAULT false /* bool */
) RETURNS TABLE (
	"chat_results" jsonb  /* pgrx::datum::json::JsonB */
)
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'rag_wrapper';

