format:
	cargo sqlx prepare
	cargo +nightly fmt --all
	cargo clippy

# ensure the DATABASE_URL is not used, since pgrx will stop postgres during compile
run:
	SQLX_OFFLINE=true cargo pgrx run