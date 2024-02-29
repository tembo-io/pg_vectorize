# Utilities

## Text to Embeddings

Transforms a block of text to embeddings using the specified transformer.

Requires the `vector-serve` container to be set via `vectorize.embedding_svc_url`, or an OpenAI key to be set if using OpenAI embedding models.

```sql
vectorize."transform_embeddings"(
    "input" TEXT,
    "model_name" TEXT DEFAULT 'text-embedding-ada-002',
    "api_key" TEXT DEFAULT NULL
) RETURNS double precision[]
```

**Parameters:**

| Parameter      | Type | Description     |
| :---        |    :----   |          :--- |
| input | text | Raw text to be transformed to an embedding |
| model_name | text | Name of the sentence-transformer or OpenAI model to use.  |
| api_key | text | API key for the transformer. Defaults to NULL. |

### Example

```sql
select vectorize.transform_embeddings(
    input => 'the quick brown fox jumped over the lazy dogs',
    model_name => 'sentence-transformers/multi-qa-MiniLM-L6-dot-v1'
);

{-0.2556323707103729,-0.3213586211204529 ..., -0.0951206386089325}
```
