<h1 align="center">
 <b>pg_vectorize: a VectorDB for Postgres</b>
<br>

<br/>
  <a href="https://tembo.io"><img src="https://github.com/tembo-io/pg_vectorize/assets/15756360/34d65cba-065b-485f-84a4-76284e9def19" alt="pg_vectorize" width="368px"></a>

<p align="center">
  <div style="text-align: center;">
    <a href="https://cloud.tembo.io/sign-up">
      <img src="https://tembo.io/tryFreeButton.svg" alt="Tembo Cloud Try Free">
    </a>
  </div>
</p>

</h1>

A Postgres extension that automates the transformation and orchestration of text to embeddings and provides hooks into the most popular LLMs. This allows you to perform vector search and build LLM applications on existing data with as little as two function calls.

This project relies heavily on the work by [pgvector](https://github.com/pgvector/pgvector) for vector similarity search, [pgmq](https://github.com/tembo-io/pgmq) for orchestration in background workers, and [SentenceTransformers](https://huggingface.co/sentence-transformers).

---

[![Static Badge](https://img.shields.io/badge/%40tembo-community?logo=slack&label=slack)](https://join.slack.com/t/tembocommunity/shared_invite/zt-277pu7chi-NHtvHWvLhHwyK0Y5Y6vTPw)
[![PGXN version](https://badge.fury.io/pg/vectorize.svg)](https://pgxn.org/dist/vectorize/)
[![OSSRank](https://shields.io/endpoint?url=https://ossrank.com/shield/3815)](https://ossrank.com/p/3815)

`pg_vectorize` powers the [VectorDB Stack](https://tembo.io/docs/product/stacks/ai/vectordb) on [Tembo Cloud](https://cloud.tembo.io/) and is available in all hobby tier instances.

**Source**: [https://github.com/tembo-io/pg_vectorize](https://github.com/tembo-io/pg_vectorize)

## Features

- Workflows for both vector search and RAG (Retrieval-Augmented Generation)
- Integrations with OpenAI's [embeddings](https://platform.openai.com/docs/guides/embeddings) and [text generation](https://platform.openai.com/docs/guides/text-generation) endpoints, and a self-hosted container for running [Hugging Face Sentence-Transformers](https://huggingface.co/sentence-transformers)
- Automated creation of Postgres triggers to keep your embeddings up to date
- High-level API: one function to initialize embeddings transformations, and another function to search

## Table of Contents

- [Installation](#installation)
  - Option 1: Local Setup with Docker
  - Option 2: Using Online Embeddings (e.g., OpenAI)
- [Vector Search Example](#vector-search-example)
- [RAG Example](#rag-example)
- [Updating Embeddings](#updating-embeddings)
- [Direct Interaction with LLMs](#direct-interaction-with-llms)

## Installation

To get started with `pg_vectorize`, you have two main options depending on whether you want to use local embeddings with Docker or online embeddings like OpenAI.

### Common Setup Steps (For Both Local and OpenAI Setups)

1. **Install `pg_vectorize` in Your Postgres Instance**

   If you are installing in an existing Postgres instance, ensure you have the necessary dependencies:

   - Rust and [pgrx toolchain](https://github.com/pgcentralfoundation/pgrx)
   - Postgres extensions:
     - [pg_cron](https://github.com/citusdata/pg_cron) ^1.5
     - [pgmq](https://github.com/tembo-io/pgmq) ^1
     - [pgvector](https://github.com/pgvector/pgvector) ^0.5.0

   Install `pg_vectorize`:

   ```sql
   CREATE EXTENSION vectorize CASCADE;
   ```

2. **Configure Postgres**

   Update your `postgresql.conf` or set configuration parameters:

   ```sql
   -- Requires restart of Postgres
   ALTER SYSTEM SET shared_preload_libraries = 'vectorize,pg_cron';
   ALTER SYSTEM SET cron.database_name = 'postgres';
   ```

### Option 1: Using Online Embeddings (e.g., OpenAI)

If you prefer to use online embeddings services like OpenAI, you can set up `pg_vectorize` to use these services without running a local model server.

1. **Set Your API Key**

   Set your OpenAI API key:

   ```sql
   ALTER SYSTEM SET vectorize.openai_key = '<your_openai_api_key>';
   SELECT pg_reload_conf();
   ```

### Option 2: Local Setup with Docker

If you want to run everything locally, including the model server, you can use Docker to set up `pg_vectorize`.

1. **Start the Tembo Docker Container and Vector Server**

   Create a `docker-compose.yml` file with the necessary services or use the one provided in the repository.

   ```bash
   docker compose up -d
   ```

2. **Connect to Postgres**

   ```bash
   docker compose exec postgres psql
   ```

3. **Set the Embedding Service URL**

   If you're running the vector-serve container, set the following URL as a configuration parameter in Postgres. The host may need to change from `localhost` to something else depending on where you are running the container.

   ```sql
   ALTER SYSTEM SET vectorize.embedding_service_url = 'http://localhost:3000/v1/embeddings';
   SELECT pg_reload_conf();
   ```

## Vector Search Example

### Using Local Embeddings

If you're using the local setup with Docker and have the model server running, you can use the following example with Hugging Face's Sentence-Transformers.

1. **Setup the Products Table**

   ```sql
   CREATE TABLE products (LIKE vectorize.example_products INCLUDING ALL);
   INSERT INTO products SELECT * FROM vectorize.example_products;
   ```

2. **Create a Vectorization Job**

   ```sql
   SELECT vectorize.table(
       job_name    => 'product_search_hf',
       "table"     => 'products',
       primary_key => 'product_id',
       columns     => ARRAY['product_name', 'description'],
       transformer => 'sentence-transformers/all-MiniLM-L6-v2',
       schedule    => 'realtime'
   );
   ```

3. **Search**

   ```sql
   SELECT * FROM vectorize.search(
       job_name        => 'product_search_hf',
       query           => 'accessories for mobile devices',
       return_columns  => ARRAY['product_id', 'product_name'],
       num_results     => 3
   );
   ```

### Using OpenAI Embeddings

If you're using online embeddings (e.g., OpenAI), adjust the transformer parameter to use an OpenAI model.

1. **Create a Vectorization Job with OpenAI**

   ```sql
   SELECT vectorize.table(
       job_name    => 'product_search_openai',
       "table"     => 'products',
       primary_key => 'product_id',
       columns     => ARRAY['product_name', 'description'],
       transformer => 'openai/text-embedding-ada-002',
       schedule    => 'realtime'
   );
   ```

2. **Search**

   ```sql
   SELECT * FROM vectorize.search(
       job_name        => 'product_search_openai',
       query           => 'accessories for mobile devices',
       return_columns  => ARRAY['product_id', 'product_name'],
       num_results     => 3
   );
   ```

## RAG Example

### Using Local Models

1. **Set Up the Products Table**

   If not already done:

   ```sql
   CREATE TABLE products (LIKE vectorize.example_products INCLUDING ALL);
   INSERT INTO products SELECT * FROM vectorize.example_products;
   ```

2. **Add a Context Column**

   ```sql
   ALTER TABLE products
   ADD COLUMN context TEXT GENERATED ALWAYS AS (product_name || ': ' || description) STORED;
   ```

3. **Initialize RAG**

   ```sql
   SELECT vectorize.init_rag(
       agent_name          => 'product_chat',
       table_name          => 'products',
       "column"            => 'context',
       unique_record_id    => 'product_id',
       transformer         => 'sentence-transformers/all-MiniLM-L6-v2'
   );
   ```

### Using OpenAI Models

1. **Initialize RAG with OpenAI**

   ```sql
   SELECT vectorize.init_rag(
       agent_name          => 'product_chat_openai',
       table_name          => 'products',
       "column"            => 'context',
       unique_record_id    => 'product_id',
       transformer         => 'openai/text-embedding-ada-002'
   );
   ```

2. **Ask a Question**

   ```sql
   SELECT vectorize.rag(
       agent_name  => 'product_chat_openai',
       query       => 'What is a pencil?',
       chat_model  => 'openai/gpt-3.5-turbo'
   ) -> 'chat_response';
   ```

## Updating Embeddings

When the source text data is updated, the embeddings can be updated automatically based on the `schedule` parameter.

- **Realtime Updates**: Use `schedule => 'realtime'` to create triggers that update embeddings immediately upon data changes.

- **Cron Schedule**: Use a cron-like schedule (e.g., `schedule => '* * * * *'`) to check for changes at regular intervals.

Example:

```sql

INSERT INTO products (product_id, product_name, description, product_category, price)
VALUES (12345, 'pizza', 'dish of Italian origin consisting of a flattened disk of bread', 'food', 5.99);

UPDATE products
SET description = 'Sling made of fabric, rope, or netting, suspended between two or more points, used for swinging, sleeping, or resting'
WHERE product_name = 'Hammock';
```

## Direct Interaction with LLMs

You can directly call various LLM providers using SQL functions provided by `pg_vectorize`.

**Text Generation Example:**

```sql
SELECT vectorize.generate(
  input => 'Tell me the difference between a cat and a dog in one sentence',
  model => 'openai/gpt-4'
);
```

This will produce the following output:

```text
Cats are generally more independent and solitary, while dogs tend to be more social and loyal companions.
```

**Embedding Generation Example:**

```sql
SELECT vectorize.encode(
  input => 'Tell me the difference between a cat and a dog in one sentence',
  model => 'openai/text-embedding-ada-002'
);
```

This will produce the following output (an array of numerical values representing the embedding):

```text
{0.0028769304, -0.005826319, -0.0035932811, ...}
```

## Contributing

We welcome contributions from the community! If you're interested in contributing to `pg_vectorize`, please check out our [Contributing Guide](CONTRIBUTING.md). Your contributions help make this project better for everyone.

## Community Support

If you encounter any issues or have any questions, feel free to join our [Tembo Community Slack](https://join.slack.com/t/tembocommunity/shared_invite/zt-2u3ctm86u-XzcyL76T7o~7Mpnt6KUx1g). We're here to help!
