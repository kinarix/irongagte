use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::types::Role;
use serde::Deserialize;
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    error::Result,
    handlers::admin_auth::AdminClaims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct TenantQuery {
    pub tenant_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parent_role_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_role_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct AssignPermissionRequest {
    pub permission_id: Uuid,
}

fn role_to_json(r: &Role) -> Value {
    json!({
        "id": r.id,
        "tenant_id": r.tenant_id,
        "name": r.name,
        "description": r.description,
        "parent_role_id": r.parent_role_id,
        "created_at": r.created_at,
        "updated_at": r.updated_at,
    })
}

pub async fn list_roles(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Value>> {
    let items = state.roles.list(q.tenant_id).await?;
    let data: Vec<Value> = items.iter().map(role_to_json).collect();
    Ok(Json(json!({ "roles": data, "total": data.len() })))
}

pub async fn create_role(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    let now = OffsetDateTime::now_utc();
    let role = Role {
        id: Uuid::new_v4(),
        tenant_id: req.tenant_id,
        name: req.name,
        description: req.description,
        parent_role_id: req.parent_role_id,
        created_at: now,
        updated_at: now,
    };
    let created = state.roles.create(role).await?;
    Ok((StatusCode::CREATED, Json(role_to_json(&created))))
}

pub async fn get_role(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    let role = state.roles.get_by_id(id, tenant_id).await?;
    Ok(Json(role_to_json(&role)))
}

pub async fn update_role(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<Value>> {
    let mut role = state.roles.get_by_id(id, tenant_id).await?;
    if let Some(name) = req.name {
        role.name = name;
    }
    if let Some(desc) = req.description {
        role.description = Some(desc);
    }
    if req.parent_role_id.is_some() {
        role.parent_role_id = req.parent_role_id;
    }
    role.updated_at = OffsetDateTime::now_utc();
    let updated = state.roles.update(role).await?;
    Ok(Json(role_to_json(&updated)))
}

pub async fn delete_role(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    state.roles.delete(id, tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_role_permissions(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, role_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    let perms = state.permissions.get_permissions_for_role(role_id, tenant_id).await?;
    let data: Vec<Value> = perms.iter().map(|p| json!({
        "id": p.id,
        "resource": p.resource,
        "action": p.action,
        "description": p.description,
    })).collect();
    Ok(Json(json!({ "permissions": data })))
}

pub async fn assign_permission(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, role_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AssignPermissionRequest>,
) -> Result<StatusCode> {
    state.permissions.assign_permission_to_role(role_id, req.permission_id, tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_permission(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, _role_id, perm_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
    state.permissions.delete(perm_id, tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
