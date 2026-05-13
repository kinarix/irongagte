DB_URL         := postgres://irongate:irongate@localhost:5433/irongate
REDIS_URL      := redis://localhost:6379
PORT           := 8081
BASE_URL       := http://localhost:$(PORT)
LOG_LEVEL      := debug
ADMIN_EMAIL    ?= admin
ADMIN_PASSWORD ?= admin

export DATABASE_URL = $(DB_URL)
export REDIS_URL
export PORT
export BASE_URL
export LOG_LEVEL

.PHONY: build build-admin test fmt lint check up down infra migrate db-reset dev admin-init clean help

help:
	@echo "Targets:"
	@echo "  infra        Start Postgres + Redis (docker compose)"
	@echo "  up           Alias for infra"
	@echo "  down         Stop and remove containers"
	@echo "  migrate      Run Postgres migrations"
	@echo "  db-reset     Drop and re-migrate the database"
	@echo "  admin-init   Bootstrap super-admin user and register admin OAuth2 client"
	@echo "  dev          Start services, migrate, run the server"
	@echo "  build-admin  Build admin UI (admin-ui/ → crates/api/static/admin/)"
	@echo "  build        Build admin UI then release binary"
	@echo "  test         Run all tests"
	@echo "  fmt          Format code (cargo fmt)"
	@echo "  lint         Run clippy (warnings as errors)"
	@echo "  check        Fast type-check (no codegen)"
	@echo "  clean        Remove build artifacts"

up: infra

down:
	docker compose down

migrate:
	sqlx migrate run --source migrations/postgres --database-url $(DB_URL)

db-reset:
	sqlx database drop -y --database-url $(DB_URL)
	sqlx database create --database-url $(DB_URL)
	$(MAKE) migrate

infra:
	docker compose up -d --wait postgres redis

admin-init: infra migrate
	cargo run -- admin init \
		--email $(ADMIN_EMAIL) \
		--password $(ADMIN_PASSWORD) \
		--extra-redirect-uri http://localhost:5173/admin/callback

dev: infra migrate
	cargo run -- serve

build-admin:
	cd admin-ui && npm run build

build: build-admin
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
