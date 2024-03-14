SQLX_OFFLINE:=true
DATABASE_URL:=postgres://${USER}:${USER}@localhost:28815/postgres
RUST_LOG:=debug


sqlx-cache:
	cargo sqlx prepare

run-worker:
	cargo run --bin vectorize-worker