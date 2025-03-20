-- Rename 'table' key to 'relation' in the params JSONB column
UPDATE vectorize.job
SET params = jsonb_set(
    params - 'table',  -- Remove old 'table' key
    '{relation}', 
    params->'table',  -- Copy value to new 'relation' key
    true  -- Ensure key exists
)
WHERE params ? 'table';

-- Update the function to reference "relation" instead of "table"
CREATE OR REPLACE FUNCTION handle_table_drop()
RETURNS event_trigger AS $$
DECLARE
    obj RECORD;
    schema_name TEXT;
    relation_name TEXT;
BEGIN
    FOR obj IN SELECT * FROM pg_event_trigger_dropped_objects() LOOP
        IF obj.object_type = 'table' THEN
            schema_name := split_part(obj.object_identity, '.', 1);  
            relation_name := split_part(obj.object_identity, '.', 2);  
            
            -- Perform cleanup: delete the associated job from the vectorize.job table
            DELETE FROM vectorize.job
            WHERE params ->> 'relation' = relation_name
            AND params ->> 'schema' = schema_name;
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;