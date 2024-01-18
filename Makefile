SQLX_OFFLINE:=true
DATABASE_URL:=postgres://${USER}:${USER}@localhost:28815/postgres
DISTNAME = $(shell grep -m 1 '^name' Trunk.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')
DISTVERSION  = $(shell grep -m 1 '^version' Trunk.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')

sqlx-cache:
	cargo sqlx prepare

format:
	SQLX_OFFLINE=${SQLX_OFFLINE} cargo +nightly fmt --all
	SQLX_OFFLINE=${SQLX_OFFLINE} cargo clippy

# ensure the DATABASE_URL is not used, since pgrx will stop postgres during compile
run:
	SQLX_OFFLINE=true DATABASE_URL=${DATABASE_URL} cargo pgrx run pg15 postgres

META.json.bak: Trunk.toml META.json
	@sed -i.bak "s/@CARGO_VERSION@/$(DISTVERSION)/g" META.json

# `git archive` only archives committed stuff, so use `git stash create` to
# create a temporary commit to archive.
pgxn-zip: META.json.bak	
	git archive --format zip --prefix=$(DISTNAME)-$(DISTVERSION)/ -o $(DISTNAME)-$(DISTVERSION).zip $$(git stash create)
	@mv META.json.bak META.json
