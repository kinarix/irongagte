# Irongate — Claude Code Context

> **GitHub:** https://github.com/kinarix/irongate
> **Docs:** [`docs/`](docs/) — start here for architecture, claims model, admin API, etc.

## Knowledge Graph

An interactive knowledge graph of this codebase is available at:

```
graphify-out/graph.html        — interactive HTML visualization (open in browser)
graphify-out/graph_export.json — GraphRAG-ready JSON
graphify-out/GRAPH_REPORT.md   — plain-language architecture report
```

Run `/graphify` to rebuild after significant code changes.

## Project Overview

**Irongate** is a full-featured, self-hostable Identity and Access Management (IAM) system
built in Rust. It implements OAuth 2.0 + OIDC (as both server and client), local
authentication, federated identity (Google, GitHub, LDAP), claim-based authorization,
SCIM 2.0, and multi-tenancy.

**Phase 1 scope — SAML is explicitly deferred to Phase 2.**

---

## Architecture

Cargo workspace with fine-grained crates. Each crate has a single responsibility and is
independently testable. Dependencies flow in one direction only: outer crates depend on inner
crates, never the reverse.

```
irongate/
├── CLAUDE.md
├── README.md
├── Cargo.toml                  # workspace root — name = "irongate"
├── docs/                       # full documentation
├── migrations/postgres/        # sqlx migrations (Postgres only)
├── admin-ui/                   # Vite + React + TS admin SPA
├── config/
│   └── default.yaml
└── crates/
    ├── core/                   # Domain types, traits, errors — no external deps
    ├── crypto/                 # JWT, JWKS, key rotation, hashing — depends on core
    ├── store/                  # PostgreSQL + Redis — depends on core, crypto
    ├── auth/                   # Local auth flows — depends on core, crypto, store
    ├── federation/             # OIDC RP, OAuth2 clients, LDAP — depends on core, store
    ├── authz/                  # Claim resolution engine — depends on core, store
    ├── scim/                   # SCIM 2.0 API — depends on core, store
    ├── webauthn/               # Passkey flows — depends on core, store
    └── api/                    # Axum routers + handlers + binary — depends on all above
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
- Domain types: `User`, `Tenant`, `Application`, `Session`, `Group`, `ClaimDefinition`,
  `ClaimType`, `GroupClaim`, `UserClaim`, `Identity`, `IdpConfig`, `RefreshToken`,
  `AuditEvent`, `Operator`, `OperatorRole`, `OperatorPermission`
- All error types (use `thiserror`)
- Core traits: `IdentityProvider`, `UserRepository`, `GroupRepository`,
  `ClaimDefinitionRepository`, `GroupClaimRepository`, `UserClaimRepository`,
  `ApplicationRepository`, `SessionRepository`, …
- Validation helpers: `validate_claim_prefix`, `validate_claim_key`
- No database, no HTTP, no crypto — pure domain logic
- **No `Role` entity for end users.** End-user authz is the claim model below.
  Admin/operator authz uses a separate `OperatorRole` + `OperatorPermission` system.

### `crypto`
- JWT sign and verify (RS256, ES256)
- JWKS serialization and publication
- Signing key lifecycle (generation, rotation, expiry)
- Argon2id password hashing and verification
- PKCE (code verifier / code challenge)
- Secure random token generation
- Base64url encoding (constant-time)

### `store`
- All sqlx queries — **runtime** `sqlx::query()` and `sqlx::query_as()`, not the
  compile-time `query!` macros. Schema changes don't break `cargo check`; correctness
  is verified by integration tests.
- PostgreSQL only — SQLite was removed.
- Redis layer for sessions and refresh-token revocation lookups
- Repository pattern: one struct per aggregate (`UserRepo`, `SessionRepo`,
  `ClaimDefinitionRepo`, `GroupClaimRepo`, `UserClaimRepo`, etc.)
- Migration files live in `migrations/postgres/`, run via `sqlx migrate run`

### `auth`
- Local credential flows: password login, magic link, TOTP
- Session creation and validation
- MFA enforcement logic
- Refresh token rotation

### `federation`
- `IdentityProvider` trait implementations:
  - `OidcProvider` — Google, generic OIDC
  - `OAuth2Provider` — GitHub and any OAuth2-without-OIDC
  - `LdapProvider` — Active Directory and OpenLDAP
- Handles: authorization URL generation, callback exchange, JIT provisioning
- Account linking: match federated identity to existing user by `(provider, sub)` or email

### `authz`
- **Claim resolution engine.** `resolve_claims_for_app(user_id, application_id, claim_prefix)`
  returns the `HashMap<String, Value>` to embed in a JWT.
- For each `ClaimDefinition` of the application:
  - `multi` → union and dedupe values from all group + user sources, emit as JSON array
  - `scalar` → user-direct wins; else highest `group.priority`; else earliest `created_at`
- Standard OIDC claims (`sub`, `email`, …) are added by `handlers/token.rs` from the
  `User` record at canonical unprefixed keys.

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
- Management REST API at `/admin/v1/*`: `tenants`, `users`, `applications`, `groups`,
  `claims/{definitions,group-assignments,user-assignments,effective}`, `idp/configs`,
  `sessions`, `audit`, `operators`, `operator-roles`, `operator-permissions`
- SCIM API, health/readiness endpoints
- Binary entry point (`cargo run -p irongate-api -- serve`)

---

## Key Design Decisions

### Claim model (no `Role` entity)

See [`docs/claims-model.md`](docs/claims-model.md) for the full design. Summary:

- **No `Role` table** — dropped in migration `0028_drop_roles.sql`.
- Each `Application` has a unique-per-tenant `claim_prefix: String` (validated against
  the OIDC reserved-name list).
- Claims are defined per application: `ClaimDefinition { application_id, key,
  claim_type: scalar|multi, description }`.
- Values are assigned to **groups** (`GroupClaim`) or directly to **users** (`UserClaim`).
- JWT key is `<prefix>:<key>`, e.g. `billing:roles`.
- Conflict resolution:
  - `multi` → BTreeSet merge across all sources → JSON array
  - `scalar` → user-direct > group with highest `priority` > earliest `created_at`
- OIDC standard claims (`sub`, `email`, `name`, `picture`, …) come from `User` and are
  emitted unprefixed at canonical OIDC keys.

When asked to "add a role", define a `multi` claim called `roles` on the relevant
application and assign values via groups. Do not reintroduce the `Role` entity.

### IdentityProvider Trait

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

- All queries use `sqlx` at runtime — `sqlx::query()` / `sqlx::query_as()`. **Do not**
  introduce compile-time `query!` / `query_as!` macros; they would require `DATABASE_URL`
  to be set at compile time and would break offline builds.
- PostgreSQL only. SQLite support was removed pre-Phase-1-complete.
- Migrations in `migrations/postgres/` numbered sequentially: `0001_create_tenants.sql`, etc.
- Every domain table has `tenant_id` for multi-tenancy — always filter by it.
- Use UUIDs (`uuid` type, `gen_random_uuid()`) for all primary keys.
- Soft-delete pattern: `deleted_at TIMESTAMPTZ` instead of `DROP`. Unique indexes that
  should exclude deleted rows are partial: `WHERE deleted_at IS NULL`.

### Tokens

- **Access Token**: JWT (RS256), short-lived (1 hour), contains `sub`, `aud`, `scope`,
  `tenant_id`, `jti`, plus custom claims resolved per the claim model.
- **ID Token**: JWT (RS256), contains identity claims, addressed to the client (`aud`).
- **Refresh Token**: opaque random string, SHA-256 hashed before DB storage,
  rotated on every use.
- **Session**: stored in Redis, referenced by secure httpOnly cookie.

### Multi-tenancy

- All domain objects are scoped to a `tenant_id`.
- Tenant is resolved early in the request lifecycle:
  - Admin API — from path/query parameter, gated by operator permissions
  - OAuth/OIDC — derived from `client_id` → `application.tenant_id`
  - SCIM — from bearer token scope
- Signing keys are per-tenant.
- IdP configurations are per-tenant.
- Application `claim_prefix` is unique per tenant.

### Operator RBAC (admin-side, separate from end-user claims)

- `Operator` (admin account) is a separate auth tree from `User` (end users).
- `OperatorPermission { scope, action }` catalog seeded on bootstrap.
- `OperatorRole` bundles permissions; can be global (`tenant_id IS NULL`) or
  tenant-scoped.
- `OperatorRoleAssignment` grants a role to an operator.
- Admin handlers call `operator_authz.require(operator, scope, action, tenant_id?)`.
- See [`docs/operator-rbac.md`](docs/operator-rbac.md).

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
sqlx             = { version = "0.7", features = ["postgres", "uuid", "time", "runtime-tokio"] }

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

## Testing Strategy

- **Unit tests**: in-module, `#[cfg(test)]`, mock traits with `mockall`
- **Integration tests**: in `tests/` per crate. Repository tests use `sqlx::test` which
  spins up a real DB, runs migrations, and rolls back after.
- **API tests**: `axum::test` + `tower::ServiceExt` — no real HTTP server needed.
- **authz** tests use mocked repositories to verify the claim resolution algorithm in
  isolation, with hand-built fixture data covering scalar/multi, priority ties, and
  user override.
- Always test the unhappy path: expired tokens, wrong audience, revoked sessions,
  locked accounts, duplicate emails, conflicting claim values.

Current state: **175 tests passing** across the workspace (see `cargo test --workspace`).

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
- Prefer `impl Trait` for function arguments, `Box<dyn Trait>` or `Arc<dyn Trait>` for
  stored heterogeneous values
- Keep handlers thin — handlers extract params and delegate to service functions
- Service functions contain business logic — no Axum types, no sqlx types
- Repository functions contain only DB queries — no business logic
- Don't add features, fallbacks, or abstractions speculatively. The system has 28
  migrations, 9 crates, and 175 tests — every addition should match that bar.

---

## Environment Variables

```bash
# Required
DATABASE_URL=postgres://user:pass@localhost:5432/irongate
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

### In Scope (all complete)
- Local auth: password, magic link, TOTP, passkeys (WebAuthn)
- Federated: Google (OIDC), GitHub (OAuth2), LDAP / Active Directory, generic OIDC
- OAuth 2.0 server: Authorization Code + PKCE, Client Credentials, Device Flow, Refresh,
  Password (gated)
- OIDC: ID Token, UserInfo, Discovery document, JWKS
- JWT: RS256 and ES256, key rotation
- Claim-based authorization (prefix per app, group + user assignments)
- Operator RBAC (admin-side permissions, separate from end-user claims)
- SCIM 2.0 (Users + Groups)
- Session management, refresh token rotation
- Audit logging
- Multi-tenancy
- Admin UI (`admin-ui/`)
- PostgreSQL + Redis deployment, Dockerfile + docker-compose

### Explicitly Out of Scope (Phase 2+)
- SAML 2.0 — deferred, ecosystem not mature enough
- Risk-based / adaptive MFA
- Webhooks for user lifecycle events
- Organization hierarchy / sub-tenants
- Verifiable Credentials (W3C)
- SQLite standalone mode (removed pre-completion in favor of single deployment target)

---

## Binary Name

The compiled binary is `irongate`. All CLI examples use this name:

```bash
irongate serve
irongate token inspect <jwt>
irongate admin init                  # bootstrap first operator
irongate tenant create acme-corp
irongate migrate run
```

---

## Repository

```
github.com/kinarix/irongate
```

---

## Working in this repo

When starting work:

1. **Read [`docs/`](docs/) first.** The claims model and operator RBAC distinction are
   the two concepts most easily misread from the code alone.
2. Use `make db-reset` to apply all migrations on a fresh DB before running tests
   that need one.
3. Run `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`
   before declaring a change done.
4. For UI changes: `cd admin-ui && npm run typecheck && npm run lint && npm run build`.
5. Don't reintroduce concepts that were explicitly removed:
   - `Role` / `Permission` for end users (use the claim model)
   - SQLite (Postgres only)
   - Compile-time `sqlx::query!` macros (use runtime `sqlx::query()`)
   - `claims_config` JSON on Application (replaced by `claim_prefix` + `ClaimDefinition`)
