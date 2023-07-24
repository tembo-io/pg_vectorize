# pg_vectorize

The simplest implementation of LLM-backed vector search on Postgres.

```sql
-- initialize an existing table
select vectorize.init_table('public', 'products', 'product_id', ARRAY['keyword_tags','summary'], 'openai', 'pgv_cosine_similarity', 'my-vector-job', 'my_api_key', 'last_updated_at');
```


```sql
select vectorize.search('my-vector-job', 'my_api_key', 'chips made without corn starch');
```
