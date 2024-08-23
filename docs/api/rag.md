# RAG

SQL API for Retrieval Augmented Generation projects.

## Initializing a RAG table

Creates embeddings for specified data in a Postgres table. Creates index, and triggers to keep embeddings up to date.

### `vectorize.init_rag`

```sql
vectorize.init_rag(
    "agent_name" TEXT,
    "table_name" TEXT,
    "unique_record_id" TEXT,
    "column" TEXT,
    "schema" TEXT DEFAULT 'public',
    "transformer" TEXT DEFAULT 'tembo/meta-llama/Meta-Llama-3-8B-Instruct',
    "index_dist_type" vectorize.IndexDist DEFAULT 'pgv_hnsw_cosine',
    "table_method" vectorize.TableMethod DEFAULT 'append'
) RETURNS TEXT
```

**Parameters:**

| Parameter      | Type | Description     |
| :---        |    :----   |          :--- |
| agent_name | text | A unique name for the project. |
| table_name | text | The name of the table to be initialized. |
| unique_record_id | text | The name of the column that contains the unique record id. |
| column | text | The name of the column that contains the content that is used for context for RAG. |
| schema | text | The name of the schema where the table is located. Defaults to 'public'. |
| transformer | text | The name of the transformer to use for the embeddings. Defaults to 'text-embedding-ada-002'. |
| index_dist_type | IndexDist | The name of index type to build. Defaults to 'pgv_hnsw_cosine'. |
| table_method | TableMethod | The method to use for the table. Defaults to 'append', which adds a column to the existing table. |

Example:

```sql
select vectorize.init_rag(
    agent_name        => 'tembo_chat',
    table_name        => 'tembo_docs',
    unique_record_id  => 'document_name',
    "column"          => 'content',
    transformer       => 'sentence-transformers/all-MiniLM-L12-v2'
);
```

---

## Query using RAG

### `vectorize.rag`

```sql
vectorize."rag"(
    "agent_name" TEXT,
    "query" TEXT,
    "chat_model" TEXT DEFAULT 'openai/gpt-3.5-turbo',
    "task" TEXT DEFAULT 'question_answer',
    "api_key" TEXT DEFAULT NULL,
    "num_context" INT DEFAULT 2,
    "force_trim" bool DEFAULT false
) RETURNS TABLE (
    "chat_results" jsonb
)
```

**Parameters:**

| Parameter      | Type | Description     |
| :---        |    :----   |          :--- |
| agent_name | text | Specify the name provided during vectorize.init_rag |
| query | text | The user provided query or command provided to the chat completion model.  |
| task | text | Specifies the name of the prompt template to use. Must exist in vectorize.prompts (prompt_type) |
| api_key | text | API key for the specified chat model. If OpenAI, this value overrides the config `vectorize.openai_key` |
| num_context | int | The number of context documents returned by similarity search include in the message submitted to the chat completion model |
| force_trim | bool | Trims the documents provided as context, starting with the least relevant documents, such that the prompt fits into the model's context window. Defaults to false. |

### Example

```sql
select vectorize.rag(
    agent_name  => 'tembo_support',
    query       => 'what are the major features from the tembo kubernetes operator?',
    chat_model  => 'openai/gpt-3.5-turbo',
    force_trim  => 'true'
);
```

The response contains the contextual data used in the prompt in addition to the chat response.

```json
{
  "context": [
    {
      "content": "\"Tembo Standard Stack\\n\\nThe Tembo Standard Stack is a tuned Postgres instance balance for general purpose computing. You have full control over compute, configuration, and extension installation.\"",
      "token_ct": 37,
      "record_id": "535"
    },
    {
      "content": "\"Why Stacks?\\n\\nAdopting a new database adds significant complexity and costs to an engineering organization. Organizations spend a huge amount of time evaluating, benchmarking or migrating databases and setting upcomplicated pipelines keeping those databases in sync.\\n\\nMost of these use cases can be served by Postgres, thanks to its stability, feature completeness and extensibility. However, optimizing Postgres for each use case is a non-trivial task and requires domain expertise, use case understanding and deep Postgres expertise, making it hard for most developers to adopt this.\\n\\nTembo Stacks solve that problem by providing pre-built, use case optimized Postgres deployments.\\n\\nA tembo stack is a pre-built, use case specific Postgres deployment which enables you to quickly deploy specialized data services that can replace external, non-Postgres data services. They help you avoid the pains associated with adopting, operationalizing, optimizing and managing new databases.\\n\\n|Name|Replacement for|\\n|----|---------------|\\n|Data Warehouse| Snowflake, Bigquery |\\n|Geospatial| ESRI, Oracle |\\n|OLTP| Amazon RDS |\\n|OLAP| Snowflake, Bigquery |\\n|Machine Learning| MindsDB |\\n|Message Queue| Amazon SQS, RabbitMQ, Redis |\\n|Mongo Alternative on Postgres| MongoDB |\\n|RAG| LangChain |\\n|Standard| Amazon RDS |\\n|Vector DB| Pinecone, Weaviate |\\n\\nWe are actively working on additional Stacks. Check out the Tembo Roadmap and upvote the stacks you''d like to see next.\"",
      "token_ct": 336,
      "record_id": "387"
    }
  ],
  "chat_response": "Tembo Stacks are pre-built, use case specific Postgres deployments that are optimized for various data services such as Data Warehouse, Geospatial, OLTP, OLAP, Machine Learning, Message Queue, and more. These Stacks aim to provide organizations with specialized data services that can replace external non-Postgres data services. Each Tembo Stack is designed to cater to specific use cases, enabling developers to quickly deploy and utilize Postgres instances tailored to their needs without the complexity of setting up and optimizing Postgres manually."
}
```

Filter the results to just the `chat_response`:

```sql
select vectorize.rag(
    agent_name  => 'tembo_support',
    query       => 'what are the major features from the tembo kubernetes operator?',
    chat_model  => 'gpt-3.5-turbo',
    force_trim  => 'true'
) -> 'chat_response';
```

```text
 "Tembo Stacks are pre-built, use case specific Postgres deployments that are optimized for various data services such as Data Warehouse, Geospatial, OLTP, OLAP, Machine Learning, Message Queue, and more. These Stacks aim to provide organizations with specialized data services that can replace external non-Postgres data services. Each Tembo Stack is designed to cater to specific use cases, enabling developers to quickly deploy and utilize Postgres instances tailored to their needs without the complexity of setting up and optimizing Postgres manually."
```
