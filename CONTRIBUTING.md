# Contributing to `pg_vectorize`

Welcome to the `pg_vectorize` contribution guide! This comprehensive guide will help you set up your development environment, build the project from source, and start contributing effectively.

If you encounter any issues or have questions, feel free to join the [Tembo Community Slack](https://join.slack.com/t/tembocommunity/shared_invite/zt-2u3ctm86u-XzcyL76T7o~7Mpnt6KUx1g) for support.

## Prerequisites

Before you begin, ensure you have the following tools installed on your system:

- **Rust Toolchain**: Install [Rust](https://www.rust-lang.org/tools/install) to get `rustc`, `cargo`, and `rustfmt`. Make sure you have the latest stable version.

- **PGRX**: Install [PGRX](https://github.com/pgcentralfoundation/pgrx), the Rust framework for building PostgreSQL extensions.

- **Docker Engine**: Install [Docker Engine](https://docs.docker.com/engine/install/) to run local containers.

- **psql**: Install [psql](https://www.postgresql.org/docs/current/app-psql.html), the command-line interface to PostgreSQL.

**Note**: This guide assumes you are using PostgreSQL version 17. If you are using a different version, adjust the commands accordingly.

## Project Structure Overview

Understanding the layout of the `pg_vectorize` project will help you navigate the codebase and contribute effectively. Here's a brief overview:

### `extension` Directory

Contains the PostgreSQL extension code, focusing on database operations and vectorized data processing within PostgreSQL.

**Key Components**:

- **Job Management**: Utilities for creating and executing jobs, including background workers.
- **Configuration Settings**: GUC settings to customize the extension's behavior (e.g., API keys, batch sizes).
- **Triggers and Scheduling**: Defines triggers and cron jobs for automated data processing.
- **Embedding and Search Operations**: Integrates with external AI services to generate embeddings and enables vector search capabilities.

**When to Contribute Here**:

- Modifying database-specific logic or background job management.
- Working on PostgreSQL interactions with external embedding services.
- Enhancing in-database vector search functionalities.

### `core` Directory

Contains core logic and abstractions that support vectorization and embedding functionalities.

**Key Components**:

- **Providers**: API clients and wrappers for external AI/embedding services.
- **Data Structures**: Definitions of embeddings and related types.
- **Embedding Processing**: Logic for handling embedding requests and responses.

**When to Contribute Here**:

- Adding support for new AI/embedding providers.
- Modifying embedding generation logic.
- Implementing enhancements that are platform-independent.

## Setting Up Your Development Environment

### 1. Initialize PGRX

The `cargo pgrx init` command initializes the PGRX development environment by downloading and compiling the required PostgreSQL versions.

Run the following command:

```bash
cargo pgrx init
```

**MacOS Users**:

- If you're on MacOS, you might need to configure Cargo for dynamic linking due to Postgres symbols not being available until runtime.

  ```bash
  mkdir -p ~/.cargo
  echo '[target.'cfg(target_os="macos")']' >> ~/.cargo/config.toml
  echo 'rustflags = ["-Clink-arg=-Wl,-undefined,dynamic_lookup"]' >> ~/.cargo/config.toml
  ```

### 2. Clone the `pg_vectorize` Repository

Clone the repository and navigate into the project directory:

```bash
git clone https://github.com/tembo-io/pg_vectorize.git
cd pg_vectorize/extension
```

### 3. Set PostgreSQL Version

Export the PostgreSQL version you are using (default is 17):

```bash
export PG_VERSION=17
```

### 4. Install Dependencies and Configure PostgreSQL

Install the required PostgreSQL extensions and configure `postgresql.conf` by running:

```bash
make setup
```

**This command will**:

- Installs the necessary extensions: `pg_cron`, `pgvector`, `pgmq`, and `vectorscale`.
- Updates `postgresql.conf` with the required settings.

### 5. Build and Run `pg_vectorize`

Compile the project and start PostgreSQL with the `pg_vectorize` extension:

```bash
make run
```

This command will:

- Build the `pg_vectorize` extension.
- Start PostgreSQL and bring you into the `psql` shell.

### 6. Create the `vectorize` Extension

Inside the `psql` shell, create the `vectorize` extension:

```sql
CREATE EXTENSION vectorize cascade;
```

## Verifying the Installation

### 1. Check Installed Extensions

List the installed extensions to verify that `vectorize` is properly installed:

```sql
\dx
```

Expected output:

```text
                           List of installed extensions
   Name     | Version |   Schema   |               Description
------------+---------+------------+------------------------------------------
 pg_cron    | 1.6     | pg_catalog | Job scheduler for PostgreSQL
 pgmq       | 1.1.1   | pgmq       | A lightweight message queue.
 plpgsql    | 1.0     | pg_catalog | PL/pgSQL procedural language
 vector     | 0.6.0   | public     | Vector data type and access methods
 vectorize  | 0.10.1  | vectorize  | Simplest way to do vector search on PG
(5 rows)
```

### 2. Verify Configuration Settings

Ensure that `vectorize.embedding_service_url` is set correctly:

```sql
SHOW vectorize.embedding_service_url;
```

Expected output:

```text
 vectorize.embedding_service_url
---------------------------------
 http://localhost:3000/v1/embeddings
(1 row)
```

**Changing the URL**: If you need to update the `embedding_service_url`, run:

```sql
ALTER SYSTEM SET vectorize.embedding_service_url TO 'http://new-url:3000/v1/embeddings';
SELECT pg_reload_conf();
```

### 3. Load Example Data

Create the `products` table and insert example data:

```sql
CREATE TABLE products (LIKE vectorize.example_products INCLUDING ALL);
INSERT INTO products SELECT * FROM vectorize.example_products;
```

Verify the data insertion:

```sql
SELECT * FROM products LIMIT 2;
```

Expected output:

```text
 product_id | product_name |                      description                       |       last_updated_at
------------+--------------+--------------------------------------------------------+------------------------------
          1 | Pencil       | Utensil used for writing and often works best on paper | 2023-07-26 17:20:43.639351
          2 | Laptop Stand | Elevated platform for laptops, enhancing ergonomics    | 2023-07-26 17:20:43.639351
(2 rows)
```

### 4. Run Sample Queries

Create a vector search job:

```sql
SELECT vectorize.table(
  job_name     => 'product_search_hf',
  "table"      => 'products',
  primary_key  => 'product_id',
  columns      => ARRAY['product_name', 'description'],
  transformer  => 'sentence-transformers/multi-qa-MiniLM-L6-dot-v1'
);
```

Expected output:

```text
             table
-------------------------------
 Successfully created job: product_search_hf
(1 row)
```

Perform a vector search:

```sql
SELECT * FROM vectorize.search(
  job_name       => 'product_search_hf',
  query          => 'accessories for mobile devices',
  return_columns => ARRAY['product_id', 'product_name'],
  num_results    => 3
);
```

Expected output:

```text
               search_results
---------------------------------------------
 {"product_id":13,"product_name":"Phone Charger","similarity_score":0.8147812194590133}
 {"product_id":6,"product_name":"Backpack","similarity_score":0.774306211384604}
 {"product_id":11,"product_name":"Stylus Pen","similarity_score":0.7709903789778251}
(3 rows)
```

## Accessing the Tembo Embedding Service

When running the Tembo Embedding Service locally, you can view its endpoints in the Swagger UI at [http://localhost:3000/docs](http://localhost:3000/docs). This UI allows you to test and interact with available endpoints directly from your browser.

For the full **API documentation** for `pg_vectorize`, refer to the hosted version at [https://tembo.io/pg_vectorize/](https://tembo.io/pg_vectorize/), which includes function references, and more usage examples.

## Troubleshooting and Tips

- **Monitoring Docker Containers**: To view the `vector-serve` container logs in real-time:

  ```bash
  docker logs $(docker ps -q --filter ancestor=quay.io/tembo/vector-serve:latest) -f
  ```

- **Checking PostgreSQL Logs**: If you encounter issues, check the PostgreSQL logs:

  ```bash
  cat ~/.pgrx/${PG_VERSION}.log
  ```

## Releases

Releases for `pg_vectorize` are automated via a [GitHub workflow](https://github.com/tembo-io/pg_vectorize/blob/main/.github/workflows/extension_ci.yml). Compiled binaries are hosted at [pgt.dev](https://pgt.dev).

To create a new release:

1. Create a new tag following [Semantic Versioning](https://semver.org/).
2. Create a release with the same tag name.
3. Auto-generate release notes and add any additional details.
