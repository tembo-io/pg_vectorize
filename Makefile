SQLX_OFFLINE:=true
DATABASE_URL:=postgres://${USER}:${USER}@localhost:28815/postgres
DISTNAME = $(shell grep -m 1 '^name' Trunk.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')
DISTVERSION  = $(shell grep -m 1 '^version' Trunk.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')
PG_VERSION:=15
PGRX_PG_CONFIG =$(shell cargo pgrx info pg-config pg${PG_VERSION})

sqlx-cache:
	cargo sqlx prepare

format:
	SQLX_OFFLINE=${SQLX_OFFLINE} cargo fmt --all
	SQLX_OFFLINE=${SQLX_OFFLINE} cargo clippy

# ensure the DATABASE_URL is not used, since pgrx will stop postgres during compile
run:
	SQLX_OFFLINE=true DATABASE_URL=${DATABASE_URL} cargo pgrx run pg${PG_VERSION} postgres

META.json: META.json.in Trunk.toml
	@sed "s/@CARGO_VERSION@/$(DISTVERSION)/g" META.json.in > META.json

# `git archive` only archives committed stuff, so use `git stash create` to
# create a temporary commit to archive.
$(DISTNAME)-$(DISTVERSION).zip: META.json
	git archive --format zip --prefix $(DISTNAME)-$(DISTVERSION)/ --add-file META.json -o $(DISTNAME)-$(DISTVERSION).zip HEAD

pgxn-zip: $(DISTNAME)-$(DISTVERSION).zip

clean:
	@rm -rf META.json $(DISTNAME)-$(DISTVERSION).zip

install_pg_cron:
	git clone https://github.com/citusdata/pg_cron.git && \
	cd pg_cron && \
	PG_CONFIG=${PGRX_PG_CONFIG} make && \
	PG_CONFIG=${PGRX_PG_CONFIG} make install

install_pg_vector:
	git clone --branch v0.6.0 https://github.com/pgvector/pgvector.git && \
	cd pgvector && \
	PG_CONFIG=${PGRX_PG_CONFIG} make && \
	PG_CONFIG=${PGRX_PG_CONFIG} make install

install_pgmq:
	git clone https://github.com/tembo-io/pgmq.git && \
	cd pgmq && \
	cargo pgrx install --pg-config=${PGRX_PG_CONFIG}

test-integration:
	cargo test -- --ignored --test-threads=1

test-unit:
	cargo pgrx test

# tests upgrading from specific version
RUN_VER:=0.9.0
test-version:
	git fetch --tags
	git checkout tags/v${RUN_VER}
	echo "\q" | make run
	cargo test -- --ignored --test-threads=1

BRANCH:=main
test-branch:
	git checkout ${BRANCH}
	echo "\q" | make run
	$(MAKE) test-integration

test-upgrade:
	$(MAKE) test-version RUN_VER=${RUN_VER}
	psql -c "ALTER EXTENSION vectorize UPDATE"
	$(MAKE) test-branch BRANCH=${BRANCH}
