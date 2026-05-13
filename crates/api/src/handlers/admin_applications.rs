use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::types::{AppType, Application};
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
pub struct CreateApplicationRequest {
    pub tenant_id: Uuid,
    pub name: String,
    pub client_id: String,
    pub app_type: String,
    pub redirect_uris: Vec<String>,
    pub allowed_scopes: Vec<String>,
    pub grant_types: Vec<String>,
    #[serde(default = "default_access_ttl")]
    pub access_token_ttl: i64,
    #[serde(default = "default_refresh_ttl")]
    pub refresh_token_ttl: i64,
}
fn default_access_ttl() -> i64 {
    3600
}
fn default_refresh_ttl() -> i64 {
    86400 * 7
}

#[derive(Debug, Deserialize)]
pub struct UpdateApplicationRequest {
    pub name: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
    pub allowed_scopes: Option<Vec<String>>,
    pub grant_types: Option<Vec<String>>,
    pub access_token_ttl: Option<i64>,
    pub refresh_token_ttl: Option<i64>,
}

fn app_to_json(a: &Application) -> Value {
    json!({
        "id": a.id,
        "tenant_id": a.tenant_id,
        "name": a.name,
        "client_id": a.client_id,
        "app_type": a.app_type.to_string(),
        "redirect_uris": a.redirect_uris,
        "allowed_scopes": a.allowed_scopes,
        "grant_types": a.grant_types,
        "access_token_ttl": a.access_token_ttl,
        "refresh_token_ttl": a.refresh_token_ttl,
        "created_at": a.created_at,
        "updated_at": a.updated_at,
    })
}

pub async fn list_applications(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Value>> {
    let items = state.applications.list(q.tenant_id, q.limit, q.offset).await?;
    let data: Vec<Value> = items.iter().map(app_to_json).collect();
    Ok(Json(json!({ "applications": data, "total": data.len() })))
}

pub async fn create_application(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateApplicationRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    let app_type: AppType = req.app_type.parse().map_err(|e: irongate_core::errors::CoreError| {
        Error::BadRequest(e.to_string())
    })?;
    let now = OffsetDateTime::now_utc();
    let app = Application {
        id: Uuid::new_v4(),
        tenant_id: req.tenant_id,
        name: req.name,
        client_id: req.client_id,
        client_secret_hash: None,
        app_type,
        redirect_uris: req.redirect_uris,
        allowed_scopes: req.allowed_scopes,
        grant_types: req.grant_types,
        access_token_ttl: req.access_token_ttl,
        refresh_token_ttl: req.refresh_token_ttl,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    let created = state.applications.create(app).await?;
    Ok((StatusCode::CREATED, Json(app_to_json(&created))))
}

pub async fn get_application(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    let app = state.applications.get_by_id(id, tenant_id).await?;
    Ok(Json(app_to_json(&app)))
}

pub async fn update_application(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateApplicationRequest>,
) -> Result<Json<Value>> {
    let mut app = state.applications.get_by_id(id, tenant_id).await?;
    if let Some(name) = req.name {
        app.name = name;
    }
    if let Some(uris) = req.redirect_uris {
        app.redirect_uris = uris;
    }
    if let Some(scopes) = req.allowed_scopes {
        app.allowed_scopes = scopes;
    }
    if let Some(grants) = req.grant_types {
        app.grant_types = grants;
    }
    if let Some(ttl) = req.access_token_ttl {
        app.access_token_ttl = ttl;
    }
    if let Some(ttl) = req.refresh_token_ttl {
        app.refresh_token_ttl = ttl;
    }
    app.updated_at = OffsetDateTime::now_utc();
    let updated = state.applications.update(app).await?;
    Ok(Json(app_to_json(&updated)))
}

pub async fn delete_application(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    state.applications.soft_delete(id, tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
