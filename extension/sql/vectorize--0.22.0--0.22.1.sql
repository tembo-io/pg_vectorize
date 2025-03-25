DROP FUNCTION IF EXISTS vectorize._handle_table_update(text, text[]);
CREATE OR REPLACE FUNCTION vectorize._handle_table_update(
    job_name text,
    record_ids text[]
) RETURNS void AS $$
DECLARE
    project_meta record;
    job_message jsonb;
BEGIN
    -- Check if job metadata exists
    SELECT 
        job_id,
        name,
        index_dist_type,
        transformer,
        params
    INTO project_meta
    FROM vectorize.job
    WHERE name = job_name;
    
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to get project metadata';
    END IF;
    
    -- Create the job message
    job_message = jsonb_build_object(
        'job_name', job_name,
        'job_meta', to_jsonb(project_meta),
        'record_ids', record_ids
    );
    
    -- Send the job message to the queue
    PERFORM pgmq.send('vectorize_jobs', job_message);
    
END;
$$ LANGUAGE plpgsql;
