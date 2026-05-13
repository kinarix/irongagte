use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::types::Tenant;
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
pub struct Pagination {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}
fn default_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTenantRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
}

fn tenant_to_json(t: &Tenant) -> Value {
    json!({
        "id": t.id,
        "name": t.name,
        "slug": t.slug,
        "settings": t.settings,
        "created_at": t.created_at,
        "updated_at": t.updated_at,
    })
}

pub async fn list_tenants(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(page): Query<Pagination>,
) -> Result<Json<Value>> {
    let items = state.tenants.list(page.limit, page.offset).await?;
    let data: Vec<Value> = items.iter().map(tenant_to_json).collect();
    Ok(Json(json!({ "tenants": data, "total": data.len() })))
}

pub async fn create_tenant(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    let now = OffsetDateTime::now_utc();
    let tenant = Tenant {
        id: Uuid::new_v4(),
        name: req.name,
        slug: req.slug,
        settings: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    let created = state.tenants.create(tenant).await?;
    Ok((StatusCode::CREATED, Json(tenant_to_json(&created))))
}

pub async fn get_tenant(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let tenant = state.tenants.get_by_id(id).await?;
    Ok(Json(tenant_to_json(&tenant)))
}

pub async fn update_tenant(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTenantRequest>,
) -> Result<Json<Value>> {
    let mut tenant = state.tenants.get_by_id(id).await?;
    if let Some(name) = req.name {
        tenant.name = name;
    }
    if let Some(slug) = req.slug {
        tenant.slug = slug;
    }
    tenant.updated_at = OffsetDateTime::now_utc();
    let updated = state.tenants.update(tenant).await?;
    Ok(Json(tenant_to_json(&updated)))
}

pub async fn delete_tenant(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    state.tenants.soft_delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
