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
pgrx init
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
`pgrx` uses a specific file path for postgres configurations, which, in the following example, utilizes Postgres version 15.
If you're using a different version, please alter the file path value `data-<postgres-version>`.
With your preferred IDE or text editor, run the following:

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

From within the pg_vectorize directory, run the following:

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

Once the above command is run, you will be brought into Postgres via `psql`



Once you have those pre-requisites, you need to setup `pgrx`.

### Install against pgrx managed Postgres (Recommended for Development)

Initialize `cargo-pgrx` development environment:

```bash
cargo pgrx init
```

**Note**: Make sure you build and install `pg_partman` against the postgres installation
you want to build against (`PG_CONFIG` in `~/.pgrx/PG_VERSION/pgrx-install/bin/pg_config`
and `PGDATA` in `~/.pgrx/data-PG_MAJOR_VERSION`)

Then, you can use the run command, which will build and install the extension
and drop you into psql:

```bash
cargo pgrx run pg15
```

Finally, you can create the extension and get started with the example in the [README.md](README.md).

```psql
CREATE EXTENSION pgmq cascade;
```

# Packaging

Run this script to package into a `.deb` file, which can be installed on Ubuntu.

```
/bin/bash build-extension.sh
```

# Releases

PGMQ Postgres Extension releases are automated through a [Github workflow](https://github.com/tembo-io/pgmq/blob/main/.github/workflows/extension_ci.yml). The compiled binaries are publish to and hosted at [pgt.dev](https://pgt.dev). To create a release, create a new tag follow a valid [semver](https://semver.org/), then create a release with the same name. Auto-generate the release notes and/or add more relevant details as needed. See subdirectories for the [Rust](https://github.com/tembo-io/pgmq/tree/main/core) and [Python](https://github.com/tembo-io/pgmq/tree/main/tembo-pgmq-python) SDK release processes.

