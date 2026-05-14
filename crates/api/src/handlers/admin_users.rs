use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::types::{
    op_action::{CREATE, DELETE, LIST, READ, UPDATE},
    op_resource::USERS,
    User, UserStatus,
};
use serde::Deserialize;
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    audit,
    authz_op::{require_perm, Scope},
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
    pub attributes: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub email_verified: Option<bool>,
    pub attributes: Option<serde_json::Value>,
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
        "attributes": u.attributes,
        "created_at": u.created_at,
        "updated_at": u.updated_at,
        "last_login_at": u.last_login_at,
    })
}

pub async fn list_users(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(q.tenant_id), USERS, LIST).await?;
    let items = state.users.list(q.tenant_id, q.limit, q.offset).await?;
    let data: Vec<Value> = items.iter().map(user_to_json).collect();
    Ok(Json(json!({ "users": data, "total": data.len() })))
}

pub async fn create_user(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    require_perm(&state, &claims, Scope::Tenant(req.tenant_id), USERS, CREATE).await?;
    let attributes = req.attributes.unwrap_or(serde_json::json!({}));
    if !attributes.is_object() {
        return Err(Error::BadRequest(
            "attributes must be a JSON object".to_string(),
        ));
    }
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
        attributes,
        created_at: now,
        updated_at: now,
        last_login_at: None,
        deleted_at: None,
    };
    let created = state.users.create(user).await?;
    audit::record(
        &state,
        &claims,
        Some(created.tenant_id),
        "user.create",
        Some(created.id),
        serde_json::json!({}),
    )
    .await;
    Ok((StatusCode::CREATED, Json(user_to_json(&created))))
}

pub async fn get_user(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), USERS, READ).await?;
    let user = state.users.get_by_id(id, tenant_id).await?;
    Ok(Json(user_to_json(&user)))
}

pub async fn update_user(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), USERS, UPDATE).await?;
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
    if let Some(attrs) = req.attributes {
        if !attrs.is_object() {
            return Err(Error::BadRequest(
                "attributes must be a JSON object".to_string(),
            ));
        }
        user.attributes = attrs;
    }
    user.updated_at = OffsetDateTime::now_utc();
    let updated = state.users.update(user).await?;
    audit::record(
        &state,
        &claims,
        Some(updated.tenant_id),
        "user.update",
        Some(updated.id),
        serde_json::json!({}),
    )
    .await;
    Ok(Json(user_to_json(&updated)))
}

pub async fn delete_user(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), USERS, DELETE).await?;
    state.users.soft_delete(id, tenant_id).await?;
    audit::record(
        &state,
        &claims,
        Some(tenant_id),
        "user.delete",
        Some(id),
        serde_json::json!({}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn suspend_user(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), USERS, UPDATE).await?;
    let mut user = state.users.get_by_id(id, tenant_id).await?;
    user.status = UserStatus::Suspended;
    user.updated_at = OffsetDateTime::now_utc();
    let updated = state.users.update(user).await?;
    audit::record(
        &state,
        &claims,
        Some(tenant_id),
        "user.suspend",
        Some(id),
        serde_json::json!({}),
    )
    .await;
    Ok(Json(user_to_json(&updated)))
}

pub async fn unsuspend_user(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), USERS, UPDATE).await?;
    let mut user = state.users.get_by_id(id, tenant_id).await?;
    user.status = UserStatus::Active;
    user.updated_at = OffsetDateTime::now_utc();
    let updated = state.users.update(user).await?;
    audit::record(
        &state,
        &claims,
        Some(tenant_id),
        "user.unsuspend",
        Some(id),
        serde_json::json!({}),
    )
    .await;
    Ok(Json(user_to_json(&updated)))
}

#[derive(Debug, Deserialize)]
pub struct BulkUserRow {
    pub email: String,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    #[serde(default)]
    pub attributes: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct BulkImportRequest {
    pub users: Vec<BulkUserRow>,
    /// If `Some`, every successfully-imported user is added to this group and
    /// inherits its claim assignments at token mint time.
    #[serde(default)]
    pub group_id: Option<Uuid>,
    /// If true, duplicate-email conflicts are skipped rather than aborting the
    /// batch. Other store errors still abort the affected row only.
    #[serde(default)]
    pub skip_duplicates: bool,
}

/// Bulk-create users for a tenant. Each row reuses the same shape as the
/// single-user create. Per-row failures are collected and returned alongside
/// the success count so the caller can surface them. The optional `group_id`
/// gives imported users the group's claims automatically — the claim
/// resolution engine reads `group_members` joined to `group_claims` at mint
/// time, so no per-claim work is needed here.
pub async fn bulk_create_users(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(tenant_id): Path<Uuid>,
    Json(req): Json<BulkImportRequest>,
) -> Result<Json<Value>> {
    use irongate_core::errors::StoreError;
    require_perm(&state, &claims, Scope::Tenant(tenant_id), USERS, CREATE).await?;

    let mut created_count = 0_usize;
    let mut skipped = 0_usize;
    let mut errors: Vec<Value> = Vec::new();
    let total = req.users.len();

    for (idx, row) in req.users.into_iter().enumerate() {
        let attributes = row.attributes.unwrap_or(serde_json::json!({}));
        if !attributes.is_object() {
            errors.push(json!({
                "index": idx,
                "email": row.email,
                "error": "attributes must be a JSON object",
            }));
            continue;
        }
        let now = OffsetDateTime::now_utc();
        let user = User {
            id: Uuid::new_v4(),
            tenant_id,
            email: row.email.clone(),
            email_verified: false,
            name: row.name,
            given_name: row.given_name,
            family_name: row.family_name,
            picture_url: None,
            status: UserStatus::Pending,
            attributes,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            deleted_at: None,
        };
        match state.users.create(user).await {
            Ok(u) => {
                if let Some(group_id) = req.group_id {
                    if let Err(e) = state.groups.add_member(group_id, u.id, tenant_id).await {
                        errors.push(json!({
                            "index": idx,
                            "email": row.email,
                            "error": format!("user created but group add failed: {e}"),
                        }));
                    }
                }
                created_count += 1;
            }
            Err(StoreError::Conflict(_)) if req.skip_duplicates => {
                skipped += 1;
            }
            Err(e) => {
                errors.push(json!({
                    "index": idx,
                    "email": row.email,
                    "error": e.to_string(),
                }));
            }
        }
    }

    audit::record(
        &state,
        &claims,
        Some(tenant_id),
        "users.bulk_import",
        None,
        json!({
            "total": total,
            "created": created_count,
            "skipped": skipped,
            "errored": errors.len(),
            "group_id": req.group_id,
        }),
    )
    .await;

    Ok(Json(json!({
        "created": created_count,
        "skipped": skipped,
        "errors": errors,
    })))
}
