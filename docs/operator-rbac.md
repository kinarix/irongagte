# Operator RBAC

Irongate distinguishes between two authorization systems:

| System | Subject | Mechanism | Used for |
|---|---|---|---|
| **End-user authorization** | `User` | [Claims model](claims-model.md) | What appears in app-issued JWTs |
| **Operator authorization** | `Operator` | Permissions + roles | Who can use the admin/management API |

This page covers the second one.

## Why a separate system

Operators administer the IAM system itself. They sign in to the admin UI, create
tenants, view audit logs, and manage other operators. The permissions they need
(`tenants:create`, `users:read`, `applications:write`) are fixed, system-defined,
and have nothing to do with the per-application claim vocabulary that end users see.

Modeling these together would muddy the claim model — operator permissions would have
to live under some synthetic app/prefix and would clutter every end-user tooling
screen. Keeping them separate also lets the admin API enforce permissions without
running the claim-merge pipeline on every request.

## Entities

| Table | Shape |
|---|---|
| `operators` | Admin account: email, password hash, status |
| `operator_credentials` | Password hash, TOTP secret |
| `operator_permissions` | Catalog of permissions: `(scope, action)` |
| `operator_roles` | Named bundle of permissions; optionally tenant-scoped |
| `operator_role_permissions` | Many-to-many between roles and permissions |
| `operator_role_assignments` | Grants a role to an operator |

A permission is a `(scope, action)` pair, e.g. `("tenants", "create")`. The catalog is
seeded on system bootstrap and grows when a new admin endpoint is added.

## Tenant scoping

An `operator_role` may be either:

- **Global** — `tenant_id IS NULL`. Applies across all tenants. Used for super-admin
  roles.
- **Tenant-scoped** — `tenant_id = <id>`. The role's permissions apply only within
  that tenant.

A `super_admin` role with `tenant_id IS NULL` and the full permission catalog can
do anything anywhere. A tenant-scoped `tenant_admin` can manage users/apps/groups
within its tenant but cannot create new tenants.

## Bootstrap

The CLI subcommand `irongate admin init` (implemented in
`crates/api/src/cmd/admin_init.rs`) creates the first operator and assigns a
super-admin role. After that, additional operators are created through the admin UI
(Operators page → Operator Roles → Operator Permissions).

## Login flow

Operators sign in via `/admin/auth/login` with password (and TOTP if configured).
The response sets an admin session cookie. The admin API checks the cookie on each
request and resolves the operator's effective permission set by union of all
assigned roles.

## Enforcement

Admin handlers call a permission check at the top:

```rust
state.operator_authz.require(operator_id, "tenants", "create").await?;
```

Tenant-scoped requests also pass the path's `tenant_id`. The check returns `Forbidden`
if the operator has neither a matching global role nor a tenant-scoped role for that
tenant.

## Admin UI surfaces

- **Operators** (`/operators`) — list, invite, suspend
- **Operator Roles** (`/operator-roles`) — define roles, assign permissions
- **Operator Permissions** (`/operator-permissions`) — read-only catalog

These appear under the "System" section of the sidebar in `admin-ui`, distinct from
the tenant-scoped surfaces (Users, Applications, Groups, Claims, IdPs).
