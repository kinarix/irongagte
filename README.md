# Irongate

A full-featured, self-hostable Identity and Access Management (IAM) system built in Rust.
Implements OAuth 2.0 + OIDC (as both server and client), local authentication, federated
identity (Google, GitHub, LDAP), RBAC/ABAC authorization, SCIM 2.0, and multi-tenancy.

## Features

| Feature | Status |
|---|---|
| Domain types, error enums, repository traits | ✅ Complete |
| JWT (RS256/ES256), JWKS, Argon2id, PKCE, TOTP, opaque tokens | ✅ Complete |
| PostgreSQL + SQLite repositories, Redis session store, migrations | ✅ Complete |
| Password auth, magic links, TOTP enrollment, session management | ✅ Complete |
| WebAuthn / passkey registration + assertion | Planned |
| Federated IdPs: Google (OIDC), GitHub (OAuth2), LDAP | Planned |
| RBAC engine, ABAC policy evaluation, scope resolution | Planned |
| SCIM 2.0 Users + Groups endpoints | Planned |
| OAuth 2.0 server, OIDC discovery, management API (Axum) | Planned |

## Architecture

Cargo workspace with fine-grained crates. Dependencies flow strictly inward.

```
crates/
├── core/        # Domain types, traits, errors — no external deps
├── crypto/      # JWT, JWKS, key rotation, hashing
├── store/       # SQLx repos (Postgres + SQLite), Redis session store
├── auth/        # Password, magic link, TOTP, session flows
├── webauthn/    # Passkey registration + assertion (webauthn-rs)
├── federation/  # OIDC RP, OAuth2 clients, LDAP
├── authz/       # RBAC engine, ABAC policy evaluation
├── scim/        # SCIM 2.0 provisioning API
└── api/         # Axum routers, all OAuth2/OIDC/management endpoints
```

Dependency rule: `api → auth, federation, authz, scim, webauthn, store, crypto, core`.
No crate may depend on a crate above it in this list.

## Self-hosting modes

- **Standalone** — SQLite, no Redis required. Single binary.
- **Distributed** — PostgreSQL + Redis. HA-ready.

## Building

```bash
# Install sqlx-cli for migrations
cargo install sqlx-cli --no-default-features --features postgres,sqlite

# Run migrations (Postgres)
sqlx migrate run --source migrations/postgres

# Run migrations (SQLite)
sqlx migrate run --source migrations/sqlite

# Build
cargo build --release

# Test
cargo test --workspace
```

Requires Rust stable (MSRV: 1.75+).

## Running

```bash
export DATABASE_URL=postgres://user:pass@localhost:5432/irongate
export REDIS_URL=redis://localhost:6379
export BASE_URL=https://auth.yourcompany.com

irongate serve
```

## Testing

```bash
# All tests (126 currently passing)
cargo test --workspace

# Single crate
cargo test -p irongate-auth
```

Integration tests in `crates/store/tests/` use `sqlx::test` — they spin up a real database,
run migrations, and roll back automatically. Set `DATABASE_URL` before running them.

## Crate test counts

| Crate | Tests | Notes |
|---|---|---|
| `core` | 37 | Serde roundtrips, trait objects, error enums |
| `crypto` | 48 | RFC test vectors, JWT sign/verify, Argon2id |
| `store` | 30 | SQLite + Postgres integration, RedisSessionStore |
| `auth` | 11 | Mockall unit tests: all four services |

## Phase 1 scope

In scope: local auth, WebAuthn, Google/GitHub/LDAP federation, OAuth 2.0 server, OIDC,
JWT key rotation, RBAC + basic ABAC, SCIM 2.0, multi-tenancy, audit logging.

Out of scope (Phase 2): SAML 2.0, risk-based MFA, webhooks, organization hierarchy,
Verifiable Credentials, admin dashboard UI.

## License

Apache 2.0 — see [LICENSE](LICENSE).
