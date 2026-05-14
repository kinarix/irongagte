# Irongate

A full-featured, self-hostable Identity and Access Management (IAM) system built in Rust.
Implements OAuth 2.0 + OIDC (as both server and client), local authentication, federated
identity (Google, GitHub, LDAP), claim-based authorization, SCIM 2.0, and multi-tenancy.

📖 **[Documentation](docs/)** — architecture, claims model, admin API, OAuth/OIDC endpoints, and more.

## Features

| Feature | Status |
|---|---|
| Domain types, error enums, repository traits | ✅ |
| JWT (RS256/ES256), JWKS, Argon2id, PKCE, TOTP, opaque tokens | ✅ |
| PostgreSQL repositories, Redis session store, sqlx migrations | ✅ |
| Password auth, magic links, TOTP enrollment, session management | ✅ |
| WebAuthn / passkey registration + assertion | ✅ |
| Federated IdPs: Google (OIDC), GitHub (OAuth2), LDAP, generic OIDC | ✅ |
| Claim-based authorization (prefix per app, group + user assignments) | ✅ |
| SCIM 2.0 Users + Groups endpoints | ✅ |
| OAuth 2.0 server, OIDC discovery, management API (Axum) | ✅ |
| Operator RBAC (admin-side permissions) | ✅ |
| Admin UI (React + Vite) | ✅ |
| SAML 2.0 | Phase 2 |

## Architecture

Cargo workspace with fine-grained crates. Dependencies flow strictly inward.

```
crates/
├── core/        # Domain types, traits, errors — no external deps
├── crypto/      # JWT, JWKS, key rotation, hashing, PKCE
├── store/       # PostgreSQL repositories, Redis session store
├── auth/        # Password, magic link, TOTP, session flows
├── webauthn/    # Passkey registration + assertion
├── federation/  # OIDC RP, OAuth2 clients, LDAP
├── authz/       # Claim resolution engine
├── scim/        # SCIM 2.0 provisioning API
└── api/         # Axum routers, all OAuth2/OIDC/management endpoints + binary
```

Dependency rule: `api → auth, federation, authz, scim, webauthn, store, crypto, core`.
No crate may depend on a crate above it in this list.

## The claim model in one breath

There is no `Role` entity for end users. Each **application** has a unique
`claim_prefix`. Custom claims are defined per app (`scalar` or `multi`) and projected
into JWTs as `<prefix>:<key>`. Values are assigned to **groups** (members inherit) or
directly to **users**. At token mint time: `multi` claims merge across sources;
`scalar` claims pick user-direct, then highest-priority group, then earliest
`created_at`. Standard OIDC claims come from the user record at canonical keys
unprefixed.

Full design: [docs/claims-model.md](docs/claims-model.md).

## Quickstart

```bash
# Bring up postgres + redis
docker compose up -d

# Migrate the database
cargo install sqlx-cli --no-default-features --features postgres
export DATABASE_URL=postgres://irongate:irongate@localhost:5432/irongate
sqlx migrate run --source migrations/postgres

# Bootstrap the first operator
cargo run -p irongate-api -- admin init

# Start the server
export REDIS_URL=redis://localhost:6379
export BASE_URL=http://localhost:8080
cargo run -p irongate-api -- serve

# (separate terminal) Start the admin UI
cd admin-ui && npm install && npm run dev
```

Requires Rust stable (MSRV: 1.75+) and Node 20+ for the admin UI.

## Running

```bash
export DATABASE_URL=postgres://user:pass@localhost:5432/irongate
export REDIS_URL=redis://localhost:6379
export BASE_URL=https://auth.yourcompany.com

irongate serve
```

A multi-stage `Dockerfile` and `docker-compose.yml` are included for production.

## Testing

```bash
cargo test --workspace        # 175 tests
cargo test -p irongate-auth   # single crate

cd admin-ui
npm run typecheck
npm run lint
```

Integration tests use `sqlx::test` — they spin up a real database, run migrations,
and roll back automatically. Set `DATABASE_URL` before running them.

## Phase 1 scope

In scope: local auth, WebAuthn, Google/GitHub/LDAP federation, OAuth 2.0 server, OIDC,
JWT key rotation, claim-based authorization, SCIM 2.0, multi-tenancy, operator RBAC,
admin UI, audit logging.

Out of scope (Phase 2): SAML 2.0, risk-based MFA, webhooks, organization hierarchy,
Verifiable Credentials.

## License

Apache 2.0 — see [LICENSE](LICENSE).
