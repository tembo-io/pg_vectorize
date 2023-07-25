# pg_vectorize

*under development*

The simplest implementation of LLM-backed vector search on Postgres.

```sql
-- initialize an existing table

select vectorize.table(
    job_name => 'my_search_job',
    "schema" => 'public',
    "table" => 'products',
    primary_key => 'product_id',
    columns => ARRAY['description', 'keyword_tags'],
    args => '{"api_key": my-openai-key"}'
);
```

-- trigger the job
```sql
select vectorize.job_execute('my_search_job');
```


```sql
select vectorize.search('my_search_job', 'my_api_key', 'chips made without corn starch');
```
