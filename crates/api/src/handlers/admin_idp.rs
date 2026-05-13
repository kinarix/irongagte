use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::types::{
    op_action::{CREATE, DELETE, LIST, READ, UPDATE},
    op_resource::IDP_CONFIGS,
    IdpConfig, IdpType,
};
use serde::Deserialize;
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    authz_op::{require_perm, Scope},
    error::{Error, Result},
    handlers::admin_auth::AdminClaims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct TenantQuery {
    pub tenant_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateIdpRequest {
    pub tenant_id: Uuid,
    pub provider_type: String,
    pub name: String,
    pub enabled: bool,
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIdpRequest {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<serde_json::Value>,
}

fn idp_to_json(c: &IdpConfig) -> Value {
    json!({
        "id": c.id,
        "tenant_id": c.tenant_id,
        "provider_type": c.provider_type.to_string(),
        "name": c.name,
        "enabled": c.enabled,
        "created_at": c.created_at,
        "updated_at": c.updated_at,
    })
}

pub async fn list_idp_configs(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(q.tenant_id), IDP_CONFIGS, LIST).await?;
    let items = state.idp_configs.list(q.tenant_id).await?;
    let data: Vec<Value> = items.iter().map(idp_to_json).collect();
    Ok(Json(json!({ "idp_configs": data, "total": data.len() })))
}

pub async fn create_idp_config(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateIdpRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    require_perm(
        &state,
        &claims,
        Scope::Tenant(req.tenant_id),
        IDP_CONFIGS,
        CREATE,
    )
    .await?;
    let provider_type: IdpType = req
        .provider_type
        .parse()
        .map_err(|e: irongate_core::errors::CoreError| Error::BadRequest(e.to_string()))?;
    let now = OffsetDateTime::now_utc();
    let config = IdpConfig {
        id: Uuid::new_v4(),
        tenant_id: req.tenant_id,
        provider_type,
        name: req.name,
        enabled: req.enabled,
        config: req.config,
        created_at: now,
        updated_at: now,
    };
    let created = state.idp_configs.create(config).await?;
    Ok((StatusCode::CREATED, Json(idp_to_json(&created))))
}

pub async fn get_idp_config(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), IDP_CONFIGS, READ).await?;
    let config = state.idp_configs.get_by_id(id, tenant_id).await?;
    Ok(Json(idp_to_json(&config)))
}

pub async fn update_idp_config(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateIdpRequest>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), IDP_CONFIGS, UPDATE).await?;
    let mut config = state.idp_configs.get_by_id(id, tenant_id).await?;
    if let Some(name) = req.name {
        config.name = name;
    }
    if let Some(enabled) = req.enabled {
        config.enabled = enabled;
    }
    if let Some(cfg) = req.config {
        config.config = cfg;
    }
    config.updated_at = OffsetDateTime::now_utc();
    let updated = state.idp_configs.update(config).await?;
    Ok(Json(idp_to_json(&updated)))
}

pub async fn delete_idp_config(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), IDP_CONFIGS, DELETE).await?;
    state.idp_configs.delete(id, tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
