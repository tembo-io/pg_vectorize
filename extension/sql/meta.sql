CREATE TABLE vectorize.job (
    job_id bigserial,
    name TEXT NOT NULL UNIQUE,
    index_dist_type TEXT NOT NULL DEFAULT 'pgv_hsnw_cosine',
    transformer TEXT NOT NULL,
    search_alg TEXT NOT NULL,
    params jsonb NOT NULL,
    last_completion TIMESTAMP WITH TIME ZONE
);

CREATE TABLE vectorize.prompts (
    prompt_type TEXT NOT NULL UNIQUE,
    sys_prompt TEXT NOT NULL,
    user_prompt TEXT NOT NULL
);

INSERT INTO vectorize.prompts (prompt_type, sys_prompt, user_prompt)
VALUES (
    'question_answer',
    'You are an expert Q&A system.\nYou must always answer the question using the provided context information. Never use any prior knowledge.\nAdditional rules to follow:\n1. Never directly reference the given context in your answer.\n2. Never use responses like ''Based on the context, ...'' or ''The context information ...'' or any responses similar to that.',
    'Context information is below.\n---------------------\n{{ context_str }}\n---------------------\nGiven the context information and not prior knowledge, answer the query.\n Query: {{ query_str }}\nAnswer: '
)
ON CONFLICT (prompt_type)
DO NOTHING;
