-- Rename 'table' key to 'relation' in the params JSONB column
UPDATE vectorize.job
SET params = jsonb_set(
    params - 'table',  -- Remove old 'table' key
    '{relation}', 
    params->'table',  -- Copy value to new 'relation' key
    true  -- Ensure key exists
)
WHERE params ? 'table';
