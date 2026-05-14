# Admin API

The admin/management API powers the `admin-ui` and any custom tooling you wire up.
All endpoints require an authenticated operator session (cookie set by
`/admin/auth/login`). Authorization is checked per-handler against the operator's
effective permissions — see [operator-rbac.md](operator-rbac.md).

Base path: `/admin/v1` (subject to change before 1.0).

## Tenants

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/tenants` | List |
| `POST` | `/tenants` | Create |
| `GET` | `/tenants/{id}` | Get |
| `PATCH` | `/tenants/{id}` | Update |
| `DELETE` | `/tenants/{id}` | Soft delete |

Handler: `admin_tenants.rs`.

## Users

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/users?tenant_id=` | List, paginated |
| `POST` | `/users` | Create |
| `GET` | `/users/{id}` | Get |
| `PATCH` | `/users/{id}` | Update (incl. `attributes`) |
| `DELETE` | `/users/{id}` | Soft delete |
| `POST` | `/users/import` | Bulk import (CSV/JSON); optional `group_id` |

Handler: `admin_users.rs`. The bulk import flow adds imported users to the supplied
group so they inherit its claims (see [claims-model.md](claims-model.md)).

## Applications

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/applications?tenant_id=` | List |
| `POST` | `/applications` | Create (requires `claim_prefix`) |
| `GET` | `/applications/{id}` | Get |
| `PATCH` | `/applications/{id}` | Update |
| `DELETE` | `/applications/{id}` | Soft delete |

Handler: `admin_applications.rs`. The `claim_prefix` field is validated against the
OIDC reserved-name list and must be unique per tenant.

## Groups

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/groups?tenant_id=` | List |
| `POST` | `/groups` | Create |
| `GET` | `/groups/{id}` | Get (with members + claim assignments) |
| `PATCH` | `/groups/{id}` | Update (incl. `priority`) |
| `DELETE` | `/groups/{id}` | Soft delete |
| `POST/DELETE` | `/groups/{id}/members` | Add/remove members |

Handler: `admin_groups.rs`.

## Claims

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/claims/definitions?tenant_id=` | List all claim definitions in the tenant |
| `POST` | `/claims/definitions` | Create |
| `GET` | `/claims/definitions/{id}` | Get |
| `PATCH` | `/claims/definitions/{id}` | Update |
| `DELETE` | `/claims/definitions/{id}` | Delete |
| `POST` | `/claims/group-assignments` | Assign `(group_id, claim_def_id, value)` |
| `DELETE` | `/claims/group-assignments` | Revoke a group assignment |
| `POST` | `/claims/user-assignments` | Assign `(user_id, claim_def_id, value)` |
| `DELETE` | `/claims/user-assignments` | Revoke a user assignment |
| `GET` | `/claims/effective?tenant_id=&user_id=&application_id=` | Preview effective claims |

Handler: `admin_claims.rs`. The `/claims/effective` endpoint runs the full token-mint
claim resolution and returns the resulting object — useful for the user-detail page in
the admin UI.

## Identity providers

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/idp/configs?tenant_id=` | List |
| `POST` | `/idp/configs` | Create |
| `PATCH` | `/idp/configs/{id}` | Update |
| `DELETE` | `/idp/configs/{id}` | Delete |

Handler: `admin_idp.rs`. Supports Google (OIDC), GitHub (OAuth2), LDAP, and generic OIDC.

## Sessions

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/sessions?user_id=` | List active sessions for a user |
| `DELETE` | `/sessions/{id}` | Revoke |

Handler: `admin_sessions.rs`.

## Audit

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/audit?tenant_id=` | Query audit log (filterable by actor, action, date range) |

Handler: `admin_audit.rs`.

## Operators (system-level)

| Method | Path | Purpose |
|---|---|---|
| `GET/POST` | `/operators` | List, create |
| `GET/PATCH/DELETE` | `/operators/{id}` | Manage |
| `GET/POST` | `/operator-roles` | List, create |
| `GET/PATCH/DELETE` | `/operator-roles/{id}` | Manage |
| `POST/DELETE` | `/operator-roles/{id}/assignments` | Assign/revoke role |
| `GET` | `/operator-permissions` | Read-only catalog |

Handlers: `admin_operators.rs`, `admin_operator_roles.rs`, `admin_operator_permissions.rs`.
See [operator-rbac.md](operator-rbac.md).

## Authentication

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/admin/auth/login` | Operator login (password + optional TOTP) |
| `POST` | `/admin/auth/logout` | End operator session |
| `GET` | `/admin/auth/me` | Current operator + permissions |

Handler: `admin_auth.rs`.

## Error format

All admin endpoints return errors as:

```json
{ "error": "human-readable message", "code": "machine_code" }
```

with appropriate HTTP status. The admin UI surfaces `error` directly.
