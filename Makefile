SQLX_OFFLINE:=true
DATABASE_URL:=postgres://${USER}:${USER}@localhost:28815/postgres
DISTNAME = $(shell grep -m 1 '^name' Trunk.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')
DISTVERSION  = $(shell grep -m 1 '^version' Trunk.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')

sqlx-cache:
	cargo sqlx prepare

format:
	SQLX_OFFLINE=${SQLX_OFFLINE} cargo fmt --all
	SQLX_OFFLINE=${SQLX_OFFLINE} cargo clippy

# ensure the DATABASE_URL is not used, since pgrx will stop postgres during compile
run:
	SQLX_OFFLINE=true DATABASE_URL=${DATABASE_URL} cargo pgrx run pg15 postgres

META.json: META.json.in Trunk.toml
	@sed "s/@CARGO_VERSION@/$(DISTVERSION)/g" META.json.in > META.json

# `git archive` only archives committed stuff, so use `git stash create` to
# create a temporary commit to archive.
$(DISTNAME)-$(DISTVERSION).zip: META.json
	git archive --format zip --prefix $(DISTNAME)-$(DISTVERSION)/ --add-file META.json -o $(DISTNAME)-$(DISTVERSION).zip HEAD

pgxn-zip: $(DISTNAME)-$(DISTVERSION).zip

clean:
	@rm -rf META.json $(DISTNAME)-$(DISTVERSION).zip
