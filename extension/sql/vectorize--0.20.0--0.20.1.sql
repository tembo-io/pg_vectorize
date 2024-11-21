DROP FUNCTION IF EXISTS vectorize."generate";

CREATE  FUNCTION vectorize."generate"(
	"input" TEXT,
	"model" TEXT DEFAULT 'tembo/meta-llama/Meta-Llama-3-8B-Instruct',
    "args" jsonb DEFAULT NULL
) RETURNS TEXT 
LANGUAGE c 
AS 'MODULE_PATHNAME', 'generate_wrapper';