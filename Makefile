SQLX_OFFLINE:=true
DATABASE_URL:=postgres://${USER}:${USER}@localhost:28815/postgres

sqlx-cache:
	cargo sqlx prepare

format:
	SQLX_OFFLINE=${SQLX_OFFLINE} cargo +nightly fmt --all
	SQLX_OFFLINE=${SQLX_OFFLINE} cargo clippy

# ensure the DATABASE_URL is not used, since pgrx will stop postgres during compile
run:
	SQLX_OFFLINE=true DATABASE_URL=${DATABASE_URL} cargo pgrx run pg15 postgres
