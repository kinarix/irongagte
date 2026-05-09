DB_URL   := postgres://irongate:irongate@localhost:5433/irongate
REDIS_URL := redis://localhost:6379
BASE_URL := http://localhost:3000
LOG_LEVEL := debug

export DATABASE_URL = $(DB_URL)
export REDIS_URL
export BASE_URL
export LOG_LEVEL

.PHONY: build test fmt lint check up down migrate db-reset dev clean help

help:
	@echo "Targets:"
	@echo "  up         Start Postgres + Redis (docker compose)"
	@echo "  down       Stop and remove containers"
	@echo "  migrate    Run Postgres migrations"
	@echo "  db-reset   Drop and re-migrate the database"
	@echo "  dev        Start services, migrate, run the server"
	@echo "  build      Build release binary"
	@echo "  test       Run all tests"
	@echo "  fmt        Format code (cargo fmt)"
	@echo "  lint       Run clippy (warnings as errors)"
	@echo "  check      Fast type-check (no codegen)"
	@echo "  clean      Remove build artifacts"

up:
	docker compose up -d --wait

down:
	docker compose down

migrate:
	sqlx migrate run --source migrations/postgres --database-url $(DB_URL)

db-reset:
	sqlx database drop -y --database-url $(DB_URL)
	sqlx database create --database-url $(DB_URL)
	$(MAKE) migrate

dev: up migrate
	cargo run -- serve

build:
	cargo build --release

test:
	cargo test --workspace

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace -- -D warnings

check:
	cargo check --workspace

clean:
	cargo clean
