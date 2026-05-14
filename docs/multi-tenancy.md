# Multi-tenancy

A **tenant** is the top-level isolation boundary in Irongate. All domain rows carry a
`tenant_id`. Repositories always filter by it. Cross-tenant reads and writes are not
exposed.

## What is scoped per tenant

| Scope | Notes |
|---|---|
| Users | Email uniqueness is per-tenant |
| Groups, group members | |
| Applications | `claim_prefix` is unique per tenant |
| Claim definitions, group/user claim assignments | |
| Identity provider configs | One Google IdP per tenant is fine |
| Signing keys | Each tenant has its own JWKS |
| Audit log | |
| Sessions, refresh tokens | |

What is **not** scoped per tenant:

- Operators (admin accounts)
- Operator permissions catalog
- Global operator roles
- The system schema itself

## How tenant is resolved on a request

| Surface | Resolution |
|---|---|
| Admin API | `tenant_id` is a path/query parameter; operator must have access |
| OAuth/OIDC | Derived from `client_id` → `applications.tenant_id` |
| SCIM | Derived from bearer token's tenant scope |
| Admin UI | Selected from the tenant dropdown; stored in React context |

The token issuer (`iss` claim) is `<BASE_URL>` and applies to all tenants; tenant
identity is conveyed via the `tenant_id` claim and via the JWKS keyset URL. This
matches how most OIDC clients expect to consume metadata while still keeping keys
isolated.

## Soft delete

Tenants are soft-deleted (`deleted_at` set). Their users and apps become inaccessible
but rows persist for forensic and audit purposes. Hard delete is a one-shot admin
script, not an API operation.

## Bootstrap tenant

`irongate admin init` creates the first operator. Tenants are created afterward via
the admin UI or `POST /admin/v1/tenants`. There is no implicit "default" tenant.

## Cross-tenant impossible-by-design

Repository methods take `tenant_id` and include it in every `WHERE` clause. A handler
that forgets to pass `tenant_id` will not compile. Integration tests create separate
tenants and assert that data inserted under one is not visible from queries scoped to
the other.

## Hostname-based routing (optional)

Irongate supports two ways for operators to address a tenant in the UI/admin layer:

- **Path-based** (default) — `https://auth.example.com/admin/...?tenant_id=…`
- **Subdomain-based** (optional) — `https://acme.auth.example.com/...` resolves to the
  `acme` tenant. Configured by setting per-tenant `hostname` and a wildcard cert.

OAuth/OIDC flows use the application's own redirect URIs and the global issuer URL;
they do not require subdomain routing.
