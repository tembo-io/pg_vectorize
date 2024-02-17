SQLX_OFFLINE:=true
DATABASE_URL:=postgres://${USER}:${USER}@localhost:28815/postgres
DISTNAME = $(shell grep -m 1 '^name' Trunk.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')
DISTVERSION  = $(shell grep -m 1 '^version' Trunk.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')
PG_VERSION:=15
PGRX_PG_CONFIG =$(shell cargo pgrx info pg-config pg${PG_VERSION})
UPGRADE_FROM_VER:=0.9.0
BRANCH:=$(git rev-parse --abbrev-ref HEAD)

.PHONY: install-pg_cron install-pg_vector install-pgmq run setup test-integration test-unit test-version test-branch test-upgrade

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

setup: install-pg_cron install-pgvector install-pgmq
	echo "shared_preload_libraries = 'pg_cron, vectorize'" >> ~/.pgrx/data-${PG_VERSION}/postgresql.conf

install-pg_cron:
	git clone https://github.com/citusdata/pg_cron.git && \
	cd pg_cron && \
	sed -i.bak 's/-Werror//g' Makefile && \
	PG_CONFIG=${PGRX_PG_CONFIG} make clean && \
	PG_CONFIG=${PGRX_PG_CONFIG} make && \
	PG_CONFIG=${PGRX_PG_CONFIG} make install && \
	cd .. && rm -rf pg_cron

install-pgvector:
	git clone --branch v0.6.0 https://github.com/pgvector/pgvector.git && \
	cd pgvector && \
	PG_CONFIG=${PGRX_PG_CONFIG} make clean && \
	PG_CONFIG=${PGRX_PG_CONFIG} make && \
	PG_CONFIG=${PGRX_PG_CONFIG} make install && \
	cd .. && rm -rf pgvector

install-pgmq:
	git clone https://github.com/tembo-io/pgmq.git && \
	cd pgmq && \
	cargo pgrx install --pg-config=${PGRX_PG_CONFIG} && \
	cd .. && rm -rf pgmq

test-integration:
	cargo test -- --ignored --test-threads=1

test-unit:
	cargo pgrx test

test-version:
	git fetch --tags
	git checkout tags/v${UPGRADE_FROM_VER}
	echo "\q" | make run
	cargo test -- --ignored --test-threads=1

test-branch:
	git checkout ${BRANCH}
	echo "\q" | make run
	make test-integration

test-upgrade:
	make test-version RUN_VER=${RUN_VER}
	psql -c "ALTER EXTENSION vectorize UPDATE"
	make test-branch BRANCH=${BRANCH}
