# OAuth 2.0 / OIDC endpoints

Irongate is an OIDC-compliant Provider. Apps integrate using standard OIDC discovery —
point the client library at `<BASE_URL>` and it will fetch the discovery document.

## Discovery

- `GET /.well-known/openid-configuration` — issuer metadata, supported flows, endpoint URLs
- `GET /.well-known/jwks.json` — current and rotating signing keys for the tenant

## Authorization endpoints

| Endpoint | Purpose |
|---|---|
| `GET /oauth2/authorize` | Authorization Code flow start (with PKCE) |
| `POST /oauth2/token` | Token exchange (code, refresh, client_credentials, password) |
| `POST /oauth2/introspect` | Token introspection (RFC 7662) |
| `POST /oauth2/revoke` | Token revocation (RFC 7009) |
| `GET /oauth2/userinfo` | UserInfo (OIDC) |

## Grant types

| Grant | Notes |
|---|---|
| `authorization_code` | Web/SPA/mobile flows; PKCE required for public clients |
| `refresh_token` | Rotates the refresh token on use; previous token is revoked |
| `client_credentials` | Machine-to-machine; no user context |
| `password` | Resource Owner Password Credentials; gated to trusted clients only |
| Device flow | `POST /oauth2/device_authorization` + polling on `/token` |

## Tokens

### Access token

JWT signed RS256. Claims include `iss`, `aud`, `sub`, `exp`, `iat`, `jti`, `scope`,
`tenant_id`, plus all custom claims resolved per the [claims model](claims-model.md).

### ID token

JWT signed RS256. Claims include `iss`, `aud` (the client), `sub`, `exp`, `iat`,
`nonce` (if requested), standard OIDC user claims (`email`, `name`, etc.), plus
custom claims resolved per the [claims model](claims-model.md).

### Refresh token

Opaque random string. The DB stores its SHA-256 hash, never the plaintext. Rotated
on each use; the old token is revoked immediately. TTL is configurable (default 30 d).

## Standard OIDC claims

These come from the user record and are emitted unprefixed at canonical OIDC keys:

`sub`, `email`, `email_verified`, `name`, `given_name`, `family_name`, `picture`,
`locale`, `updated_at`.

Custom claims live under `<app.claim_prefix>:<key>`. See [claims-model.md](claims-model.md).

## Scopes

`openid`, `profile`, `email` behave per OIDC core. Additional custom scopes can be
configured per application; granted scopes appear in the `scope` claim of the access
token and are surfaced to consent screens.

## PKCE

Required for public clients (no client secret). Code verifier and challenge follow
RFC 7636 with `S256`. The `crypto` crate generates verifiers and validates
challenges in constant time.

## Federation (IdP login)

For federated sign-in, the authorization endpoint redirects the user to the configured
IdP's authorization URL. On callback (`/oauth2/callback`), Irongate exchanges the code,
loads the federated identity, JIT-provisions or links the user, mints an Irongate
session, and resumes the original OAuth flow.

Configured IdPs (per tenant):

- Google (OIDC)
- GitHub (OAuth2)
- LDAP / Active Directory
- Generic OIDC

Each implements the `core::IdentityProvider` trait. SAML is Phase 2.

## Handler files

- `crates/api/src/handlers/authorize.rs` — `/oauth2/authorize` + login form
- `crates/api/src/handlers/token.rs` — `/oauth2/token`, claim resolution
- `crates/api/src/handlers/oidc.rs` — discovery, JWKS, userinfo
