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
        "search_alg" vectorize.SimilarityAlg DEFAULT 'pgv_cosine_similarity', /* vectorize::types::SimilarityAlg */
        "table_method" vectorize.TableMethod DEFAULT 'join', /* vectorize::types::TableMethod */
        "schedule" TEXT DEFAULT 'realtime' /* &str */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'table_wrapper';

-- all 'realtime' jobs must be moved to the 'join' method
-- this moves the embeddings from the source table to a dedicated table
-- this provides far more efficient insert/update performance
DO $$
DECLARE
    r RECORD;
    project TEXT;
    src_table TEXT;
    src_schema TEXT;
    src_pkey TEXT;
    src_pkey_type TEXT;
    src_text_cols TEXT[];
    src_embeddings_col TEXT;
    src_embeddings_updated_at TEXT;
    src_embeddings_dtype TEXT;
    dest_table TEXT;
    create_query TEXT;
    insert_query TEXT;
    drop_col_query TEXT;
    alter_job_query TEXT;
    
    trigger_handler TEXT;
    trigger_update TEXT;
    trigger_insert TEXT;
BEGIN
    FOR r IN SELECT * FROM vectorize.job LOOP
        src_table := r.params ->> 'table';
        src_schema := r.params ->> 'schema';
        src_pkey := r.params ->> 'primary_key';
        src_pkey_type := r.params ->> 'pkey_type';
        src_embeddings_col := r.name || '_embeddings';
        src_embeddings_updated_at := r.name || '_updated_at';
        dest_table := '_embeddings_' || r.name;
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
            -- get the data type of the embeddings column
            EXECUTE format(
                'SELECT pg_catalog.format_type(a.atttypid, a.atttypmod) AS data_type
                    FROM pg_catalog.pg_class as cls
                    JOIN pg_catalog.pg_attribute as a ON a.attrelid = cls.oid
                    JOIN pg_catalog.pg_type as t ON a.atttypid = t.oid
                    JOIN pg_catalog.pg_namespace n ON n.oid = cls.relnamespace
                    WHERE cls.relname = %L
                    AND a.attname = %L
                    AND a.attnum > 0
                    AND NOT a.attisdropped
                    AND n.nspname = %L;
                    ',
                src_table, src_embeddings_col, src_schema
            ) INTO src_embeddings_dtype;

            create_query := format(
                'CREATE TABLE vectorize.%I ( %I %s UNIQUE, embeddings %s, updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL )',
                dest_table, src_pkey, src_pkey_type, src_embeddings_dtype
            );
            EXECUTE create_query;

            insert_query := format(
                'INSERT INTO vectorize.%I ( %I, embeddings, updated_at )
                 SELECT %I, %I, %I
                 FROM %s',
                 dest_table, src_pkey, src_pkey, src_embeddings_col, src_embeddings_updated_at, src_schema || '.' || src_table
            );
            EXECUTE insert_query;

            alter_job_query := format(
                'UPDATE vectorize.job SET params = jsonb_set(params, ''{table_method}'', ''"join"'') WHERE name = ''%s''', r.name
            );
            EXECUTE alter_job_query;

            drop_col_query = format(
                'ALTER TABLE %I.%I DROP COLUMN %I, DROP COLUMN %I',
                src_schema, src_table, src_embeddings_col, src_embeddings_updated_at
            );
            EXECUTE drop_col_query;

            src_text_cols := ARRAY(SELECT jsonb_array_elements_text(r.params -> 'columns'));
            -- drop the triggers, then re-init to create new ones
            
            EXECUTE format('ALTER EXTENSION vectorize ADD FUNCTION vectorize.handle_update_%s();', r.name);
            EXECUTE format('DROP FUNCTION vectorize.handle_update_%I CASCADE', r.name);

            PERFORM vectorize.table(
                    job_name => r.name,
                    "table" => src_table,
                    "schema" => src_schema,
                    primary_key => src_pkey,
                    columns => src_text_cols,
                    transformer => r.transformer,
                    table_method => 'join',
                    schedule => 'realtime'
                );            
        END IF;
    END LOOP;
END $$;
