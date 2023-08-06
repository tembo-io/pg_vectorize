SQLX_OFFLINE:=true

sqlx-cache:
	cargo sqlx prepare

format:
	SQLX_OFFLINE=${SQLX_OFFLINE} cargo +nightly fmt --all
	SQLX_OFFLINE=${SQLX_OFFLINE} cargo clippy

# ensure the DATABASE_URL is not used, since pgrx will stop postgres during compile
run:
	SQLX_OFFLINE=true cargo pgrx run
