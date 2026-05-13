use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::types::{User, UserStatus};
use serde::Deserialize;
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
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
    50
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub tenant_id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub email_verified: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AssignRoleRequest {
    pub role_id: Uuid,
}

fn user_to_json(u: &User) -> Value {
    json!({
        "id": u.id,
        "tenant_id": u.tenant_id,
        "email": u.email,
        "email_verified": u.email_verified,
        "name": u.name,
        "given_name": u.given_name,
        "family_name": u.family_name,
        "picture_url": u.picture_url,
        "status": u.status.to_string(),
        "created_at": u.created_at,
        "updated_at": u.updated_at,
        "last_login_at": u.last_login_at,
    })
}

pub async fn list_users(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Value>> {
    let items = state.users.list(q.tenant_id, q.limit, q.offset).await?;
    let data: Vec<Value> = items.iter().map(user_to_json).collect();
    Ok(Json(json!({ "users": data, "total": data.len() })))
}

pub async fn create_user(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    let now = OffsetDateTime::now_utc();
    let user = User {
        id: Uuid::new_v4(),
        tenant_id: req.tenant_id,
        email: req.email,
        email_verified: false,
        name: req.name,
        given_name: req.given_name,
        family_name: req.family_name,
        picture_url: None,
        status: UserStatus::Pending,
        created_at: now,
        updated_at: now,
        last_login_at: None,
        deleted_at: None,
    };
    let created = state.users.create(user).await?;
    Ok((StatusCode::CREATED, Json(user_to_json(&created))))
}

pub async fn get_user(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    let user = state.users.get_by_id(id, tenant_id).await?;
    Ok(Json(user_to_json(&user)))
}

pub async fn update_user(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<Value>> {
    let mut user = state.users.get_by_id(id, tenant_id).await?;
    if let Some(email) = req.email {
        user.email = email;
    }
    if let Some(name) = req.name {
        user.name = Some(name);
    }
    if let Some(given_name) = req.given_name {
        user.given_name = Some(given_name);
    }
    if let Some(family_name) = req.family_name {
        user.family_name = Some(family_name);
    }
    if let Some(ev) = req.email_verified {
        user.email_verified = ev;
    }
    user.updated_at = OffsetDateTime::now_utc();
    let updated = state.users.update(user).await?;
    Ok(Json(user_to_json(&updated)))
}

pub async fn delete_user(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    state.users.soft_delete(id, tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn suspend_user(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    let mut user = state.users.get_by_id(id, tenant_id).await?;
    user.status = UserStatus::Suspended;
    user.updated_at = OffsetDateTime::now_utc();
    let updated = state.users.update(user).await?;
    Ok(Json(user_to_json(&updated)))
}

pub async fn unsuspend_user(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    let mut user = state.users.get_by_id(id, tenant_id).await?;
    user.status = UserStatus::Active;
    user.updated_at = OffsetDateTime::now_utc();
    let updated = state.users.update(user).await?;
    Ok(Json(user_to_json(&updated)))
}

pub async fn list_user_roles(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    let roles = state.authz_svc.get_user_roles(id, tenant_id).await.map_err(|e| Error::Internal(e.to_string()))?;
    let data: Vec<Value> = roles.iter().map(|r| json!({
        "id": r.id,
        "name": r.name,
        "description": r.description,
    })).collect();
    Ok(Json(json!({ "roles": data })))
}

pub async fn assign_role(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AssignRoleRequest>,
) -> Result<StatusCode> {
    state.authz_svc.assign_role(user_id, req.role_id, tenant_id).await.map_err(|e| Error::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_role(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, user_id, role_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
    state.authz_svc.remove_role(user_id, role_id, tenant_id).await.map_err(|e| Error::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
