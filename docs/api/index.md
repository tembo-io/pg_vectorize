# PG Vectorize API Overview

pg vectorize provides tools for two closely related tasks; vector search and retrieval augmented generation (RAG), and there are APIs dedicated to both of these tasks. Vector search is an important component of RAG and the RAG APIs depend on the vector search APIs. It could be helpful to think of the vector search APIs as lower level than RAG. However, relative to Postgres's APIs, both of these vectorize APIs are very high level.

## Importing Pre-existing Embeddings

If you have already computed embeddings for your data using a compatible model, you can import these directly into pg_vectorize using the `vectorize.import_embeddings` function:

```sql
SELECT vectorize.import_embeddings(
    job_name => 'my_search_project',
    src_table => 'my_source_table',
    src_primary_key => 'id',
    src_embeddings_col => 'embeddings'
);
```

This function allows you to:
- Import pre-computed embeddings without recomputation
- Support both join and append table methods
- Automatically validate embedding dimensions
- Clean up any pending realtime jobs

The embeddings must match the dimensions expected by the model specified when creating the project with `vectorize.table()`.

### Parameters

- `job_name`: The name of your pg_vectorize project (created via `vectorize.table()`)
- `src_table`: The table containing your pre-computed embeddings
- `src_primary_key`: The primary key column in your source table
- `src_embeddings_col`: The column containing the vector embeddings

### Example

```sql
-- First create a vectorize project
SELECT vectorize.table(
    job_name => 'product_search',
    table => 'products',
    primary_key => 'id',
    columns => ARRAY['description'],
    transformer => 'sentence-transformers/all-MiniLM-L6-v2'
);

-- Then import pre-existing embeddings
SELECT vectorize.import_embeddings(
    job_name => 'product_search',
    src_table => 'product_embeddings',
    src_primary_key => 'product_id',
    src_embeddings_col => 'embedding_vector'
);
```
