use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::types::{
    op_action::{ASSIGN, CREATE, DELETE, LIST, READ, REVOKE, UPDATE},
    op_resource::GROUPS,
    Group,
};
use serde::Deserialize;
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    audit,
    authz_op::{require_perm, Scope},
    error::Result,
    handlers::admin_auth::AdminClaims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct TenantQuery {
    pub tenant_id: Uuid,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub tenant_id: Uuid,
    pub display_name: String,
    pub external_id: Option<String>,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupRequest {
    pub display_name: Option<String>,
    pub external_id: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
}

fn group_to_json(g: &Group) -> Value {
    json!({
        "id": g.id,
        "tenant_id": g.tenant_id,
        "display_name": g.display_name,
        "external_id": g.external_id,
        "priority": g.priority,
        "created_at": g.created_at,
        "updated_at": g.updated_at,
    })
}

pub async fn list_groups(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(q.tenant_id), GROUPS, LIST).await?;
    let items = state.groups.list(q.tenant_id, q.limit, q.offset).await?;
    let data: Vec<Value> = items.iter().map(group_to_json).collect();
    Ok(Json(json!({ "groups": data, "total": data.len() })))
}

pub async fn create_group(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateGroupRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    require_perm(
        &state,
        &claims,
        Scope::Tenant(req.tenant_id),
        GROUPS,
        CREATE,
    )
    .await?;
    let now = OffsetDateTime::now_utc();
    let group = Group {
        id: Uuid::new_v4(),
        tenant_id: req.tenant_id,
        display_name: req.display_name,
        external_id: req.external_id,
        priority: req.priority,
        created_at: now,
        updated_at: now,
    };
    let created = state.groups.create(group).await?;
    audit::record(
        &state,
        &claims,
        Some(created.tenant_id),
        "group.create",
        Some(created.id),
        serde_json::json!({ "display_name": created.display_name, "priority": created.priority }),
    )
    .await;
    Ok((StatusCode::CREATED, Json(group_to_json(&created))))
}

pub async fn get_group(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), GROUPS, READ).await?;
    let group = state.groups.get_by_id(id, tenant_id).await?;
    Ok(Json(group_to_json(&group)))
}

pub async fn update_group(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateGroupRequest>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), GROUPS, UPDATE).await?;
    let mut group = state.groups.get_by_id(id, tenant_id).await?;
    if let Some(name) = req.display_name {
        group.display_name = name;
    }
    if let Some(ext) = req.external_id {
        group.external_id = Some(ext);
    }
    if let Some(p) = req.priority {
        group.priority = p;
    }
    group.updated_at = OffsetDateTime::now_utc();
    let updated = state.groups.update(group).await?;
    audit::record(
        &state,
        &claims,
        Some(updated.tenant_id),
        "group.update",
        Some(updated.id),
        serde_json::json!({}),
    )
    .await;
    Ok(Json(group_to_json(&updated)))
}

pub async fn delete_group(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), GROUPS, DELETE).await?;
    state.groups.delete(id, tenant_id).await?;
    audit::record(
        &state,
        &claims,
        Some(tenant_id),
        "group.delete",
        Some(id),
        serde_json::json!({}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_group_members(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), GROUPS, READ).await?;
    let users = state.groups.list_members(id, tenant_id).await?;
    let data: Vec<Value> = users
        .iter()
        .map(|u| {
            json!({
                "id": u.id,
                "email": u.email,
                "name": u.name,
            })
        })
        .collect();
    Ok(Json(json!({ "members": data })))
}

pub async fn add_group_member(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AddMemberRequest>,
) -> Result<StatusCode> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), GROUPS, ASSIGN).await?;
    state.groups.add_member(id, req.user_id, tenant_id).await?;
    audit::record(
        &state,
        &claims,
        Some(tenant_id),
        "group.member_added",
        Some(id),
        serde_json::json!({ "user_id": req.user_id }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_group_member(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, group_id, user_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), GROUPS, REVOKE).await?;
    state
        .groups
        .remove_member(group_id, user_id, tenant_id)
        .await?;
    audit::record(
        &state,
        &claims,
        Some(tenant_id),
        "group.member_removed",
        Some(group_id),
        serde_json::json!({ "user_id": user_id }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
