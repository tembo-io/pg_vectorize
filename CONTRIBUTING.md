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

### 2. Clone and compile `pg_vectorize` and extension dependencies

When progressing through these steps, refer to the following for troubleshooting:

```bash
cat ~/.pgrx/15.log
```

#### 2.1. Clone and enter directory

```bash
git clone https://github.com/tembo-io/pg_vectorize.git

cd pg_vectorize
```

#### 2.2. Install dependencies

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

#### 2.3. Apply configurations

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

#### 2.4. Compile and run `pg_vector`

```bash
make run
```


### 2. 
Once you have those pre-requisites, you need to setup `pgrx`.

```bash
cargo install --locked cargo-pgrx --version 0.11.0
```

Clone the repo and change into the directory.

```bash
git clone https://github.com/tembo-io/pgmq.git
cd pgmq
```

After this point, the steps differ slightly based on if you'd like to build
and install against an existing Postgres setup or develop against pgrx managed
development environment (which installs and allows you to test against multiple
Postgres versions).

### Install to a pre-existing Postgres

Initialize `cargo-pgrx`, and tell it the path to the your `pg_config`. For example,
if `pg_config` is on your `$PATH` and you have Postgres 15, you can run:

```bash
cargo pgrx init --pg15=`which pg_config`
```
Then, to install the release build, you can simply run:
```
cargo pgrx install --release
```

### Install against pgrx managed Postgres (Recommended for Development)

Initialize `cargo-pgrx` development environment:

```bash
cargo pgrx init
```

**Note**: Make sure you build and install `pg_partman` against the postgres installation
you want to build against (`PG_CONFIG` in `~/.pgrx/PG_VERSION/pgrx-install/bin/pg_config`
and `PGDATA` in `~/.pgrx/data-PG_MAJOR_VERSION`)

Example steps using `pg_partman` 4.7.3 and `PostgreSQL` 15.5:

```bash
wget https://github.com/pgpartman/pg_partman/archive/refs/tags/v4.7.3.tar.gz
tar xvfz v4.7.3.tar.gz
cd pg_partman-4.7.3
make install PG_CONFIG=~/.pgrx/15.5/pgrx-install/bin/pg_config PG_DATA=~/.pgrx/data-15
```

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

