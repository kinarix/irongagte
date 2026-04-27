# Identity System Specification
**Version:** 1.0  
**Status:** Draft

---

## 1. Overview

This document specifies a full-featured, self-hostable Identity and Access Management (IAM) system. It supports federated authentication via external providers (Google, GitHub, SAML, etc.), local credential management, and fine-grained authorization — all deployable on-premise or in the cloud.

### Goals

- Single source of truth for user identity across all services
- Support both federated (external provider) and local (self-hosted) authentication
- Flexible authorization model (RBAC and ABAC)
- Standards-compliant (OAuth 2.0, OIDC, SAML 2.0, SCIM)
- Self-hostable with no external dependencies required
- Multi-tenant capable

### Non-Goals

- Not a full SSO proxy (though it can act as one)
- Not a secrets manager (though it manages signing keys)
- Not a user-facing product UI (admin UI is a separate concern)

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Identity System                          │
│                                                                 │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────────┐  │
│  │    Auth      │  │  Federation  │  │    Authorization      │  │
│  │   Server     │  │   Gateway    │  │      Engine           │  │
│  │  (OIDC/OAuth)│  │ (IdP Bridge) │  │  (RBAC / ABAC)        │  │
│  └──────┬───────┘  └──────┬───────┘  └───────────┬───────────┘  │
│         │                 │                       │              │
│  ┌──────▼─────────────────▼───────────────────────▼───────────┐ │
│  │                      Core Services                         │ │
│  │   User Store │ Session Store │ Token Store │ Audit Log      │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                     APIs                                │   │
│  │   Management API │ SCIM API │ JWKS Endpoint │ Admin API  │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
         ↕                     ↕                    ↕
   Your Applications     External IdPs         Admin Tooling
   (Resource Servers)   (Google, SAML...)      (Dashboard, CLI)
```

---

## 3. Core Concepts

### 3.1 Tenants

The system is multi-tenant. Each tenant is an isolated namespace with its own:
- Users and groups
- Applications (OAuth clients)
- Identity providers
- Roles and permissions
- Signing keys
- Branding

```
Tenant: acme-corp
  ├── Users
  ├── Groups
  ├── Applications
  │     ├── web-app (client_id: abc)
  │     └── mobile-app (client_id: xyz)
  ├── Identity Providers
  │     ├── Google (federated)
  │     └── SAML (corporate IdP)
  └── Roles / Permissions
```

### 3.2 Identity Providers (IdPs)

An IdP is a source of user identity. The system supports two categories:

**Local IdP** — the system itself manages credentials  
**Federated IdP** — an external system authenticates the user (Google, GitHub, SAML, LDAP)

### 3.3 Applications (OAuth Clients)

Any service that wants to authenticate users registers as an Application. Each application has:
- `client_id` — public identifier
- `client_secret` — for confidential clients (server-side apps)
- Allowed redirect URIs
- Allowed scopes
- Grant types permitted
- Token lifetime configuration

### 3.4 Users

A user is a unique identity within a tenant. Users have:
- A stable internal `id` (UUID)
- One or more linked identity sources (local credentials, Google, SAML, etc.)
- Profile attributes (email, name, etc.)
- Group memberships
- Direct role assignments
- Status (active, suspended, pending)

### 3.5 Sessions

Sessions track an authenticated user across requests. Sessions have:
- A session ID stored in a secure, httpOnly cookie
- Reference to the user and the IdP used
- Creation and expiry timestamps
- Device/IP metadata for anomaly detection

---

## 4. Authentication

### 4.1 Local Authentication

The system acts as its own IdP. Supports:

#### Username / Password
- Passwords hashed using Argon2id
- Configurable complexity requirements
- Brute-force protection (lockout after N failed attempts)
- Breach detection (check against HaveIBeenPwned API — optional)

#### Passwordless (Magic Link)
- User enters email → receives a one-time link
- Link is single-use and expires in 15 minutes
- Delivered via email (SMTP configurable)

#### TOTP (Time-based One-Time Password)
- RFC 6238 compliant
- QR code enrollment flow
- Backup codes (8 single-use codes generated at enrollment)

#### Passkeys (WebAuthn)
- FIDO2 compliant
- Supports platform authenticators (Face ID, Windows Hello) and roaming authenticators (YubiKey)
- Replaces password entirely for enrolled users

#### MFA Policy
- Configurable per tenant, per application, or per user group
- Step-up authentication (require MFA only for sensitive operations)
- MFA enrollment grace period (N days before enforcement)

---

### 4.2 Federated Authentication

The system acts as an OIDC Relying Party (RP) or SAML Service Provider (SP) to external IdPs.

#### Supported Protocols

| Protocol | Use Case |
|---|---|
| **OIDC** | Google, GitHub, Azure AD, Okta, any OIDC-compliant IdP |
| **SAML 2.0** | Enterprise corporate IdPs (Okta, ADFS, PingIdentity) |
| **OAuth 2.0** | Providers that don't implement OIDC (GitHub, Twitter) |
| **LDAP / AD** | On-premise Active Directory or OpenLDAP |

#### Federation Flow (OIDC)

```
User → Login Page → Selects "Sign in with Google"
      ↓
Identity System redirects to Google (as RP)
      ↓
Google authenticates user, returns ID Token + Auth Code
      ↓
Identity System validates ID Token
      ↓
Identity System looks up or provisions local user
      ↓
Identity System issues its own tokens to the Application
```

The application only ever sees tokens issued by the Identity System — it is shielded from the complexity of the underlying IdP.

#### Account Linking

When a federated user's email matches an existing local user:

- **Auto-link**: Automatically merge identities (configurable)
- **Prompt**: Ask user to confirm they want to link accounts
- **Reject**: Require explicit account linking via settings page
- **Conflict**: If email is verified by both sides, auto-link is safe

Each user can have multiple linked identities:

```
User: alice@company.com
  ├── Local credential (password)
  ├── Google: sub=google-12345
  └── GitHub: sub=github-67890
```

#### SAML 2.0 Configuration

Per-tenant SAML SP configuration:
- SP Entity ID (auto-generated, downloadable as metadata XML)
- ACS (Assertion Consumer Service) URL
- SLO (Single Logout) URL
- Attribute mappings (SAML attributes → user profile fields)
- NameID format (email, persistent, transient)
- Signature verification (IdP certificate upload)

#### JIT Provisioning (Just-in-Time)

On first federated login, if the user doesn't exist:
1. Create user record from IdP-provided claims
2. Assign default role(s) based on tenant policy
3. Optionally assign to groups based on IdP group claims (SAML groups, Google Workspace groups)
4. Send welcome email (configurable)

---

### 4.3 Authentication Flows (OAuth 2.0)

#### Authorization Code Flow (Web Apps — Default)

```
App → /authorize (response_type=code) → Login → /token (code exchange) → Tokens
```

With PKCE (Proof Key for Code Exchange) — required for public clients (SPAs, mobile).

#### Client Credentials Flow (Machine-to-Machine)

```
Service → POST /token (client_id + client_secret) → Access Token
```

No user involved. Used for backend services calling APIs.

#### Device Authorization Flow (CLI / TV Apps)

```
Device → /device/code → Show code to user → User visits URL on another device
→ User approves → Device polls /token → Tokens issued
```

#### Refresh Token Flow

```
Client → POST /token (grant_type=refresh_token) → New Access Token
```

Refresh tokens are:
- Rotated on each use (old token invalidated)
- Absolute expiry (e.g., 30 days, regardless of activity)
- Revocable via management API or user session management UI

---

## 5. Token Architecture

### 5.1 ID Token

Always a signed JWT (JWS). Addressed to the Application.

```json
{
  "iss": "https://auth.yourcompany.com",
  "sub": "usr_01HXYZ",
  "aud": "client_abc123",
  "exp": 1714003600,
  "iat": 1714000000,
  "auth_time": 1714000000,
  "nonce": "abc",
  "email": "alice@company.com",
  "email_verified": true,
  "name": "Alice Smith",
  "given_name": "Alice",
  "family_name": "Smith",
  "picture": "https://...",
  "amr": ["pwd", "totp"],
  "acr": "urn:mfa"
}
```

Key claims:
- `amr` — Authentication Methods References (what factors were used)
- `acr` — Authentication Context Class Reference (assurance level)
- `auth_time` — when the user actually authenticated (important for step-up flows)

### 5.2 Access Token

JWT issued to callers of protected APIs (Resource Servers). Follows RFC 9068.

```json
{
  "iss": "https://auth.yourcompany.com",
  "sub": "usr_01HXYZ",
  "aud": "https://api.yourapp.com",
  "exp": 1714003600,
  "iat": 1714000000,
  "jti": "tok_01HABC",
  "client_id": "client_abc123",
  "scope": "read:reports write:reports",
  "roles": ["editor"],
  "permissions": ["read:reports", "write:reports"]
}
```

- Short-lived (default: 1 hour, configurable per application)
- Includes roles and permissions for the Resource Server to enforce locally
- `jti` enables token revocation checking

### 5.3 Refresh Token

Opaque token (not a JWT). Stored server-side, referenced by ID. Enables:
- Rotation on every use
- Immediate revocation
- Device-level session tracking

### 5.4 Signing Keys

- RS256 (RSA + SHA-256) — default, asymmetric, widely supported
- ES256 (ECDSA + SHA-256) — smaller tokens, also supported
- Keys published at `/.well-known/jwks.json`
- Key rotation supported: new key added to JWKS, old key retained for token lifetime before removal
- Keys stored encrypted at rest (AES-256)

---

## 6. Authorization

### 6.1 Model

The system supports both RBAC and ABAC, composable:

```
User → has Roles → Roles have Permissions
User → has direct Permissions (override)
Permissions → evaluated against Resource + Context (ABAC)
```

### 6.2 RBAC (Role-Based Access Control)

```
Roles:
  admin       → [read:*, write:*, delete:*, manage:users]
  editor      → [read:*, write:reports]
  viewer      → [read:reports]
  billing     → [read:invoices, write:invoices]

User Assignments:
  alice → [admin]
  bob   → [editor, billing]
  carol → [viewer]
```

- Roles are tenant-scoped
- Roles can inherit from other roles (role hierarchy)
- Applications can define their own scopes; roles map to scopes

### 6.3 ABAC (Attribute-Based Access Control)

Policies are evaluated against a context object:

```
Principal:  { userId, roles, department, location }
Resource:   { type, ownerId, status, classification }
Action:     { type: "delete" }
Environment:{ time, ip, deviceTrusted }

Policy:
  ALLOW delete IF
    principal.roles includes "admin"
    OR (principal.userId == resource.ownerId AND resource.status != "archived")
```

Policies expressed in a simple policy language (OPA/Rego compatible or custom DSL).

### 6.4 Scopes

OAuth scopes map to permissions:

```
Scope               →  Permission
read:reports        →  Can read reports
write:reports       →  Can create/update reports
admin:users         →  Can manage users
openid              →  (OIDC) Include ID Token
profile             →  Include name, picture
email               →  Include email
```

Applications request scopes. Users consent. The access token only includes granted scopes.

### 6.5 Token-based Permission Propagation

Access tokens include roles and permissions so Resource Servers can enforce locally:

```javascript
// Resource Server (your API)
const token = verifyJWT(req.headers.authorization, JWKS_URI)

// Option A: Scope check
if (!token.scope.includes('write:reports')) return 403

// Option B: Permission check (embedded in token)
if (!token.permissions.includes('write:reports')) return 403

// Option C: Call authorization endpoint (for dynamic policies)
const allowed = await authz.check({
  principal: token.sub,
  resource: { type: 'report', id: req.params.id },
  action: 'write'
})
```

---

## 7. API Specification

### 7.1 OIDC Discovery

```
GET /.well-known/openid-configuration
```

Returns all endpoints, supported scopes, claims, signing algorithms.

### 7.2 Authorization Endpoint

```
GET /oauth2/authorize
  ?response_type=code
  &client_id=...
  &redirect_uri=...
  &scope=openid email profile
  &state=...
  &code_challenge=...         (PKCE)
  &code_challenge_method=S256 (PKCE)
  &nonce=...
  &prompt=login|consent|none
  &login_hint=alice@company.com
  &acr_values=urn:mfa        (require MFA)
```

### 7.3 Token Endpoint

```
POST /oauth2/token
Content-Type: application/x-www-form-urlencoded

# Authorization Code
grant_type=authorization_code
&code=...
&redirect_uri=...
&client_id=...
&code_verifier=...  (PKCE)

# Client Credentials
grant_type=client_credentials
&client_id=...
&client_secret=...
&scope=...

# Refresh Token
grant_type=refresh_token
&refresh_token=...
&client_id=...
```

Response:
```json
{
  "access_token": "...",
  "id_token": "...",
  "refresh_token": "...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "scope": "openid email profile"
}
```

### 7.4 Token Introspection (RFC 7662)

```
POST /oauth2/introspect
Authorization: Basic <client credentials>

token=...
```

Response:
```json
{
  "active": true,
  "sub": "usr_01HXYZ",
  "scope": "read:reports",
  "exp": 1714003600,
  "client_id": "client_abc"
}
```

### 7.5 Token Revocation (RFC 7009)

```
POST /oauth2/revoke

token=...
token_type_hint=refresh_token
```

### 7.6 UserInfo Endpoint

```
GET /oauth2/userinfo
Authorization: Bearer <access_token>
```

Returns claims about the authenticated user based on granted scopes.

### 7.7 JWKS Endpoint

```
GET /.well-known/jwks.json
```

Public keys used to verify tokens. Cached by Resource Servers (with TTL respect).

### 7.8 Management API (REST)

Protected by an admin access token (client credentials of a privileged client).

```
# Users
GET    /api/v1/users
POST   /api/v1/users
GET    /api/v1/users/:id
PATCH  /api/v1/users/:id
DELETE /api/v1/users/:id
POST   /api/v1/users/:id/suspend
POST   /api/v1/users/:id/unsuspend

# Groups
GET    /api/v1/groups
POST   /api/v1/groups
POST   /api/v1/groups/:id/members

# Roles
GET    /api/v1/roles
POST   /api/v1/roles
POST   /api/v1/users/:id/roles

# Applications (OAuth Clients)
GET    /api/v1/applications
POST   /api/v1/applications
PATCH  /api/v1/applications/:id
POST   /api/v1/applications/:id/rotate-secret

# Identity Providers
GET    /api/v1/idps
POST   /api/v1/idps
PATCH  /api/v1/idps/:id

# Sessions
GET    /api/v1/users/:id/sessions
DELETE /api/v1/users/:id/sessions        (revoke all)
DELETE /api/v1/sessions/:id             (revoke one)

# Audit Log
GET    /api/v1/audit-log
```

### 7.9 SCIM 2.0 API

For automated user provisioning/deprovisioning from HR systems, Okta, Azure AD, etc.

```
GET    /scim/v2/Users
POST   /scim/v2/Users
GET    /scim/v2/Users/:id
PUT    /scim/v2/Users/:id
PATCH  /scim/v2/Users/:id
DELETE /scim/v2/Users/:id

GET    /scim/v2/Groups
POST   /scim/v2/Groups
PATCH  /scim/v2/Groups/:id
```

Supported operations:
- Create user on hire
- Deactivate user on termination
- Sync group memberships
- Update profile attributes

---

## 8. Security Requirements

### 8.1 Transport
- TLS 1.2 minimum, TLS 1.3 preferred
- HSTS enforced
- Certificate pinning for mobile SDK (optional)

### 8.2 Token Security
- Short-lived access tokens (1 hour default)
- Refresh token rotation on every use
- Token binding (optional, DPoP — RFC 9449)
- `jti` claim for replay prevention
- Audience validation enforced at Resource Server

### 8.3 CSRF Protection
- `state` parameter validated on all authorization code returns
- SameSite=Strict cookies for session
- PKCE required for all public clients

### 8.4 Brute Force Protection
- Login rate limiting per IP and per account
- Exponential backoff after failed attempts
- Account lockout after N failures (configurable)
- CAPTCHA integration (reCAPTCHA v3 or hCaptcha)

### 8.5 Key Management
- Signing keys rotated on configurable schedule (default: 90 days)
- Old keys retained until all issued tokens expire
- Keys never logged or included in error responses
- HSM support for key storage (optional)

### 8.6 Audit Logging

Every security-relevant event is logged:

| Event | Data Captured |
|---|---|
| Login success/failure | User, IP, IdP, timestamp, user-agent |
| Token issued | Token ID, client, scopes, user |
| Token revoked | Token ID, reason, actor |
| Password change | User, IP, timestamp |
| MFA enrolled/removed | User, method, actor |
| User created/suspended | User, actor |
| Role assigned/removed | User, role, actor |
| IdP added/modified | IdP ID, actor |
| Admin API access | Endpoint, actor, status |

Audit logs are:
- Immutable (append-only)
- Exportable (JSON, CSV)
- Queryable via API
- Retained for configurable period (default: 1 year)

---

## 9. Self-Hosting

### 9.1 Deployment Modes

| Mode | Description |
|---|---|
| **Standalone** | Single binary, SQLite, suitable for small deployments |
| **Distributed** | Multiple instances behind a load balancer, shared DB + Redis |
| **Kubernetes** | Helm chart provided, horizontal scaling |

### 9.2 Dependencies

| Component | Standalone | Distributed |
|---|---|---|
| Database | SQLite (embedded) | PostgreSQL 14+ |
| Cache / Sessions | In-memory | Redis 7+ |
| Email | SMTP | SMTP or SES/SendGrid |
| Storage (avatars, etc.) | Local disk | S3-compatible |

### 9.3 Configuration

All configuration via environment variables or a YAML config file:

```yaml
server:
  host: 0.0.0.0
  port: 8080
  base_url: https://auth.yourcompany.com

database:
  url: postgres://user:pass@host:5432/identity

redis:
  url: redis://localhost:6379

signing:
  algorithm: RS256
  rotation_days: 90

tenants:
  default: acme-corp

email:
  smtp_host: smtp.yourcompany.com
  smtp_port: 587
  from: noreply@yourcompany.com

mfa:
  totp_enabled: true
  webauthn_enabled: true
  required: false   # set true to enforce globally
```

### 9.4 High Availability

- All instances are stateless (session state in Redis)
- DB read replicas supported for scaling reads
- Health check endpoint: `GET /health`
- Readiness check endpoint: `GET /ready`
- Graceful shutdown with in-flight request draining

### 9.5 Self-hosted vs Cloud-hosted Tradeoffs

| Concern | Self-hosted | Cloud-hosted (SaaS) |
|---|---|---|
| Data residency | Full control | Depends on provider |
| Maintenance | You own upgrades | Managed |
| Cost | Infrastructure cost | Subscription |
| Compliance | Easier for strict regimes | Provider certifications |
| Availability | You own uptime | Provider SLA |

---

## 10. Data Model

### Users Table

```
users
  id              UUID PK
  tenant_id       UUID FK
  email           TEXT UNIQUE (within tenant)
  email_verified  BOOLEAN
  name            TEXT
  given_name      TEXT
  family_name     TEXT
  picture_url     TEXT
  status          ENUM(active, suspended, pending)
  created_at      TIMESTAMP
  updated_at      TIMESTAMP
  last_login_at   TIMESTAMP
```

### Identities Table (linked IdP accounts)

```
identities
  id              UUID PK
  user_id         UUID FK → users.id
  provider        TEXT (local, google, github, saml:idp-id)
  provider_user_id TEXT
  email           TEXT
  data            JSONB (raw claims from IdP)
  created_at      TIMESTAMP
```

### Applications Table

```
applications
  id              UUID PK
  tenant_id       UUID FK
  name            TEXT
  client_id       TEXT UNIQUE
  client_secret   TEXT (hashed)
  type            ENUM(web, spa, native, machine)
  redirect_uris   TEXT[]
  allowed_scopes  TEXT[]
  grant_types     TEXT[]
  access_token_ttl  INTEGER (seconds)
  refresh_token_ttl INTEGER (seconds)
```

### Sessions Table

```
sessions
  id              UUID PK
  user_id         UUID FK
  tenant_id       UUID FK
  idp_id          TEXT
  ip_address      TEXT
  user_agent      TEXT
  created_at      TIMESTAMP
  expires_at      TIMESTAMP
  revoked_at      TIMESTAMP
```

### Tokens Table (Refresh Tokens)

```
refresh_tokens
  id              UUID PK
  session_id      UUID FK
  application_id  UUID FK
  token_hash      TEXT (SHA-256 of opaque token)
  scope           TEXT
  previous_id     UUID (for rotation chain)
  created_at      TIMESTAMP
  expires_at      TIMESTAMP
  revoked_at      TIMESTAMP
```

---

## 11. SDK Requirements

Client SDKs to be provided for:
- **JavaScript/TypeScript** (browser + Node.js)
- **React** (hooks: `useAuth`, `useUser`, `withAuth` HOC)
- **Python**
- **Go**
- **Java / Kotlin**

Each SDK must handle:
- Authorization Code + PKCE flow
- Token storage (secure, not localStorage)
- Silent token refresh
- Logout (local + server-side session termination)
- Exposing user profile and token claims

---

## 12. Admin Dashboard (UI)

A web-based admin interface for tenant administrators:

**Users**
- List, search, filter users
- View linked identities, sessions, roles
- Suspend / unsuspend / delete
- Impersonate (for support — with audit log)
- Force MFA enrollment

**Applications**
- Register and manage OAuth clients
- View token usage
- Rotate secrets

**Identity Providers**
- Configure Google, GitHub, SAML, LDAP
- Test federation before enabling
- Map attributes to user profile fields

**Roles & Permissions**
- Create and manage roles
- Assign permissions
- Bulk role assignment via groups

**Audit Log**
- Search and filter log
- Export to CSV / JSON

**Tenant Settings**
- MFA policy
- Session lifetime
- Allowed email domains
- Branding (logo, colors for hosted login page)

---

## 13. Hosted Login Page

A customizable, hosted login UI served by the Identity System:

- Tenant-branded (logo, colors, custom domain)
- Responsive (mobile-first)
- Shows only IdPs configured for the tenant
- Handles all MFA enrollment and verification flows
- Supports `login_hint` to skip IdP selection
- Localization (i18n) support
- Custom CSS injection (for advanced branding)

---

## 14. Compliance Considerations

| Requirement | How Addressed |
|---|---|
| GDPR | User data export API, right-to-erasure endpoint |
| SOC 2 | Audit logs, encryption at rest, access controls |
| HIPAA | Configurable data retention, encryption, audit |
| CCPA | Data export and deletion APIs |
| FIDO2 | WebAuthn support |
| NIST 800-63 | Password policy, MFA assurance levels, audit |

---

## 15. Open Questions / Future Considerations

- **Organization hierarchy**: Sub-tenants or org units within a tenant
- **Delegated administration**: Org admins managing a subset of users
- **Risk-based authentication**: Step-up MFA triggered by anomalous login signals
- **Verifiable Credentials (W3C VC)**: Decentralized identity support
- **Rate limiting**: Per-tenant and per-application token rate limits
- **Webhooks**: Event-driven notifications for user lifecycle events (created, suspended, etc.)
