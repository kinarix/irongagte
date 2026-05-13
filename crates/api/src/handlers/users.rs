use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use irongate_core::{User, UserStatus};
use serde::Deserialize;
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
}

fn extract_tenant_id(headers: &HeaderMap) -> Result<Uuid> {
    headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| Error::BadRequest("missing or invalid X-Tenant-ID header".into()))
}

fn user_to_json(user: &User) -> Value {
    json!({
        "id": user.id,
        "tenant_id": user.tenant_id,
        "email": user.email,
        "email_verified": user.email_verified,
        "name": user.name,
        "given_name": user.given_name,
        "family_name": user.family_name,
        "picture_url": user.picture_url,
        "status": user.status.to_string(),
        "created_at": user.created_at,
        "updated_at": user.updated_at,
    })
}

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let tenant_id = extract_tenant_id(&headers)?;
    let users = state.users.list(tenant_id, 100, 0).await?;
    let items: Vec<Value> = users.iter().map(user_to_json).collect();
    let total = items.len();
    Ok(Json(json!({ "users": items, "total": total })))
}

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    let tenant_id = extract_tenant_id(&headers)?;
    let now = OffsetDateTime::now_utc();
    let user = User {
        id: Uuid::new_v4(),
        tenant_id,
        email: req.email,
        email_verified: false,
        name: req.name,
        given_name: req.given_name,
        family_name: req.family_name,
        picture_url: None,
        status: UserStatus::Pending,
        attributes: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        last_login_at: None,
        deleted_at: None,
    };
    let created = state.users.create(user).await?;
    Ok((StatusCode::CREATED, Json(user_to_json(&created))))
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let tenant_id = extract_tenant_id(&headers)?;
    let user = state.users.get_by_id(id, tenant_id).await?;
    Ok(Json(user_to_json(&user)))
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let tenant_id = extract_tenant_id(&headers)?;
    state.users.soft_delete(id, tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
