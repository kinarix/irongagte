# Migrations

All migrations live in `migrations/postgres/` and are applied via `sqlx-cli`. They are
numbered sequentially and run in order. There is no down-migration tooling in this
repo — rolling back means writing a new forward migration.

## Current migrations

| # | File | Purpose |
|---|---|---|
| 0001 | `create_operators.sql` | Operator (admin) accounts |
| 0002 | `create_operator_credentials.sql` | Password hash, TOTP secret |
| 0003 | `create_tenants.sql` | Top-level isolation boundary |
| 0004 | `create_users.sql` | End users |
| 0005 | `create_user_credentials.sql` | Password hash, TOTP |
| 0006 | `create_identities.sql` | Federated identity links |
| 0007 | `create_applications.sql` | OAuth clients |
| 0008 | `create_refresh_tokens.sql` | Opaque refresh tokens (SHA-256 hash) |
| 0009 | `create_roles.sql` | (dropped in 0028) |
| 0010 | `create_user_roles.sql` | (dropped in 0028) |
| 0011 | `create_idp_configs.sql` | Per-tenant IdP configs |
| 0012 | `create_magic_links.sql` | Magic link tokens |
| 0013 | `create_passkeys.sql` | WebAuthn credentials |
| 0014 | `create_groups.sql` | User groups |
| 0015 | `create_group_members.sql` | Group membership |
| 0016 | `create_audit_events.sql` | Append-only audit log |
| 0017 | `create_operator_permissions.sql` | Admin permission catalog |
| 0018 | `create_operator_roles.sql` | Admin role definitions |
| 0019 | `create_operator_role_permissions.sql` | Many-to-many |
| 0020 | `create_operator_role_assignments.sql` | Operator ↔ role |
| 0021 | `add_users_attributes.sql` | JSONB `attributes` on users |
| 0022 | `add_operator_roles_tenant_id.sql` | Scope operator roles to a tenant |
| 0023 | `add_applications_claim_prefix.sql` | New: per-app claim prefix; drops legacy `claims_config` |
| 0024 | `create_claim_definitions.sql` | New: typed custom claim schema |
| 0025 | `create_group_claims.sql` | New: claim values assigned to groups |
| 0026 | `create_user_claims.sql` | New: claim values assigned to users |
| 0027 | `add_groups_priority.sql` | New: scalar-claim conflict resolution |
| 0028 | `drop_roles.sql` | New: drops `user_roles` then `roles` — old role system removed |

The 0023–0028 sequence implements the claim model documented in
[claims-model.md](claims-model.md).

## Running migrations

```bash
# install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# apply
export DATABASE_URL=postgres://user:pass@localhost:5432/irongate
sqlx migrate run --source migrations/postgres
```

The `Makefile` provides shortcuts:

```bash
make db-reset        # drop, create, migrate
make db-migrate      # migrate only
```

## Adding a new migration

1. Pick the next number, e.g. `0029_my_change.sql`.
2. Write forward SQL only. Wrap in a transaction where useful.
3. If the change is destructive, dump row counts in a `SELECT` first so the migration's
   own output documents what was destroyed.
4. Run `make db-reset` locally to verify a clean apply.
5. Add a row to the table above.

## Schema invariants worth knowing

- Every domain row has `tenant_id` (except operator-system tables and tenants themselves).
- Soft delete is `deleted_at TIMESTAMPTZ NULL`. Unique indexes that should ignore deleted
  rows are partial: `WHERE deleted_at IS NULL`.
- Primary keys are UUIDs (`uuid` type, `gen_random_uuid()` defaults).
- Timestamps are `TIMESTAMPTZ`, never naive `TIMESTAMP`.

## SQLite

SQLite support has been removed. All deployments use PostgreSQL. If you find a stray
`migrations/sqlite/` directory in an old branch, ignore it.
