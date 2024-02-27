# Contributing to pg_vectorize

## Prerequisites

- [Rust](https://www.rust-lang.org/learn/get-started) - Toolchain including `rustc`, `cargo`, and `rustfmt`
- [PGRX](https://github.com/pgcentralfoundation/pgrx) - Rust-based PostgreSQL extension development framework
- [Docker Engine](https://docs.docker.com/engine/install/) - For running local containers
- [psql](https://www.postgresql.org/docs/current/app-psql.html) - Terminal-based front-end to PostgreSQL
- [pgmq](https://github.com/tembo-io/pgmq) - PostgreSQL extension for message queues
- [pg_cron](https://github.com/citusdata/pg_cron) - PostgreSQL extension for cron-based job scheduling

## Building from source

This process is more involved, but can easily be distilled down into a handful of steps.

### 1. Set up pgrx

```bash
cargo pgrx init
```

### 2. Set up Docker container

```bash
docker run -d -p 3000:3000 quay.io/tembo/vector-serve:latest
```

Confirm a successful set up by running the following:

```bash
docker ps
```

:wrench: Note: Consider running the following to see the container logs real time:

```bash
docker logs <your-container-id> -f
```

### 3. Clone and compile `pg_vectorize` and extension dependencies

When progressing through these steps, refer to the following for troubleshooting:

```bash
cat ~/.pgrx/15.log
```

#### 3.1. Apply configurations

Prior to compiling and running `pg_vector`, it's essential to update the `postgresql.conf` file.
`pgrx` uses a Postgres version-specific data directory, each containing its own `postgresql.conf` file.
The following example, utilizes Postgres version 15.
If you're using a different version, please alter the file path value `data-<postgres-version>` and run the following:

```bash
<your-editor> ~/.pgrx/data-15/postgresql.conf
```

Within this document, add the following:

```text
shared_preload_libraries = 'pg_cron, vectorize'
cron.database_name = 'postgres'
vectorize.embedding_service_url = 'http://vector-serve:3000/v1/embeddings'
```

:wrench: Note: If your machine is running a MacOS, you may need to apply the following configurations to Cargo's config file:

```
<your-editor> ~/.cargo/config
```

```text
[target.'cfg(target_os="macos")']
# Postgres symbols won't be available until runtime
rustflags = ["-Clink-arg=-Wl,-undefined,dynamic_lookup"]
```

#### 3.2. Clone and enter directory

```bash
git clone https://github.com/tembo-io/pg_vectorize.git

cd pg_vectorize
```

#### 3.3. Install dependencies

From within the pg_vectorize directory, run the following, which will install `pg_cron`, `pgmq`, and `pg_vector`:

```bash
make setup
```

Or you can run the commands individually:
```bash
make install-pg_cron
```
```bash
make install-pgmq
```
```bash
make install-pgvector
```

#### 3.4. Compile and run `pg_vector`

```bash
make run
```

### 4. Confirm successful build

#### 4.1. Check extension presence

Once the above command is run, you will be brought into Postgres via `psql`.

To list out the enabled extensions, run:

```sql
\dx
```
```text
                                      List of installed extensions
    Name    | Version |   Schema   |                             Description
------------+---------+------------+---------------------------------------------------------------------
 pg_cron    | 1.6     | pg_catalog | Job scheduler for PostgreSQL
 pg_partman | 4.7.3   | public     | Extension to manage partitioned tables by time or ID
 pgmq       | 1.1.1   | pgmq       | A lightweight message queue. Like AWS SQS and RSMQ but on Postgres.
 plpgsql    | 1.0     | pg_catalog | PL/pgSQL procedural language
 vector     | 0.6.0   | public     | vector data type and ivfflat and hnsw access methods
 vectorize  | 0.10.1  | vectorize  | The simplest way to do vector search on Postgres
(6 rows)
```

#### 4.2

```sql
SHOW vectorize.embedding_service_url;
```
```text
   vectorize.embedding_service_url
-------------------------------------
 http://vector-serve:3000/v1/embeddings
(1 row)
```

We have to use local host
```
ALTER SYSTEM SET vectorize.embedding_service_url TO 'http://localhost:3000/v1/embeddings';
```

Upon making this change, run:

```sql
SELECT pg_reload_conf();
```

Running the earlier SHOW command should reveal the appropriate change:

```sql
SHOW vectorize.embedding_service_url;
```
```text
   vectorize.embedding_service_url
-------------------------------------
 http://localhost:3000/v1/embeddings
(1 row)
```

#### 4.3. Load example data

The following can be found within the this project's README, under [Hugging Face Example](https://github.com/tembo-io/pg_vectorize/blob/main/README.md#hugging-face-example).

Begin by creating a `producs` table with the dataset that comes included with `pg_vectorize`.

```sql
CREATE TABLE products AS
SELECT * FROM vectorize.example_products;
```

You can then confirm everything is correct by running the following:

```sql
SELECT * FROM products limit 2;
```
```text
 product_id | product_name |                      description                       |        last_updated_at        
------------+--------------+--------------------------------------------------------+-------------------------------
          1 | Pencil       | Utensil used for writing and often works best on paper | 2023-07-26 17:20:43.639351-05
          2 | Laptop Stand | Elevated platform for laptops, enhancing ergonomics    | 2023-07-26 17:20:43.639351-05
```

#### placeholder

```sql
SELECT vectorize.table(
job_name => 'product_search_hf',
"table" => 'products',
primary_key => 'product_id',
columns => ARRAY['product_name', 'description'],
transformer => 'sentence-transformers/multi-qa-MiniLM-L6-dot-v1'
);
```
```text
                    table
---------------------------------------------
 Successfully created job: product_search_hf
(1 row)
```

```sql
SELECT * FROM vectorize.search(
job_name => 'product_search_hf',
query => 'accessories for mobile devices',
return_columns => ARRAY['product_id', 'product_name'],
num_results => 3
);
```
```text
                                       search_results
---------------------------------------------------------------------------------------------
 {"product_id": 13, "product_name": "Phone Charger", "similarity_score": 0.8147812194590133}
 {"product_id": 6, "product_name": "Backpack", "similarity_score": 0.774306211384604}
 {"product_id": 11, "product_name": "Stylus Pen", "similarity_score": 0.7709903789778251}
(3 rows)
```


### 5. Local URL

Once all of the following is complete, you should be able to visit the `Tembo-Embedding-Service` at [http://localhost:3000/docs](http://localhost:3000/docs)

# Packaging

Run this script to package into a `.deb` file, which can be installed on Ubuntu.

```
/bin/bash build-extension.sh
```

# Releases

`pg_vectorize` releases are automated through a [Github workflow](https://github.com/tembo-io/pg_vectorize/blob/main/.github/workflows/extension_ci.yml).
The compiled binaries are publish to and hosted at [pgt.dev](https://pgt.dev).
To create a release, create a new tag follow a valid [semver](https://semver.org/), then create a release with the same name.
Auto-generate the release notes and/or add more relevant details as needed.

