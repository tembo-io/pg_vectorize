-- changed default table method on table()
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
        "search_alg" vectorize.SimilarityAlg DEFAULT 'pgv_cosine_similarity', /* vectorize::types::SimilarityAlg */
        "table_method" vectorize.TableMethod DEFAULT 'join', /* vectorize::types::TableMethod */
        "schedule" TEXT DEFAULT 'realtime' /* &str */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'table_wrapper';

-- 'realtime' on 'append' must be moved to 'join' method 
DO $$
DECLARE
    r RECORD;
    project TEXT;
    src_table TEXT;
    src_schema TEXT;
    src_pkey TEXT;
    src_pkey_type TEXT;
    src_embeddings_col TEXT;
    src_embeddings_updated_at TEXT;
    dest_table TEXT;
BEGIN
    FOR r IN SELECT * FROM vectorize.job LOOP
        src_table := r.params ->> 'table';
        src_schema := r.params ->> 'schema';
        src_pkey := r.params ->> 'pkey';
        src_pkey_type := r.params ->> 'pkey_type';
        src_embeddings_col = r.name || '_embeddings';
        src_embeddings_updated_at = r.name || '_updated_at';

        dest_table = 'vectorize._embeddings_' || r.name;
        -- if table has vectorize trigger, its a 'realtime' job
        IF EXISTS (
            SELECT 1
            FROM pg_trigger
            JOIN pg_class ON pg_class.oid = pg_trigger.tgrelid
            JOIN pg_namespace ON pg_namespace.oid = pg_class.relnamespace
            WHERE pg_class.relname = src_table
            AND pg_namespace.nspname = src_schema
            AND pg_trigger.tgname ILIKE '%vectorize%'
        ) THEN

            -- Construct the table name and column name dynamically
            EXECUTE 'CREATE TABLE ' || dest_table || ' ('
                || quote_ident(src_pkey) || ' ' || src_pkey_type || ',
                embeddings TEXT,
                updated_at TIMESTAMP,
            )';

            -- Copy data to the new table, adjust the column names as necessary
            EXECUTE 'INSERT INTO ' || dest_table || '(' || quote_ident(src_pkey) || ', embeddings, updated_at)
                    SELECT ' || quote_ident(src_pkey) || ', embeddings, updated_at FROM ' || quote_ident(src_schema) || '.' || quote_ident(src_table);
            -- Delete columns from the source table, adjust column names as necessary
            EXECUTE 'ALTER TABLE ' || quote_ident(r.table_name) ||
                ' DROP COLUMN ' || src_embeddings_col || ',
                DROP COLUMN ' || src_embeddings_updated_at || ';'
        END IF;
    END LOOP;
END $$;



SELECT *
FROM pg_trigger
JOIN pg_class ON pg_class.oid = pg_trigger.tgrelid
JOIN pg_namespace ON pg_namespace.oid = pg_class.relnamespace
WHERE pg_class.relname = 'products' -- Use the table name specified in r.params
AND pg_namespace.nspname = 'public' -- Use the schema name specified in r.params
AND pg_trigger.tgname ILIKE '%vectorize%'