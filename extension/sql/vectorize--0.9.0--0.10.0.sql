CREATE TABLE vectorize.prompts (
    prompt_type TEXT NOT NULL UNIQUE,
    sys_prompt TEXT NOT NULL,
    user_prompt TEXT NOT NULL
);

INSERT INTO vectorize.prompts (prompt_type, sys_prompt, user_prompt)
VALUES (
    'question_answer',
    'You are an expert Q&A system.\nYou must always answer the question using the provided context information. Never use any prior knowledge.\nAdditional rules to follow:\n1. Never directly reference the given context in your answer.\n2. Never use responses like ''Based on the context, ...'' or ''The context information ...'' or any responses similar to that.',
    'Context information is below.\n---------------------\n{{ context_str }}\n---------------------\nGiven the context information and not prior knowledge, answer the query.\n Query: {{ query_str }}\nAnswer: '
)
ON CONFLICT (prompt_type)
DO NOTHING;

-- src/api.rs:95
-- vectorize::api::rag
CREATE  FUNCTION vectorize."rag"(
        "agent_name" TEXT, /* &str */
        "query" TEXT, /* &str */
        "chat_model" TEXT DEFAULT 'gpt-3.5-turbo', /* alloc::string::String */
        "task" TEXT DEFAULT 'question_answer', /* alloc::string::String */
        "api_key" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
        "num_context" INT DEFAULT 2, /* i32 */
        "force_trim" bool DEFAULT false /* bool */
) RETURNS TABLE (
        "chat_results" jsonb  /* pgrx::datum::json::JsonB */
)
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'rag_wrapper';

-- src/api.rs:63
-- vectorize::api::init_rag
CREATE  FUNCTION vectorize."init_rag"(
        "agent_name" TEXT, /* &str */
        "table_name" TEXT, /* &str */
        "unique_record_id" TEXT, /* &str */
        "column" TEXT, /* &str */
        "schema" TEXT DEFAULT 'public', /* &str */
        "transformer" TEXT DEFAULT 'text-embedding-ada-002', /* &str */
        "search_alg" vectorize.SimilarityAlg DEFAULT 'pgv_cosine_similarity', /* vectorize::types::SimilarityAlg */
        "table_method" vectorize.TableMethod DEFAULT 'append' /* vectorize::types::TableMethod */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'init_rag_wrapper';
