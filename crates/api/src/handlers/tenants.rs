use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use irongate_core::Tenant;
use serde::Deserialize;
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    error::Result,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
}

fn tenant_to_json(tenant: &Tenant) -> Value {
    json!({
        "id": tenant.id,
        "name": tenant.name,
        "slug": tenant.slug,
        "settings": tenant.settings,
        "created_at": tenant.created_at,
        "updated_at": tenant.updated_at,
    })
}

pub async fn list_tenants(State(state): State<Arc<AppState>>) -> Result<Json<Value>> {
    let tenants = state.tenants.list(100, 0).await?;
    let items: Vec<Value> = tenants.iter().map(tenant_to_json).collect();
    let total = items.len();
    Ok(Json(json!({ "tenants": items, "total": total })))
}

pub async fn create_tenant(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    let now = OffsetDateTime::now_utc();
    let tenant = Tenant {
        id: Uuid::new_v4(),
        name: req.name,
        slug: req.slug,
        settings: serde_json::Value::Object(Default::default()),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    let created = state.tenants.create(tenant).await?;
    Ok((StatusCode::CREATED, Json(tenant_to_json(&created))))
}

pub async fn get_tenant(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let tenant = state.tenants.get_by_id(id).await?;
    Ok(Json(tenant_to_json(&tenant)))
}
