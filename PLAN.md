# Irongate — Full Implementation Plan

## Context

Irongate is a greenfield self-hostable IAM system in Rust. The project directory currently
contains only `SPEC.md` and `CLAUDE.md`. No code exists yet. This plan covers all 9 crates
from scratch through to a working binary, following the build order and design constraints
prescribed by CLAUDE.md. SAML is explicitly deferred to Phase 2 and is not included here.

---

## Step 0 — Repository Bootstrap

1. `git init` in `/Users/harikrishnan/Projects/kinarix/irongate`
2. Create workspace `Cargo.toml` (name = `irongate`, resolver = "2")
3. Scaffold all 9 crate directories (`crates/{core,crypto,store,auth,webauthn,federation,authz,scim,api}`)
   — each gets a minimal `Cargo.toml` + `src/lib.rs` stub with `todo!()`
4. Add `.gitignore` (target/, .env)
5. Add `config/default.yaml` skeleton
6. `cargo build` to verify workspace compiles as empty stubs
7. Initial commit: "chore: scaffold workspace"

---

## Crate 1 — `core`

**Purpose**: Pure domain layer. Zero external dependencies beyond std + serde + uuid + time + thiserror.

### Domain types to implement

| Type | Key fields |
|---|---|
| `Tenant` | id, name, slug, settings (JSON), created_at |
| `User` | id, tenant_id, email, email_verified, name, given_name, family_name, picture_url, status (enum), created_at, updated_at, last_login_at |
| `UserStatus` | `Active`, `Suspended`, `Pending` |
| `Identity` | id, user_id, provider (enum/string), provider_user_id, email, raw_claims (JSON), created_at |
| `Application` | id, tenant_id, name, client_id, client_secret_hash, app_type (enum), redirect_uris, allowed_scopes, grant_types, access_token_ttl, refresh_token_ttl |
| `AppType` | `Web`, `Spa`, `Native`, `Machine` |
| `Session` | id, user_id, tenant_id, idp_id, ip_address, user_agent, created_at, expires_at, revoked_at |
| `RefreshToken` | id, session_id, application_id, token_hash, scope, previous_id, created_at, expires_at, revoked_at |
| `Role` | id, tenant_id, name, description, parent_role_id |
| `Permission` | id, tenant_id, resource, action, description |
| `IdpConfig` | id, tenant_id, provider_type (enum), name, enabled, config (JSON) |
| `IdpType` | `Local`, `Oidc`, `Oauth2`, `Ldap` |
| `FederatedIdentity` | provider_user_id, email, email_verified, name, picture, raw_claims |
| `AuditEvent` | id, tenant_id, event_type, actor_id, target_id, ip_address, metadata, created_at |

### Traits to define

```rust
// Core repository traits (in core::repositories)
pub trait UserRepository: Send + Sync { ... }      // CRUD + find_by_email
pub trait TenantRepository: Send + Sync { ... }
pub trait ApplicationRepository: Send + Sync { ... }
pub trait SessionRepository: Send + Sync { ... }
pub trait IdentityRepository: Send + Sync { ... }
pub trait RefreshTokenRepository: Send + Sync { ... }
pub trait RoleRepository: Send + Sync { ... }
pub trait PermissionRepository: Send + Sync { ... }
pub trait AuditRepository: Send + Sync { ... }

// Identity provider trait (most important)
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn authorization_url(&self, state: &str, nonce: Option<&str>) -> Result<url::Url, IdpError>;
    async fn exchange_callback(&self, params: CallbackParams) -> Result<FederatedIdentity, IdpError>;
}

pub struct CallbackParams { pub code: String, pub state: String, pub nonce: Option<String> }
```

### Error types (one per future crate)

`CoreError`, `CryptoError`, `StoreError`, `AuthError`, `FederationError`, `AuthzError`, `ScimError`, `WebAuthnError`, `ApiError` — all using `thiserror`.

### Tests
- Roundtrip serialize/deserialize every domain type
- `UserStatus` transitions
- `IdpType` display/parse

**Commit**: "feat(core): domain types, traits, and error enums"

---

## Crate 2 — `crypto`

**Purpose**: All cryptographic primitives. Depends only on `core`.

### Modules

| Module | Responsibility |
|---|---|
| `keys` | `SigningKey` (RSA/EC), generation, rotation, expiry, `KeyPair` struct |
| `jwks` | Serialize `SigningKey` → JWKS JSON format (`JwkSet`) |
| `jwt` | Sign JWT (`encode`), verify JWT (`decode`), extract unverified header |
| `hash` | Argon2id password hashing (`hash_password`, `verify_password`) |
| `pkce` | `generate_code_verifier`, `compute_code_challenge` (S256) |
| `token` | `generate_opaque_token` (secure random, 32 bytes, base64url), `hash_token` (SHA-256) |
| `totp` | TOTP generation and verification wrapping `totp-rs` |

### Key types

```rust
pub enum SigningAlgorithm { RS256, ES256 }

pub struct KeyPair {
    pub kid: String,
    pub algorithm: SigningAlgorithm,
    pub created_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
    // private key material — never serialized
}

pub struct JwtClaims {  // generic, for both access + id tokens
    pub iss: String, pub sub: String, pub aud: Vec<String>,
    pub exp: i64, pub iat: i64, pub jti: String,
    // extra: HashMap<String, serde_json::Value>
}
```

### Tests
- Sign + verify round-trip for RS256 and ES256
- Reject expired token
- Reject wrong audience
- Argon2id: hash ≠ plaintext, verify passes, wrong password fails
- PKCE: challenge = BASE64URL(SHA-256(verifier))
- RFC test vectors for TOTP

**Commit**: "feat(crypto): JWT, JWKS, hashing, PKCE, token generation"

---

## Crate 3 — `store`

**Purpose**: All persistence. Depends on `core` + `crypto`.

### Database strategy

- Dual support via feature flags: `postgres` (default) and `sqlite` (standalone mode)
- `sqlx` compile-time verified queries (`query!` / `query_as!`)
- Migrations in `/migrations/` numbered: `0001_create_tenants.sql`, `0002_create_users.sql`, etc.
- Every table has `tenant_id`; soft-delete via `deleted_at`
- UUIDs as PK (uuid type in postgres, text in sqlite)

### Migration sequence

```
0001_create_tenants.sql
0002_create_users.sql
0003_create_identities.sql
0004_create_applications.sql
0005_create_sessions.sql
0006_create_refresh_tokens.sql
0007_create_roles.sql
0008_create_permissions.sql
0009_create_role_assignments.sql
0010_create_idp_configs.sql
0011_create_signing_keys.sql
0012_create_audit_log.sql
0013_create_mfa_credentials.sql   (TOTP secrets, backup codes)
0014_create_webauthn_credentials.sql
```

### Repository implementations

One struct per aggregate implementing the corresponding `core` trait:

`PgUserRepo`, `PgTenantRepo`, `PgApplicationRepo`, `PgSessionRepo`,
`PgIdentityRepo`, `PgRefreshTokenRepo`, `PgRoleRepo`, `PgPermissionRepo`, `PgAuditRepo`

Plus: `RedisSessionStore` for fast session lookup (backed by `fred` crate).

### Tests
- Use `sqlx::test` macro (spins up real DB, rolls back)
- CRUD round-trips per repo
- Soft delete does not return deleted records in list queries
- `find_by_email` is tenant-scoped (can't retrieve user from different tenant)

**Commit**: "feat(store): migrations, repositories (postgres + sqlite), redis session store"

---

## Crate 4 — `auth`

**Purpose**: Local authentication flows. Depends on `core`, `crypto`, `store`.

### Modules

| Module | Responsibility |
|---|---|
| `password` | Login with password: verify hash, check lockout, return session |
| `magic_link` | Generate + store one-time token, validate on click, expire after 15 min |
| `totp` | TOTP enrollment (QR URI), verification, backup code generation + use |
| `session` | Create session, validate session cookie, refresh, revoke |
| `mfa` | MFA policy enforcement: is MFA required? which factors enrolled? |

### Service functions (no Axum types)

```rust
pub async fn login_with_password(
    repos: &dyn UserRepository,
    creds: PasswordCredentials,
) -> Result<Session, AuthError>

pub async fn issue_magic_link(email: &str, tenant_id: Uuid) -> Result<(), AuthError>
pub async fn verify_magic_link(token: &str) -> Result<Session, AuthError>

pub async fn enroll_totp(user_id: Uuid) -> Result<TotpEnrollment, AuthError>
pub async fn verify_totp(user_id: Uuid, code: &str) -> Result<(), AuthError>

pub async fn validate_session(session_id: Uuid) -> Result<Session, AuthError>
pub async fn revoke_session(session_id: Uuid) -> Result<(), AuthError>
```

### Error cases to test
- Wrong password → `AuthError::InvalidCredentials`
- Account locked after 5 failures
- Expired magic link → `AuthError::TokenExpired`
- Used magic link (single-use) → `AuthError::TokenAlreadyUsed`
- Invalid TOTP code
- Valid backup code (marks it used)
- Expired session

**Commit**: "feat(auth): password login, magic link, TOTP, session management"

---

## Crate 5 — `webauthn`

**Purpose**: FIDO2/WebAuthn passkey flows. Depends on `core`, `store`.

### Flows

**Registration ceremony**:
1. `start_registration(user_id)` → `PublicKeyCredentialCreationOptions` (challenge stored in Redis)
2. `finish_registration(user_id, credential_response)` → stores `WebAuthnCredential` in DB

**Authentication ceremony**:
1. `start_authentication(user_id)` → `PublicKeyCredentialRequestOptions` (challenge stored)
2. `finish_authentication(user_id, assertion_response)` → validates, updates sign count, returns `Session`

Uses `webauthn-rs` crate. Wraps it with tenant-aware storage.

### Tests
- Mock `webauthn-rs` for unit tests
- Verify challenge mismatch is rejected
- Verify replay (same authenticator data twice) is rejected

**Commit**: "feat(webauthn): passkey registration and authentication ceremonies"

---

## Crate 6 — `federation`

**Purpose**: Federated IdP implementations. Depends on `core`, `store`, `crypto`.

### Implementations of `IdentityProvider` trait

| Struct | Protocol | Notes |
|---|---|---|
| `OidcProvider` | OIDC | Google, GitHub (with OIDC), any compliant IdP. Discovers endpoints from `/.well-known/openid-configuration` |
| `OAuth2Provider` | OAuth2 only | For providers without OIDC (plain GitHub OAuth) |
| `LdapProvider` | LDAP | Active Directory + OpenLDAP via `ldap3` |

### JIT Provisioning (`jit.rs`)

```rust
pub async fn provision_or_link_user(
    identity: FederatedIdentity,
    tenant_id: Uuid,
    link_policy: AccountLinkPolicy,
    repos: &dyn UserRepository,
) -> Result<User, FederationError>
```

Policies: `AutoLink`, `Prompt`, `Reject`.

### IdP registry

```rust
pub struct IdpRegistry {
    providers: HashMap<String, Box<dyn IdentityProvider>>,
}
impl IdpRegistry {
    pub fn get(&self, id: &str) -> Option<&dyn IdentityProvider>
    pub fn register(&mut self, provider: Box<dyn IdentityProvider>)
}
```

### Tests
- Mock HTTP for OIDC token exchange (test against fixture responses)
- JIT: new user gets created with default roles
- JIT: existing user by email gets linked (auto-link policy)
- JIT: conflict on reject policy returns error

**Commit**: "feat(federation): OIDC, OAuth2, LDAP providers, JIT provisioning"

---

## Crate 7 — `authz`

**Purpose**: RBAC + basic ABAC authorization engine. Depends on `core`, `store`.

### RBAC engine

```rust
pub async fn get_user_permissions(
    user_id: Uuid,
    tenant_id: Uuid,
    repos: &dyn RoleRepository,
) -> Result<Vec<Permission>, AuthzError>
```

- Resolves role hierarchy (parent roles)
- Merges direct + role-derived permissions
- De-duplicates

### ABAC policy evaluation

Simple policy DSL (not OPA). Policies stored as JSON rules.

```rust
pub struct PolicyContext {
    pub principal: PrincipalAttrs,   // { roles, user_id, department }
    pub resource: ResourceAttrs,     // { type, owner_id, status }
    pub action: String,
    pub environment: EnvAttrs,       // { ip, time }
}

pub fn evaluate_policy(policy: &Policy, ctx: &PolicyContext) -> Decision
// Decision: Allow | Deny
```

### Scope resolution

```rust
pub fn resolve_scopes(
    requested: &[String],
    user_permissions: &[Permission],
) -> Vec<String>   // intersection of requested + what user actually has
```

### Tests
- Role hierarchy: admin inherits from editor
- Direct permission overrides role permission
- Policy evaluation: allow/deny scenarios
- Scope resolution: requested scope not in permissions → excluded from token

**Commit**: "feat(authz): RBAC engine, ABAC policy evaluation, scope resolution"

---

## Crate 8 — `scim`

**Purpose**: SCIM 2.0 REST API for automated provisioning. Depends on `core`, `store`.

### Endpoints (as Axum Router — exported, mounted by `api` crate)

```
GET    /scim/v2/Users           → list with filter
POST   /scim/v2/Users           → create
GET    /scim/v2/Users/:id       → get
PUT    /scim/v2/Users/:id       → replace
PATCH  /scim/v2/Users/:id       → patch (SCIM patch operations)
DELETE /scim/v2/Users/:id       → deactivate (soft delete)

GET    /scim/v2/Groups          → list
POST   /scim/v2/Groups          → create
PATCH  /scim/v2/Groups/:id      → update members
```

### SCIM schema compliance

- `ListResponse` with `totalResults`, `startIndex`, `itemsPerPage`, `Resources`
- SCIM `User` resource mapped to/from `core::User`
- `PATCH` operations: `add`, `remove`, `replace`
- Filter parsing: `eq`, `co`, `sw`, `and`
- Error format: `{ "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"], "status": "404" }`

### Tests
- Create user via SCIM → appears in user list
- Deactivate → status = suspended
- PATCH add group member
- Filter: `userName eq "alice"` returns correct user

**Commit**: "feat(scim): SCIM 2.0 Users and Groups endpoints"

---

## Crate 9 — `api`

**Purpose**: Axum application wiring everything together. Depends on all other crates.

### Router structure

```
/                           → health / readiness
/.well-known/
  openid-configuration      → OIDC discovery
  jwks.json                 → public keys
/oauth2/
  authorize                 → authorization endpoint
  token                     → token endpoint
  introspect                → RFC 7662
  revoke                    → RFC 7009
  userinfo                  → claims
/device/
  code                      → device authorization flow
/api/v1/
  users/                    → management API
  groups/
  roles/
  applications/
  idps/
  sessions/
  audit-log/
/scim/v2/                   → mounted from scim crate
```

### Middleware stack (tower layers, outermost first)

1. `TraceLayer` (tracing + request IDs)
2. Rate limiting (per-IP, per-tenant)
3. Tenant resolution (from hostname or `X-Tenant-Id` header)
4. Authentication (bearer token or session cookie)
5. CORS

### OAuth 2.0 server logic (in `api::oauth`)

- `GET /oauth2/authorize` — validate params, check client, render login redirect or login page
- `POST /oauth2/token` — dispatch to `authorization_code`, `client_credentials`, `refresh_token`, `device_code` handlers
- Authorization code storage: Redis with 10-minute TTL
- PKCE: S256 required for public clients, verified on code exchange

### Token issuance

```rust
// in api::tokens
pub fn issue_access_token(user: &User, app: &Application, scopes: &[String], ...) -> String
pub fn issue_id_token(user: &User, app: &Application, nonce: Option<&str>, ...) -> String
pub fn issue_refresh_token(session_id: Uuid, app_id: Uuid, ...) -> (String, RefreshToken)
```

### State / AppState

```rust
pub struct AppState {
    pub user_repo: Arc<dyn UserRepository>,
    pub tenant_repo: Arc<dyn TenantRepository>,
    pub session_repo: Arc<dyn SessionRepository>,
    // ... all repos
    pub idp_registry: Arc<IdpRegistry>,
    pub key_store: Arc<KeyStore>,
    pub config: Arc<Config>,
}
```

### Integration tests (`tests/`)

- Full authorization code + PKCE flow (in-process via `axum::test`)
- Client credentials flow
- Refresh token rotation
- Revoked refresh token rejected
- JWKS endpoint returns valid keys
- Discovery document contains correct URLs

**Commit**: "feat(api): Axum application, OAuth2/OIDC endpoints, management API"

---

## Cross-cutting: `main.rs` (binary)

Lives at `crates/api/src/main.rs` or a top-level `src/main.rs` depending on workspace layout.
Recommended: top-level binary in `src/main.rs` that imports `api`.

CLI subcommands (using `clap`):
```
irongate serve
irongate migrate run
irongate token inspect <jwt>
irongate tenant create <slug>
irongate user suspend <email>
```

---

## Deployment / Config

- `config/default.yaml` — all defaults
- Env vars override config (via `config` crate)
- `DATABASE_URL`, `REDIS_URL`, `BASE_URL` required at start

---

## Testing Strategy (summary)

| Layer | Tool | Scope |
|---|---|---|
| Unit | `#[cfg(test)]` in-module | Pure logic, no I/O |
| DB integration | `sqlx::test` | Real DB, auto-rollback |
| API integration | `axum::test` + `tower::ServiceExt` | No real HTTP server |
| Crypto | RFC test vectors | JWT, PKCE, TOTP |
| Mock traits | `mockall` | Isolate service functions |

---

## Verification (per crate)

After each crate:
1. `cargo test -p <crate>` — all tests pass
2. `cargo clippy -p <crate> -- -D warnings` — zero warnings
3. `cargo fmt --check` — formatted
4. Commit

After all crates:
1. `cargo build --release` — binary compiles
2. Integration test suite passes
3. `irongate serve` starts and responds to `GET /health`
4. Full OAuth2 authorization code flow exercised end-to-end

---

## Files to create (in order)

```
Cargo.toml                          workspace root
.gitignore
config/default.yaml
migrations/0001_create_tenants.sql
migrations/... (14 files total)
crates/core/Cargo.toml + src/
crates/crypto/Cargo.toml + src/
crates/store/Cargo.toml + src/
crates/auth/Cargo.toml + src/
crates/webauthn/Cargo.toml + src/
crates/federation/Cargo.toml + src/
crates/authz/Cargo.toml + src/
crates/scim/Cargo.toml + src/
crates/api/Cargo.toml + src/
src/main.rs                         binary entry point
```
