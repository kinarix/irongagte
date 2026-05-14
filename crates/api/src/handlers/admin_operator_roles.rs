use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::{
    repositories::OperatorRoleScope,
    types::{
        op_action::{ASSIGN, CREATE, DELETE, LIST, READ, REVOKE, UPDATE},
        op_resource::{OPERATORS, OPERATOR_ROLES},
        OperatorPermission, OperatorRole,
    },
};
use serde::Deserialize;
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    audit,
    authz_op::{require_perm, scope_of, Scope},
    error::Result,
    handlers::admin_auth::AdminClaims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateOperatorRoleRequest {
    pub name: String,
    pub description: Option<String>,
    /// `None` (or omitted) creates a global role. `Some(id)` scopes it to one tenant.
    #[serde(default)]
    pub tenant_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOperatorRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListOperatorRolesQuery {
    /// `None` → roles in scopes the caller can access. `"global"` → only
    /// cross-tenant roles (requires global `operator_roles:list`).
    #[serde(default)]
    pub scope: Option<String>,
    /// Narrow to a single tenant. Requires `operator_roles:list` for that tenant.
    #[serde(default)]
    pub tenant_id: Option<Uuid>,
}

fn role_to_json(r: &OperatorRole) -> Value {
    json!({
        "id": r.id,
        "tenant_id": r.tenant_id,
        "name": r.name,
        "description": r.description,
        "created_at": r.created_at,
        "updated_at": r.updated_at,
    })
}

fn permission_to_json(p: &OperatorPermission) -> Value {
    json!({
        "id": p.id,
        "resource": p.resource,
        "action": p.action,
        "description": p.description,
        "created_at": p.created_at,
    })
}

/// Auto-filter `list_operator_roles` by what the caller can actually see, so a
/// tenant-scoped admin doesn't leak the existence of cross-tenant policy. Order:
///   - `?scope=global` → require global perm, return only global roles.
///   - `?tenant_id=<id>` → require perm for that tenant, return only its roles.
///   - no filter → if caller has the global perm, return everything; otherwise
///     return only the roles for tenants where they hold the perm. (For now we
///     conservatively require global; per-tenant fan-out can come later.)
pub async fn list_operator_roles(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListOperatorRolesQuery>,
) -> Result<Json<Value>> {
    let (scope_filter, perm_scope) = match (q.scope.as_deref(), q.tenant_id) {
        (Some("global"), _) => (OperatorRoleScope::Global, Scope::Global),
        (_, Some(tid)) => (OperatorRoleScope::Tenant(tid), Scope::Tenant(tid)),
        _ => (OperatorRoleScope::All, Scope::Global),
    };
    require_perm(&state, &claims, perm_scope, OPERATOR_ROLES, LIST).await?;
    let items = state.operator_roles_repo.list(scope_filter).await?;
    let data: Vec<Value> = items.iter().map(role_to_json).collect();
    Ok(Json(json!({ "roles": data, "total": data.len() })))
}

pub async fn create_operator_role(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateOperatorRoleRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    require_perm(
        &state,
        &claims,
        scope_of(req.tenant_id),
        OPERATOR_ROLES,
        CREATE,
    )
    .await?;
    let now = OffsetDateTime::now_utc();
    let role = OperatorRole {
        id: Uuid::new_v4(),
        tenant_id: req.tenant_id,
        name: req.name,
        description: req.description,
        created_at: now,
        updated_at: now,
    };
    let created = state.operator_roles_repo.create(role).await?;
    audit::record(
        &state,
        &claims,
        created.tenant_id,
        "operator_role.create",
        Some(created.id),
        serde_json::json!({ "name": created.name }),
    )
    .await;
    Ok((StatusCode::CREATED, Json(role_to_json(&created))))
}

pub async fn get_operator_role(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    // Look up first — scope comes from the role itself, never the URL.
    let role = state.operator_roles_repo.get_by_id(id).await?;
    require_perm(
        &state,
        &claims,
        scope_of(role.tenant_id),
        OPERATOR_ROLES,
        READ,
    )
    .await?;
    Ok(Json(role_to_json(&role)))
}

pub async fn update_operator_role(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOperatorRoleRequest>,
) -> Result<Json<Value>> {
    let mut role = state.operator_roles_repo.get_by_id(id).await?;
    require_perm(
        &state,
        &claims,
        scope_of(role.tenant_id),
        OPERATOR_ROLES,
        UPDATE,
    )
    .await?;
    if let Some(name) = req.name {
        role.name = name;
    }
    if let Some(description) = req.description {
        role.description = Some(description);
    }
    role.updated_at = OffsetDateTime::now_utc();
    let updated = state.operator_roles_repo.update(role).await?;
    audit::record(
        &state,
        &claims,
        updated.tenant_id,
        "operator_role.update",
        Some(updated.id),
        serde_json::json!({}),
    )
    .await;
    Ok(Json(role_to_json(&updated)))
}

pub async fn delete_operator_role(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let role = state.operator_roles_repo.get_by_id(id).await?;
    require_perm(
        &state,
        &claims,
        scope_of(role.tenant_id),
        OPERATOR_ROLES,
        DELETE,
    )
    .await?;
    state.operator_roles_repo.delete(id).await?;
    audit::record(
        &state,
        &claims,
        role.tenant_id,
        "operator_role.delete",
        Some(id),
        serde_json::json!({}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_role_permissions(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<Uuid>,
) -> Result<Json<Value>> {
    let role = state.operator_roles_repo.get_by_id(role_id).await?;
    require_perm(
        &state,
        &claims,
        scope_of(role.tenant_id),
        OPERATOR_ROLES,
        READ,
    )
    .await?;
    let items = state.operator_roles_repo.list_permissions(role_id).await?;
    let data: Vec<Value> = items.iter().map(permission_to_json).collect();
    Ok(Json(json!({ "permissions": data, "total": data.len() })))
}

pub async fn assign_permission_to_role(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((role_id, permission_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    let role = state.operator_roles_repo.get_by_id(role_id).await?;
    require_perm(
        &state,
        &claims,
        scope_of(role.tenant_id),
        OPERATOR_ROLES,
        ASSIGN,
    )
    .await?;
    state
        .operator_roles_repo
        .assign_permission(role_id, permission_id)
        .await?;
    audit::record(
        &state,
        &claims,
        role.tenant_id,
        "operator_role.permission_assigned",
        Some(role_id),
        serde_json::json!({ "permission_id": permission_id }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke_permission_from_role(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((role_id, permission_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    let role = state.operator_roles_repo.get_by_id(role_id).await?;
    require_perm(
        &state,
        &claims,
        scope_of(role.tenant_id),
        OPERATOR_ROLES,
        REVOKE,
    )
    .await?;
    state
        .operator_roles_repo
        .revoke_permission(role_id, permission_id)
        .await?;
    audit::record(
        &state,
        &claims,
        role.tenant_id,
        "operator_role.permission_revoked",
        Some(role_id),
        serde_json::json!({ "permission_id": permission_id }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_operator_role_assignments(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(operator_id): Path<Uuid>,
) -> Result<Json<Value>> {
    // Listing an operator's roles is a property of the operator (global resource).
    require_perm(&state, &claims, Scope::Global, OPERATORS, READ).await?;
    let items = state
        .operator_roles_repo
        .list_for_operator(operator_id)
        .await?;
    let data: Vec<Value> = items.iter().map(role_to_json).collect();
    Ok(Json(json!({ "roles": data, "total": data.len() })))
}

pub async fn assign_role_to_operator(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((operator_id, role_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    // Two-sided check: caller must have permission to grant in the *role's* scope
    // AND must be allowed to manage operators (operators are global).
    let role = state.operator_roles_repo.get_by_id(role_id).await?;
    require_perm(
        &state,
        &claims,
        scope_of(role.tenant_id),
        OPERATOR_ROLES,
        ASSIGN,
    )
    .await?;
    require_perm(&state, &claims, Scope::Global, OPERATORS, ASSIGN).await?;
    state
        .operator_roles_repo
        .assign_to_operator(operator_id, role_id)
        .await?;
    audit::record(
        &state,
        &claims,
        role.tenant_id,
        "operator_role.assigned_to_operator",
        Some(operator_id),
        serde_json::json!({ "role_id": role_id }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke_role_from_operator(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((operator_id, role_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    let role = state.operator_roles_repo.get_by_id(role_id).await?;
    require_perm(
        &state,
        &claims,
        scope_of(role.tenant_id),
        OPERATOR_ROLES,
        REVOKE,
    )
    .await?;
    require_perm(&state, &claims, Scope::Global, OPERATORS, REVOKE).await?;
    state
        .operator_roles_repo
        .revoke_from_operator(operator_id, role_id)
        .await?;
    audit::record(
        &state,
        &claims,
        role.tenant_id,
        "operator_role.revoked_from_operator",
        Some(operator_id),
        serde_json::json!({ "role_id": role_id }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
