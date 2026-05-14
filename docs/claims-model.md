# Claims model

Irongate does not have a `Role` entity for end users. Anything role-like is expressed
as a **claim**: a typed key/value pair projected into the JWT at mint time. This page
explains how custom claims are defined, assigned, and resolved.

## Why this design

A "role" in most IAM systems is a controlled vocabulary that only matters when a token
is issued. Modeling it as a first-class entity adds tables, handlers, and UI without
adding semantics that downstream apps actually use — apps consume the *string in the
token*. Claims compress role + permission + arbitrary attribute (`plan`, `region`,
`feature_flags`) into one mechanism with one merge rule.

## The shape of a custom claim

A claim is defined on an **application**:

| Field | Notes |
|---|---|
| `application_id` | Parent app |
| `key` | Unprefixed identifier — `roles`, `plan`, `region` |
| `claim_type` | `scalar` (single value) or `multi` (array) |
| `description` | Freeform |

At mint time, the JWT key is `<application.claim_prefix>:<claim_definition.key>`, e.g.
`billing:roles`. The prefix is set on the Application, is unique per tenant, and is
validated against the OIDC reserved-name list (see [validation](#validation) below).

OIDC standard claims (`sub`, `email`, `email_verified`, `name`, `given_name`,
`family_name`, `picture`, `locale`, `updated_at`) come from the user record and are
emitted unprefixed at their canonical keys. They are not configurable via the claim
system.

## Two assignment surfaces

### Group claims

```
GroupClaim { group_id, claim_def_id, value }
```

Every member of the group inherits the value at mint time. Multiple members of the
same group all get the same value.

### User claims

```
UserClaim { user_id, claim_def_id, value }
```

A direct assignment to one user. Useful for overrides or for users outside any group.

## Resolution rules

For each `ClaimDefinition` associated with the application a token is being minted
against, the engine collects:

1. All `GroupClaim`s for the user's groups
2. All `UserClaim`s for the user

Then, based on `claim_type`:

### `multi`

All values from groups and from direct user assignment are unioned, deduplicated, and
emitted as a JSON array. Order is `(priority DESC, created_at ASC, value ASC)` to make
output deterministic, but consumers should not depend on order.

Example: user in `billing-admins` (`roles=admin`) and `billing-viewers` (`roles=viewer`),
plus direct user claim `roles=owner`:

```json
{ "billing:roles": ["admin", "owner", "viewer"] }
```

### `scalar`

Precedence:

1. If a `UserClaim` exists for the user, its value wins.
2. Otherwise, pick the group with the highest `priority` (integer, default 0).
3. Ties broken by `created_at` ascending.

If no source resolves a value, the claim is omitted from the token. There is no
"default value" mechanism — keep the model strict.

Example: user in `enterprise` (`plan=enterprise`, priority 10) and `trial`
(`plan=trial`, priority 0):

```json
{ "billing:plan": "enterprise" }
```

If the same user has a direct claim `plan=internal`:

```json
{ "billing:plan": "internal" }
```

## Validation

### Prefix

`claim_prefix` on an Application:

- Required, non-empty.
- Must not collide with reserved OIDC top-level claims:
  `sub`, `iss`, `aud`, `exp`, `iat`, `jti`, `nbf`, `nonce`, `auth_time`, `acr`, `amr`,
  `azp`, `tenant_id`, `scope`.
- Unique per tenant (enforced by a partial index excluding soft-deleted rows).

### Key

`key` on a `ClaimDefinition`:

- Letters, digits, underscore, hyphen.
- Unique per `(application_id, key)`.

### Value

Stored as `TEXT`. The engine does not coerce types — what you assign is what shows up
in the JWT. Consumers needing typed values should parse on their side.

## Bulk user import

The user import handler accepts an optional `group_id`. Imported users are added to
that group, so they automatically inherit the group's claims without further setup.

## Token-mint walkthrough

When a token is minted for `(user_id, application_id)`:

1. Load standard OIDC claims from `User`. Emit unprefixed.
2. Look up `application.claim_prefix`.
3. For each `ClaimDefinition` with that `application_id`:
   - Query `group_claims` joined with `group_members` and `groups` for the user.
   - Query `user_claims` for the user.
   - Apply the merge rule for the `claim_type`.
   - If a value resolves, emit it under `"<prefix>:<key>"`.

See `crates/authz/src/engine.rs::resolve_claims_for_app` for the implementation
and `crates/api/src/handlers/token.rs::resolve_claims` for the integration point.

## Admin UI surfaces

- **Claims** (global per tenant) — list and CRUD all `ClaimDefinition`s across apps,
  filterable by app. Located at `/claims`.
- **Application form** — set `claim_prefix` only; claim definitions are managed on
  the Claims page.
- **Group detail** — assign `(claim_def, value)` pairs.
- **User detail** — assign direct overrides; preview effective claims for any
  application via `/claims/effective`.

See [admin-ui.md](admin-ui.md).

## API endpoints

Backed by `crates/api/src/handlers/admin_claims.rs`:

- `GET/POST /admin/v1/claims/definitions`
- `GET/PATCH/DELETE /admin/v1/claims/definitions/{id}`
- `POST/DELETE /admin/v1/claims/group-assignments`
- `POST/DELETE /admin/v1/claims/user-assignments`
- `GET /admin/v1/claims/effective?tenant_id=…&user_id=…&application_id=…`

See [admin-api.md](admin-api.md) for full request/response shapes.
