SQLX_OFFLINE:=true
DATABASE_URL:=postgres://${USER}:${USER}@localhost:28817/postgres
RUST_LOG:=debug


sqlx-cache:
	DATABASE_URL=${DATABASE_URL} cargo sqlx prepare

run-worker:
	cargo run --bin vectorize-worker