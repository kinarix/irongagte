# Development

## Prerequisites

- Rust stable (MSRV: 1.75+)
- PostgreSQL 14+
- Redis 6+
- Node.js 20+ (for `admin-ui`)
- `sqlx-cli` (`cargo install sqlx-cli --no-default-features --features postgres`)

## Quickstart with Docker

```bash
docker compose up -d           # postgres + redis
make db-reset                  # create db, run migrations
cargo run -p irongate-api -- admin init    # bootstrap first operator
cargo run -p irongate-api -- serve         # start API on :8080
```

In another terminal:

```bash
cd admin-ui
npm install
npm run dev                    # vite dev server on :5173
```

## Environment variables

```bash
DATABASE_URL=postgres://user:pass@localhost:5432/irongate
REDIS_URL=redis://localhost:6379
BASE_URL=https://auth.yourcompany.com
LOG_LEVEL=info                 # trace | debug | info | warn | error
LOG_FORMAT=pretty              # pretty (dev) | json (prod)

# Optional
SMTP_HOST=smtp.yourcompany.com
SMTP_PORT=587
SMTP_FROM=noreply@yourcompany.com
```

Defaults are in `config/default.yaml`. Environment variables override.

## Build

```bash
cargo build                    # debug
cargo build --release          # optimized
```

The binary is `irongate` and is produced from `crates/api`.

## Test

```bash
cargo test --workspace         # all tests (175 currently passing)
cargo test -p irongate-auth    # single crate
cargo test -p irongate-api --test api_tests   # one integration test
```

Integration tests in `crates/store/tests/` and `crates/api/tests/` use `sqlx::test`,
which spins up a real database per test and rolls back on completion. `DATABASE_URL`
must be set.

The `authz` crate's tests use `mockall` to mock repository traits — no DB needed.

### Test counts by crate

| Crate | Tests | What they cover |
|---|---|---|
| `core` | ~40 | Serde roundtrips, validation, trait objects |
| `crypto` | ~48 | JWT sign/verify, JWKS, PKCE, Argon2id, RFC vectors |
| `store` | ~30 | Postgres repos + Redis session store |
| `auth` | ~11 | Password, magic link, TOTP, sessions |
| `federation` | ~5 | OIDC RP, callback exchange |
| `authz` | ~10 | Claim resolution rules |
| `scim` | ~13 | SCIM 2.0 Users + Groups |
| `api` | ~15 | End-to-end OAuth + admin flows |

Total approx 175; see `cargo test --workspace` for the live count.

## Lint

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

CI runs both. Treat warnings as errors locally too.

## Admin UI

```bash
cd admin-ui
npm install
npm run dev         # dev server with HMR, proxies /admin to :8080
npm run build       # production build → admin-ui/dist
npm run typecheck   # tsc --noEmit
npm run lint
```

See [admin-ui.md](admin-ui.md) for the page structure.

## Database

```bash
make db-up         # docker compose up postgres
make db-reset      # drop, create, run all migrations
make db-migrate    # apply new migrations only
make db-shell      # psql into the running container
```

## Working in this repo (for AI assistants)

- All DB access uses runtime `sqlx::query()` — schema changes do not break compilation.
  Runtime errors surface in integration tests, not in `cargo check`.
- Don't `unwrap()` or `expect()` in library code. `anyhow` is only for `main.rs`.
- Don't add features, fallbacks, or abstractions speculatively. Three similar lines
  is fine; build abstractions when the third caller actually shows up.
- Don't reintroduce `Role` — see [claims-model.md](claims-model.md) for why it's gone.

## Useful commands

```bash
# inspect a JWT
cargo run -p irongate-api -- token inspect <jwt>

# create a tenant
curl -X POST http://localhost:8080/admin/v1/tenants \
  -H "Content-Type: application/json" \
  -b "irongate_admin_session=..." \
  -d '{"name":"Acme","slug":"acme"}'

# rebuild the knowledge graph (graphify skill)
/graphify
```
