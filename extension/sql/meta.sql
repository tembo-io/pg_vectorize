CREATE TABLE vectorize.job (
    job_id bigserial,
    name TEXT NOT NULL UNIQUE,
    index_dist_type TEXT NOT NULL DEFAULT 'pgv_hsnw_cosine',
    transformer TEXT NOT NULL,
    params jsonb NOT NULL,
    last_completion TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_job_params_table_schema
ON vectorize.job ((params ->> 'table'), (params ->> 'schema'));

-- create an event trigger function to delete jobs when corresponding tables are dropped
CREATE OR REPLACE FUNCTION after_drop_trigger()
RETURNS event_trigger AS $$
DECLARE
    dropped_object RECORD;
    schema_name TEXT;
    table_name TEXT;
BEGIN
    -- Using pg_event_trigger_dropped_objects() to fetch details of dropped objects.
    -- This function provides metadata for objects affected by the DROP event.
    FOR dropped_object IN SELECT * FROM pg_event_trigger_dropped_objects() LOOP
        IF dropped_object.object_type = 'table' THEN
            schema_name := split_part(dropped_object.object_identity, '.', 1);
            table_name := split_part(dropped_object.object_identity, '.', 2);
            RAISE NOTICE 'Processing drop for table: %, schema: %', table_name, schema_name;

        -- Delete jobs associated with the dropped table
            DELETE FROM vectorize.job
            WHERE params ? 'table' AND params ? 'schema'
              AND params ->> 'table' = table_name
              AND params ->> 'schema' = schema_name;
            RAISE NOTICE 'Job deletion executed for table: %, schema: %', table_name, schema_name;
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

DROP EVENT TRIGGER IF EXISTS vectorize_job_drop_trigger;

-- create the event trigger for DROP TABLE events
CREATE EVENT TRIGGER vectorize_job_drop_trigger
ON sql_drop
WHEN TAG IN ('DROP TABLE')
EXECUTE FUNCTION after_drop_trigger();

CREATE TABLE vectorize.prompts (
    prompt_type TEXT NOT NULL UNIQUE,
    sys_prompt TEXT NOT NULL,
    user_prompt TEXT NOT NULL
);

-- allow pg_monitor to read from vectorize schema
GRANT USAGE ON SCHEMA vectorize TO pg_monitor;
GRANT SELECT ON ALL TABLES IN SCHEMA vectorize TO pg_monitor;
GRANT SELECT ON ALL SEQUENCES IN SCHEMA vectorize TO pg_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA vectorize GRANT SELECT ON TABLES TO pg_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA vectorize GRANT SELECT ON SEQUENCES TO pg_monitor;


INSERT INTO vectorize.prompts (prompt_type, sys_prompt, user_prompt)
VALUES (
    'question_answer',
    'You are an expert Q&A system.\nYou must always answer the question using the provided context information. Never use any prior knowledge.\nAdditional rules to follow:\n1. Never directly reference the given context in your answer.\n2. Never use responses like ''Based on the context, ...'' or ''The context information ...'' or any responses similar to that.',
    'Context information is below.\n---------------------\n{{ context_str }}\n---------------------\nGiven the context information and not prior knowledge, answer the query.\n Query: {{ query_str }}\nAnswer: '
)
ON CONFLICT (prompt_type)
DO NOTHING;
