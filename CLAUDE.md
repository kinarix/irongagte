# Irongate — Claude Code Context

> **GitHub:** https://github.com/kinarix/irongate

## Project Overview

**Irongate** is a full-featured, self-hostable Identity and Access Management (IAM) system
built in Rust. It implements OAuth 2.0 + OIDC (as both server and client), local
authentication, federated identity (Google, GitHub, LDAP), RBAC/ABAC authorization,
SCIM 2.0, and multi-tenancy. Open source, enterprise-ready.

**Phase 1 scope — SAML is explicitly deferred to Phase 2.**

---

## Architecture

Cargo workspace with fine-grained crates. Each crate has a single responsibility and is
independently testable. Dependencies flow in one direction only: outer crates depend on inner
crates, never the reverse.

```
identity-system/
├── CLAUDE.md
├── Cargo.toml                  # workspace root — name = "irongate"
├── migrations/                 # sqlx migrations (postgres + sqlite)
├── config/
│   └── default.yaml
└── crates/
    ├── core/                   # Domain types, traits, errors — no external deps
    ├── crypto/                 # JWT, JWKS, key rotation, hashing — depends on core
    ├── store/                  # DB + Redis layer — depends on core, crypto
    ├── auth/                   # Local auth flows — depends on core, crypto, store
    ├── federation/             # OIDC RP, OAuth2 clients, LDAP — depends on core, store
    ├── authz/                  # RBAC/ABAC engine — depends on core, store
    ├── scim/                   # SCIM 2.0 API — depends on core, store
    ├── webauthn/               # Passkey flows — depends on core, store
    └── api/                    # Axum routers + handlers — depends on all crates above
```

### Dependency Rule

```
api → auth, federation, authz, scim, webauthn, store, crypto, core
auth → store, crypto, core
federation → store, crypto, core
authz → store, core
scim → store, core
webauthn → store, core
crypto → core
store → core
core → (nothing internal)
```

---

## Crate Responsibilities

### `core`
- All domain types: `User`, `Tenant`, `Application`, `Session`, `Role`, `Permission`, `Identity`
- All error types (use `thiserror`)
- Core traits: `IdentityProvider`, `TokenStore`, `UserStore`
- No database, no HTTP, no crypto — pure domain logic

### `crypto`
- JWT sign and verify (RS256, ES256)
- JWKS serialization and publication
- Signing key lifecycle (generation, rotation, expiry)
- Argon2id password hashing and verification
- PKCE (code verifier / code challenge)
- Secure random token generation
- Base64url encoding (constant-time)

### `store`
- All sqlx queries — compile-time verified
- Dual database support: PostgreSQL (distributed) and SQLite (standalone)
- Redis layer for sessions and refresh tokens
- Repository pattern: one struct per aggregate (`UserRepo`, `SessionRepo`, etc.)
- Migration files live in `/migrations`, run via `sqlx migrate run`

### `auth`
- Local credential flows: password login, magic link, TOTP
- Session creation and validation
- MFA enforcement logic
- Refresh token rotation

### `federation`
- `IdentityProvider` trait implementations:
  - `OidcProvider` — Google, GitHub, any OIDC-compliant IdP
  - `OAuth2Provider` — providers without OIDC (plain OAuth2)
  - `LdapProvider` — Active Directory and OpenLDAP
- Handles: authorization URL generation, callback exchange, JIT provisioning
- Account linking: match federated identity to existing user by `sub` or email

### `authz`
- RBAC engine: user → roles → permissions
- ABAC policy evaluation (simple policy DSL, not full OPA)
- Scope resolution: OAuth scopes → permissions
- Permission embedding into access tokens

### `scim`
- SCIM 2.0 REST API (Users and Groups endpoints)
- Supports create, read, update, patch, delete, list with filtering
- Used by Okta, Azure AD, HR systems for automated provisioning

### `webauthn`
- Registration ceremony (credential creation)
- Authentication ceremony (credential assertion)
- Credential storage and retrieval
- Uses `webauthn-rs` crate

### `api`
- All Axum routers and handlers
- Middleware: auth, rate limiting, tracing, tenant resolution
- OIDC endpoints: `/oauth2/authorize`, `/oauth2/token`, `/oauth2/introspect`,
  `/oauth2/revoke`, `/oauth2/userinfo`, `/.well-known/openid-configuration`,
  `/.well-known/jwks.json`
- Management REST API: `/api/v1/users`, `/api/v1/applications`, etc.
- Admin API, SCIM API, health/readiness endpoints

---

## Key Design Decisions

### IdentityProvider Trait (most important abstraction)

Every IdP — local, Google, GitHub, LDAP, and eventually SAML — implements this:

```rust
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;

    async fn authorization_url(
        &self,
        state: &str,
        nonce: Option<&str>,
    ) -> Result<Url, IdpError>;

    async fn exchange_callback(
        &self,
        params: CallbackParams,
    ) -> Result<FederatedIdentity, IdpError>;
}

pub struct FederatedIdentity {
    pub provider_user_id: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub raw_claims: serde_json::Value,
}
```

The rest of the system only talks to this trait. SAML in Phase 2 is just another implementor.

### Error Handling

- Each crate defines its own error enum using `thiserror`
- Errors propagate upward and are mapped to HTTP responses only at the `api` crate boundary
- Never use `unwrap()` or `expect()` in library code — only in tests or `main`
- Use `anyhow` only in binary entry points (`main.rs`), not in library crates

### Database

- All queries use `sqlx` with compile-time verification (`query!` / `query_as!` macros)
- Migrations in `/migrations` numbered sequentially: `0001_create_tenants.sql`, etc.
- Every table has `tenant_id` for multi-tenancy — always filter by it
- Use UUIDs for all primary keys (stored as `uuid` in postgres, `text` in sqlite)
- Soft-delete pattern: `deleted_at TIMESTAMP` instead of `DROP`

### Tokens

- **Access Token**: JWT (RS256), short-lived (1 hour), contains `sub`, `aud`, `scope`,
  `roles`, `permissions`, `jti`
- **ID Token**: JWT (RS256), contains identity claims, addressed to the client (`aud`)
- **Refresh Token**: opaque random string, SHA-256 hashed before DB storage,
  rotated on every use
- **Session**: stored in Redis, referenced by secure httpOnly cookie

### Multi-tenancy

- All domain objects are scoped to a `tenant_id`
- Tenant is resolved early in the request lifecycle from the hostname or a path prefix
- Signing keys are per-tenant
- IdP configurations are per-tenant

---

## Dependencies (Workspace)

```toml
[workspace.dependencies]
# Web
axum             = "0.7"
tower            = "0.4"
tower-http       = "0.5"
tokio            = { version = "1", features = ["full"] }

# Database
sqlx             = { version = "0.7", features = ["postgres", "sqlite", "uuid", "time", "runtime-tokio"] }

# Cache
fred             = "8"

# Crypto / JWT
jsonwebtoken     = "9"
argon2           = "0.5"
password-hash    = "0.5"
sha2             = "0.10"
rand             = "0.8"
base64ct         = "1"
p256             = "0.13"
rsa              = "0.9"

# MFA
totp-rs          = "5"
webauthn-rs      = "0.5"

# Federation
reqwest          = { version = "0.12", features = ["json"] }
ldap3            = "0.11"

# Email
lettre           = "0.11"

# Serialization
serde            = { version = "1", features = ["derive"] }
serde_json       = "1"
uuid             = { version = "1", features = ["v4"] }
time             = { version = "0.3", features = ["serde"] }

# Config
config           = "0.14"

# Error handling
thiserror        = "1"
anyhow           = "1"

# Async traits
async-trait      = "0.1"

# Observability
tracing                     = "0.1"
tracing-subscriber          = "0.3"
metrics                     = "0.22"
metrics-exporter-prometheus = "0.13"
```

---

## Build Order (follow this sequence)

Build and fully test each crate before starting the next.

```
1. core        → domain types, traits, error types
2. crypto      → JWT, hashing, key management
3. store       → DB schema, migrations, repositories
4. auth        → password, magic link, TOTP, sessions
5. webauthn    → passkey registration + assertion
6. federation  → OIDC RP, OAuth2, LDAP providers
7. authz       → RBAC engine, permission resolution
8. scim        → SCIM 2.0 endpoints
9. api         → wire everything together, full integration tests
```

---

## Testing Strategy

- **Unit tests**: in-module, `#[cfg(test)]`, mock traits with `mockall`
- **Integration tests**: in `tests/` per crate, use `sqlx::test` for DB tests
  (spins up a real DB, runs migrations, rolls back after)
- **API tests**: `axum::test` + `tower::ServiceExt` — no real HTTP server needed
- **Crypto tests**: test vectors from RFC specs where available
- Always test the unhappy path: expired tokens, wrong audience, revoked sessions,
  locked accounts, duplicate emails

```toml
[dev-dependencies]
mockall     = "0.12"
axum-test   = "0.1"
fake        = "2"        # generate fake user data in tests
```

---

## Code Style

- `rustfmt` — always (`cargo fmt` before commit)
- `clippy` — treat warnings as errors in CI (`cargo clippy -- -D warnings`)
- No `unwrap()` in library code
- Prefer `impl Trait` for function arguments, `Box<dyn Trait>` for stored heterogeneous values
- Keep handlers thin — handlers extract params and delegate to service functions
- Service functions contain business logic — no Axum types, no sqlx types
- Repository functions contain only DB queries — no business logic

---

## Environment Variables

```bash
# Required
DATABASE_URL=postgres://user:pass@localhost:5432/identity
REDIS_URL=redis://localhost:6379
BASE_URL=https://auth.yourcompany.com

# Optional — fall back to config/default.yaml
SMTP_HOST=smtp.yourcompany.com
SMTP_PORT=587
SMTP_FROM=noreply@yourcompany.com

LOG_LEVEL=info          # trace | debug | info | warn | error
LOG_FORMAT=json         # json (prod) | pretty (dev)
```

---

## What's In Phase 1 / What's Not

### In Scope
- Local auth: password, magic link, TOTP, passkeys (WebAuthn)
- Federated: Google (OIDC), GitHub (OAuth2), LDAP / Active Directory
- OAuth 2.0 server: Authorization Code + PKCE, Client Credentials, Device Flow, Refresh
- OIDC: ID Token, UserInfo, Discovery document, JWKS
- JWT: RS256 and ES256, key rotation
- RBAC + basic ABAC
- SCIM 2.0 (Users + Groups)
- Session management, refresh token rotation
- Audit logging
- Multi-tenancy
- Self-host modes: standalone (SQLite) and distributed (PostgreSQL + Redis)

### Explicitly Out of Scope (Phase 2+)
- SAML 2.0 — deferred, ecosystem not mature enough
- Risk-based / adaptive MFA
- Webhooks for user lifecycle events
- Organization hierarchy / sub-tenants
- Verifiable Credentials (W3C)
- Admin dashboard UI (spec exists, UI is a separate project)

---

## Binary Name

The compiled binary is `irongate`. All CLI examples use this name:

```bash
irongate serve
irongate token inspect <jwt>
irongate idp add google
irongate tenant create acme-corp
irongate user suspend alice@company.com
irongate migrate run
```

---

## Repository

```
github.com/kinarix/irongate
```

---

## Starting Point

When starting work, begin with `crates/core`:

1. Create the workspace `Cargo.toml`
2. Scaffold all crate directories with stub `Cargo.toml` and `src/lib.rs`
3. Implement `core` fully first:
   - All domain structs (`User`, `Tenant`, `Application`, `Session`, `Role`, `Permission`, `Identity`, `IdpConfig`)
   - All error enums per crate
   - The `IdentityProvider` trait
   - The repository traits (`UserRepository`, `SessionRepository`, etc.)
4. Then move to `crypto`
5. Commit after each crate is complete and tests pass
