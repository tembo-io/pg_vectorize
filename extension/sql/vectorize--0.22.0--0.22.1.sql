--- called by the trigger function when a table is updated
--- handles enqueueing the embedding transform jobs
CREATE OR REPLACE FUNCTION vectorize._handle_table_update(
    job_name text,
    record_ids text[]
) RETURNS void AS $$
DECLARE
    batch_size integer;
    batch_result RECORD;
    job_messages jsonb[] := '{}';
BEGIN
    -- create jobs of size batch_size
    batch_size := current_setting('vectorize.batch_size')::integer;
    FOR batch_result IN SELECT batches FROM vectorize.batch_texts(record_ids, batch_size) LOOP
        job_messages := array_append(
            job_messages,
            jsonb_build_object(
                'job_name', job_name,
                'record_ids', batch_result.batches
            )
        );
    END LOOP;

    PERFORM pgmq.send_batch(
        queue_name=>'vectorize_jobs'::text,
        msgs=>job_messages::jsonb[])
    ;

END;
$$ LANGUAGE plpgsql;
