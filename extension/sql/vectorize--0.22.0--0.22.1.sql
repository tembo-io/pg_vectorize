DROP FUNCTION IF EXISTS vectorize._handle_table_update(text, text[]);
CREATE OR REPLACE FUNCTION vectorize._handle_table_update(
    job_name text,
    record_ids text[]
) RETURNS void AS $$
DECLARE
    job_message jsonb;
BEGIN
    job_message = jsonb_build_object(
        'job_name', job_name,
        'record_ids', record_ids
    );
    
    -- Send the job message to the queue
    PERFORM pgmq.send('vectorize_jobs', job_message);
    
END;
$$ LANGUAGE plpgsql;
