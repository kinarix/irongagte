# Irongate Documentation

Welcome. Irongate is a self-hostable Identity and Access Management (IAM) system written in Rust.
It is a Phase 1 implementation of OAuth 2.0 + OIDC server, federated identity, claim-based
authorization, SCIM 2.0 provisioning, and multi-tenancy — packaged as a single binary backed
by PostgreSQL and Redis.

## Table of contents

| Doc | Audience | What's in it |
|---|---|---|
| [Architecture](architecture.md) | New contributors | Crate layout, dependency rule, request lifecycle |
| [Claims model](claims-model.md) | Operators, integrators | The prefix/group/user claim system that replaces roles |
| [Operator RBAC](operator-rbac.md) | Operators | Admin-side permissions, separate from end-user authz |
| [Multi-tenancy](multi-tenancy.md) | Integrators | Tenant isolation rules and resolution |
| [OAuth / OIDC endpoints](oauth-oidc.md) | App developers | `/oauth2/*`, `/.well-known/*`, supported flows |
| [Admin API](admin-api.md) | Operators | Management REST endpoints used by `admin-ui` |
| [Admin UI](admin-ui.md) | Operators | Page layout and screens in `admin-ui/` |
| [Migrations](migrations.md) | Contributors | Migration list, how to add a new one |
| [Development](development.md) | Contributors | Build, test, dev loop, Docker setup |

## Quick links

- Source: [github.com/kinarix/irongate](https://github.com/kinarix/irongate)
- Top-level [README](../README.md) — install + quickstart
- [CLAUDE.md](../CLAUDE.md) — context for AI assistants working in this repo
- [SPEC.md](../SPEC.md) — full Phase 1 specification

## Conceptual model in one paragraph

A **tenant** is the isolation boundary. Inside a tenant live **users**, **groups**,
**applications**, **IdP configs**, and **claim definitions**. Each application has a
unique `claim_prefix`. Its claim definitions (`scalar` or `multi`) become JWT keys of
the form `<prefix>:<key>`. Values are assigned to groups (members inherit) or directly
to users. At token-mint time, claims are resolved: `multi` claims merge across all
sources; `scalar` claims pick user-direct, then highest-priority group, then earliest
`created_at`. Standard OIDC claims (`sub`, `email`, `name`, …) come from the user
record and are emitted unprefixed.
