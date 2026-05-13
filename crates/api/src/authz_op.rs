//! Operator authorization helpers. Every admin handler calls `require_perm`
//! at the top to assert the caller's effective permissions cover the
//! (resource, action) being attempted in the given scope.
//!
//! Scope rules:
//! - `Scope::Global` — uses the operator's permissions granted by **global**
//!   roles only (`tenant_id IS NULL`).
//! - `Scope::Tenant(tid)` — uses the union of global permissions and
//!   permissions granted by roles scoped to `tid`.
//!
//! Always resolve `Scope` from the **resource being touched**, not from
//! `Path`/`Body` blindly. For role-id-only endpoints, look up the role first
//! and pass `scope_of(role.tenant_id)`.

use std::sync::Arc;

use uuid::Uuid;

use crate::{claims::OperatorClaims, error::Error, state::AppState};

#[derive(Debug, Clone, Copy)]
pub enum Scope {
    /// Action targets a global resource (tenants, operators, etc.) — only
    /// permissions from the caller's global roles count.
    Global,
    /// Action targets a resource inside `tenant_id` — both global and
    /// tenant-scoped roles for that tenant count.
    Tenant(Uuid),
}

/// Convenience for handlers that already hold an `Option<Uuid>` representing
/// the resource's owning tenant (e.g. `OperatorRole.tenant_id`).
pub fn scope_of(tenant_id: Option<Uuid>) -> Scope {
    match tenant_id {
        Some(t) => Scope::Tenant(t),
        None => Scope::Global,
    }
}

/// Assert that the operator identified by `claims` has the (resource, action)
/// permission in the given scope. Returns `Error::Forbidden` if not.
pub async fn require_perm(
    state: &Arc<AppState>,
    claims: &OperatorClaims,
    scope: Scope,
    resource: &str,
    action: &str,
) -> Result<(), Error> {
    let operator_id: Uuid = claims
        .sub
        .parse()
        .map_err(|_| Error::Unauthorized("invalid operator id in token".into()))?;

    let perms = match scope {
        Scope::Global => state
            .operator_roles_repo
            .list_permissions_for_operator_global(operator_id)
            .await
            .map_err(|e| Error::Internal(e.to_string()))?,
        Scope::Tenant(tid) => state
            .operator_roles_repo
            .list_permissions_for_operator_in_tenant(operator_id, tid)
            .await
            .map_err(|e| Error::Internal(e.to_string()))?,
    };

    if perms
        .iter()
        .any(|p| p.resource == resource && p.action == action)
    {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}
