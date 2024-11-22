DROP FUNCTION IF EXISTS vectorize."search";
DROP FUNCTION IF EXISTS vectorize."transform_embeddings";
DROP FUNCTION IF EXISTS vectorize."encode";
DROP FUNCTION IF EXISTS vectorize."rag";
DROP FUNCTION IF EXISTS vectorize."generate";

CREATE FUNCTION vectorize."search"(
    job_name TEXT,
    query TEXT,
    api_key TEXT DEFAULT NULL,
    return_columns TEXT[] DEFAULT ARRAY['*'],
    num_results INTEGER DEFAULT 10,
    where_sql TEXT DEFAULT NULL,
    args JSONB DEFAULT NULL
) RETURNS TABLE(search_results JSONB)
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_wrapper';

CREATE FUNCTION vectorize."transform_embeddings"(
    input TEXT,
    model_name TEXT DEFAULT 'sentence-transformers/all-MiniLM-L6-v2',
    api_key TEXT DEFAULT NULL,
    args JSONB DEFAULT NULL
) RETURNS FLOAT8[]
LANGUAGE c
AS 'MODULE_PATHNAME', 'transform_embeddings_wrapper';

CREATE FUNCTION vectorize."encode"(
    input TEXT,
    model TEXT DEFAULT 'sentence-transformers/all-MiniLM-L6-v2',
    api_key TEXT DEFAULT NULL,
    args JSONB DEFAULT NULL
) RETURNS FLOAT8[]
LANGUAGE c
AS 'MODULE_PATHNAME', 'encode_wrapper';

CREATE FUNCTION vectorize."rag"(
    agent_name TEXT,
    query TEXT,
    chat_model TEXT DEFAULT 'tembo/meta-llama/Meta-Llama-3-8B-Instruct',
    task TEXT DEFAULT 'question_answer',
    api_key TEXT DEFAULT NULL,
    num_context INTEGER DEFAULT 2,
    force_trim BOOLEAN DEFAULT FALSE,
    args JSONB DEFAULT NULL
) RETURNS TABLE(chat_results JSONB)
LANGUAGE c
AS 'MODULE_PATHNAME', 'rag_wrapper';


CREATE  FUNCTION vectorize."generate"(
	"input" TEXT,
	"model" TEXT DEFAULT 'tembo/meta-llama/Meta-Llama-3-8B-Instruct',
    "args" jsonb DEFAULT NULL
) RETURNS TEXT 
LANGUAGE c 
AS 'MODULE_PATHNAME', 'generate_wrapper';