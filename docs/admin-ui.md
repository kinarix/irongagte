# Admin UI

The admin UI is a Vite + React + TypeScript SPA in `admin-ui/`. It talks to the admin
API documented in [admin-api.md](admin-api.md). State is managed with TanStack Query;
routing with TanStack Router.

## Running

```bash
cd admin-ui
npm install
npm run dev
```

The dev server runs on `:5173` and proxies `/admin` to the Rust API on `:8080`.

## Layout

The sidebar has two sections:

### Tenant-scoped pages

Show data for the currently selected tenant (chosen from the dropdown at the top of
the sidebar):

| Route | Page | Purpose |
|---|---|---|
| `/users` | `pages/users/UserList.tsx`, `UserDetail.tsx` | End-user management; direct claim assignment; effective-claims preview |
| `/applications` | `pages/applications/AppList.tsx`, `AppForm.tsx` | OAuth clients; `claim_prefix` is set here |
| `/groups` | `pages/groups/GroupList.tsx`, `GroupDetail.tsx` | Groups, members, and group claim assignments |
| `/claims` | `pages/claims/ClaimDefList.tsx`, `ClaimDefForm.tsx` | All claim definitions across apps for the tenant |
| `/idp` | `pages/idp/IdpList.tsx`, `IdpForm.tsx` | Identity provider configurations |

### System pages

Not tenant-scoped — admin tooling for the system itself:

| Route | Page | Purpose |
|---|---|---|
| `/tenants` | `pages/tenants/TenantList.tsx`, `TenantDetail.tsx` | Tenant management; overview counters; quick links |
| `/operators` | `pages/operators/` | Admin accounts |
| `/operator-roles` | `pages/operator-roles/` | Admin role definitions |
| `/operator-permissions` | `pages/operator-permissions/` | Read-only permission catalog |

## Tenant Detail page

Tenants are clicked through from the list. The detail page shows:

- Header: name, slug, ID, created-at
- "Enter Tenant" button — switches the global tenant context and navigates to `/users`
- Five counters (Users, Applications, Groups, IdP configs, Claim defs) — each clickable,
  sets the tenant context, then navigates to the appropriate list page
- "Quick links" panel — same destinations as the counters, with icons

There is intentionally no "Roles" surface — see [claims-model.md](claims-model.md).

## Claim Definitions page

Located at `/claims`. This is a **global per-tenant** page, not nested under
Applications. It lists every `ClaimDefinition` across all apps in the tenant with
columns: App | Prefix | Key | Type | Description. The form picks the parent
application from a dropdown.

The Application form (`AppForm.tsx`) only sets `claim_prefix`. It does not embed claim
definitions inline.

## Group Detail page

Two sections:

- **Members** — add/remove users
- **Claims** — `(claim_key, value)` pairs. All group members inherit these.

## User Detail page

- **Profile & attributes** — JSON attributes editor
- **Direct claim assignments** — overrides scoped to this user
- **Effective claims preview** — pick an application; shows the resolved JSON that
  would appear in a token. Backed by `GET /admin/v1/claims/effective`.

## Code organization

```
admin-ui/src/
├── api/                # one file per backend resource
│   ├── applications.ts
│   ├── claims.ts       # claim defs + group/user assignments + effective preview
│   ├── groups.ts
│   ├── idp.ts
│   ├── operators.ts
│   ├── tenants.ts
│   └── users.ts
├── components/         # Layout, ConfirmDialog, Logo, ui/* (shadcn-style)
├── context/
│   └── tenant.tsx      # global tenant selection (id in localStorage)
├── pages/              # one folder per resource
├── router.tsx          # route table
├── auth.ts             # access token helpers
└── main.tsx
```

## Conventions

- API calls return parsed shapes — `listX()` → `{ x: X[], total: number }`.
- Forms use plain `useState` + TanStack Query mutations; no global form library.
- Error messages from the backend (`{ error: string }`) are shown verbatim.
- Tables and forms follow Tailwind utility classes — no theme system yet.
