use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::types::Permission;
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
pub struct CreatePermissionRequest {
    pub tenant_id: Uuid,
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
}

fn perm_to_json(p: &Permission) -> Value {
    json!({
        "id": p.id,
        "tenant_id": p.tenant_id,
        "resource": p.resource,
        "action": p.action,
        "description": p.description,
        "created_at": p.created_at,
    })
}

pub async fn list_permissions(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Value>> {
    let items = state.permissions.list(q.tenant_id).await?;
    let data: Vec<Value> = items.iter().map(perm_to_json).collect();
    Ok(Json(json!({ "permissions": data, "total": data.len() })))
}

pub async fn create_permission(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePermissionRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    let perm = Permission {
        id: Uuid::new_v4(),
        tenant_id: req.tenant_id,
        resource: req.resource,
        action: req.action,
        description: req.description,
        created_at: OffsetDateTime::now_utc(),
    };
    let created = state.permissions.create(perm).await?;
    Ok((StatusCode::CREATED, Json(perm_to_json(&created))))
}

pub async fn delete_permission(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    state.permissions.delete(id, tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
