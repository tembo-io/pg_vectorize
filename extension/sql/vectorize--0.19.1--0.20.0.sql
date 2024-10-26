ALTER TABLE vectorize.job DROP COLUMN search_alg;

DROP FUNCTION IF EXISTS vectorize."table";
CREATE FUNCTION vectorize."table"(
    "table" TEXT,
    "columns" TEXT[],
    "job_name" TEXT,
    "primary_key" TEXT,
    "schema" TEXT DEFAULT 'public',
    "update_col" TEXT DEFAULT 'last_updated_at',
    "index_dist_type" vectorize.IndexDist DEFAULT 'pgv_hnsw_cosine',
    "transformer" TEXT DEFAULT 'sentence-transformers/all-MiniLM-L6-v2',
    "table_method" vectorize.TableMethod DEFAULT 'join',
    "schedule" TEXT DEFAULT '* * * * *'
) RETURNS TEXT
STRICT
LANGUAGE c
AS 'MODULE_PATHNAME', 'table_wrapper';

DROP FUNCTION IF EXISTS vectorize."init_rag";
CREATE FUNCTION vectorize."init_rag"(
    "agent_name" TEXT,
    "table_name" TEXT,
    "unique_record_id" TEXT,
    "column" TEXT,
    "schema" TEXT DEFAULT 'public',
    "index_dist_type" vectorize.IndexDist DEFAULT 'pgv_hnsw_cosine',
    "transformer" TEXT DEFAULT 'sentence-transformers/all-MiniLM-L6-v2',
    "table_method" vectorize.TableMethod DEFAULT 'join',
    "schedule" TEXT DEFAULT '* * * * *'
) RETURNS TEXT
STRICT
LANGUAGE c
AS 'MODULE_PATHNAME', 'init_rag_wrapper';
