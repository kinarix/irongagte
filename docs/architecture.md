# Architecture

Irongate is a Cargo workspace of nine library crates plus one binary (`crates/api`).
Each crate has a single responsibility. Dependencies flow strictly inward.

## Crates

```
crates/
├── core/        # Domain types, traits, errors — no external deps
├── crypto/      # JWT, JWKS, key rotation, password hashing, PKCE
├── store/       # PostgreSQL repositories + Redis session store
├── auth/        # Password, magic link, TOTP, session services
├── webauthn/    # Passkey registration and assertion
├── federation/  # OIDC RP, OAuth2 clients, LDAP
├── authz/       # Claim resolution engine (replaces traditional RBAC)
├── scim/        # SCIM 2.0 Users + Groups
└── api/         # Axum routers, HTTP handlers, binary entry point
```

### Dependency rule

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

A crate may not depend on a crate above it in this list. The compiler enforces this.

## Domain types (`core`)

| Type | Purpose |
|---|---|
| `Tenant` | Top-level isolation boundary |
| `User` | End user account inside a tenant |
| `Group` | Collection of users; carries claim assignments and `priority` for scalar conflict resolution |
| `Application` | OAuth client; owns a unique `claim_prefix` per tenant |
| `ClaimDefinition` | `(application_id, key, claim_type)` — the schema of a custom JWT claim |
| `GroupClaim` | `(group_id, claim_def_id, value)` — group-level assignment |
| `UserClaim` | `(user_id, claim_def_id, value)` — direct user assignment |
| `Identity` | A federated identity (`provider`, `sub`) linked to a user |
| `IdpConfig` | Per-tenant identity provider configuration |
| `Session` | Browser session; stored in Redis |
| `RefreshToken` | Opaque refresh token; SHA-256 hash stored in DB |
| `AuditEvent` | Append-only audit row |
| `Operator` | System-level admin account (separate auth tree from users) |
| `OperatorRole`, `OperatorPermission`, `OperatorRoleAssignment` | Admin-side RBAC |

Note: there is no `Role` entity for end users. Role-like semantics are expressed as
claim definitions. See [claims-model.md](claims-model.md).

## Request lifecycle

```
HTTP request
    │
    ▼
Axum router (crates/api/src/router.rs)
    │
    ├─ tower-http: tracing, CORS, request-id
    ├─ middleware: auth (JWT verify), tenant resolution
    │
    ▼
Handler (crates/api/src/handlers/*)
    │
    ├─ Validates request, extracts params
    ├─ Delegates to a service in auth / federation / authz / scim
    │
    ▼
Service (business logic)
    │
    ├─ Calls repositories (store)
    │
    ▼
Repository (sqlx)
    │
    ▼
PostgreSQL / Redis
```

Handlers are thin: parse request, call service, map result to HTTP response. Services
contain business logic and do not know about Axum or HTTP. Repositories contain only
queries and do not know about business rules.

## Database access

All DB access uses runtime `sqlx::query()` and `sqlx::query_as()` — not the compile-time
`query!` macros. This means schema changes do not require `DATABASE_URL` at compile time.
The trade-off is that query errors surface at runtime; integration tests are responsible
for catching them.

Connections are pooled via `sqlx::PgPool`. Repository structs hold the pool and implement
trait objects from `core::repositories`.

## Multi-tenancy

Every domain row has a `tenant_id`. Repositories always filter by it. The tenant for a
request is resolved by:

1. Admin endpoints — `tenant_id` is a path parameter or query parameter.
2. OAuth/OIDC endpoints — derived from the application (`client_id` → `application.tenant_id`).
3. SCIM endpoints — derived from the bearer token's tenant scope.

See [multi-tenancy.md](multi-tenancy.md).

## Tokens

| Token | Format | Storage | TTL |
|---|---|---|---|
| Access token | JWT (RS256) | Stateless | 1 h (configurable) |
| ID token | JWT (RS256) | Stateless | matches access |
| Refresh token | opaque random | DB row (SHA-256 hash) | 30 d, rotated on use |
| Session | cookie ref | Redis | sliding |

Signing keys are per-tenant and rotate. JWKS is published at
`/.well-known/jwks.json` keyed by `tenant`.

## Error handling

Each crate defines its own error enum with `thiserror`. Errors propagate upward and
are mapped to HTTP responses only at the `api` boundary. Library crates never `unwrap`
or `expect`; `anyhow` is reserved for binary entry points.

## Configuration

`config/default.yaml` plus environment overrides. Required environment variables:

- `DATABASE_URL` (Postgres)
- `REDIS_URL`
- `BASE_URL` (issuer URL used in JWT `iss` and discovery)

See [development.md](development.md) for the full list.
