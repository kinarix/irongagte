//! Admin handlers for the new claim model.
//!
//! Three resources live under `/admin/v1/tenants/<tid>/claims/...`:
//!
//! * `claim_definitions` — declared per-application; the unprefixed name + type
//!   that the app emits.
//! * `group_claims` — `(group, claim_def, value)` rows; members inherit.
//! * `user_claims` — direct user assignments; override (scalar) or merge
//!   (multi) with group-derived values.
//!
//! `require_perm` uses the `claims` resource for every action since the global
//! Claims page in the admin UI surfaces all three.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::types::{
    op_action::{ASSIGN, CREATE, DELETE, LIST, READ, REVOKE, UPDATE},
    op_resource::CLAIMS,
    validate_claim_key, ClaimDefinition, ClaimType, GroupClaim, UserClaim,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    authz_op::{require_perm, Scope},
    error::{Error, Result},
    handlers::admin_auth::AdminClaims,
    state::AppState,
};

// ── Claim definitions ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TenantQuery {
    pub tenant_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ListDefsQuery {
    pub tenant_id: Uuid,
    pub application_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateClaimDefRequest {
    pub tenant_id: Uuid,
    pub application_id: Uuid,
    pub key: String,
    pub claim_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateClaimDefRequest {
    pub key: Option<String>,
    pub claim_type: Option<String>,
    pub description: Option<String>,
}

fn def_to_json(d: &ClaimDefinition) -> Value {
    json!({
        "id": d.id,
        "application_id": d.application_id,
        "key": d.key,
        "claim_type": d.claim_type.as_str(),
        "description": d.description,
        "created_at": d.created_at,
        "updated_at": d.updated_at,
    })
}

pub async fn list_claim_definitions(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListDefsQuery>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(q.tenant_id), CLAIMS, LIST).await?;
    let defs = if let Some(app_id) = q.application_id {
        state.claim_definitions.list_for_app(app_id).await?
    } else {
        state.claim_definitions.list_for_tenant(q.tenant_id).await?
    };
    let data: Vec<Value> = defs.iter().map(def_to_json).collect();
    Ok(Json(json!({ "claim_definitions": data, "total": data.len() })))
}

pub async fn create_claim_definition(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateClaimDefRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    require_perm(&state, &claims, Scope::Tenant(req.tenant_id), CLAIMS, CREATE).await?;
    // Verify the application belongs to this tenant.
    let _app = state
        .applications
        .get_by_id(req.application_id, req.tenant_id)
        .await?;
    validate_claim_key(&req.key).map_err(Error::BadRequest)?;
    let claim_type =
        ClaimType::from_str(&req.claim_type).map_err(|e| Error::BadRequest(e.to_string()))?;
    let now = OffsetDateTime::now_utc();
    let def = ClaimDefinition {
        id: Uuid::new_v4(),
        application_id: req.application_id,
        key: req.key,
        claim_type,
        description: req.description,
        created_at: now,
        updated_at: now,
    };
    let created = state.claim_definitions.create(def).await?;
    Ok((StatusCode::CREATED, Json(def_to_json(&created))))
}

pub async fn get_claim_definition(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), CLAIMS, READ).await?;
    let def = state.claim_definitions.get_by_id(id).await?;
    // Tenant ownership check via the parent application.
    let _ = state
        .applications
        .get_by_id(def.application_id, tenant_id)
        .await?;
    Ok(Json(def_to_json(&def)))
}

pub async fn update_claim_definition(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateClaimDefRequest>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), CLAIMS, UPDATE).await?;
    let mut def = state.claim_definitions.get_by_id(id).await?;
    let _ = state
        .applications
        .get_by_id(def.application_id, tenant_id)
        .await?;
    if let Some(key) = req.key {
        validate_claim_key(&key).map_err(Error::BadRequest)?;
        def.key = key;
    }
    if let Some(t) = req.claim_type {
        def.claim_type = ClaimType::from_str(&t).map_err(|e| Error::BadRequest(e.to_string()))?;
    }
    if let Some(d) = req.description {
        def.description = Some(d);
    }
    def.updated_at = OffsetDateTime::now_utc();
    let updated = state.claim_definitions.update(def).await?;
    Ok(Json(def_to_json(&updated)))
}

pub async fn delete_claim_definition(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path((tenant_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    require_perm(&state, &claims, Scope::Tenant(tenant_id), CLAIMS, DELETE).await?;
    let def = state.claim_definitions.get_by_id(id).await?;
    let _ = state
        .applications
        .get_by_id(def.application_id, tenant_id)
        .await?;
    state.claim_definitions.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Group claims ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GroupClaimAssignment {
    pub tenant_id: Uuid,
    pub group_id: Uuid,
    pub claim_def_id: Uuid,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupClaimsQuery {
    pub tenant_id: Uuid,
    pub group_id: Uuid,
}

fn group_claim_to_json(c: &GroupClaim) -> Value {
    json!({
        "group_id": c.group_id,
        "claim_def_id": c.claim_def_id,
        "value": c.value,
        "created_at": c.created_at,
    })
}

pub async fn list_group_claims(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<GroupClaimsQuery>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(q.tenant_id), CLAIMS, LIST).await?;
    let rows = state.group_claims.list_for_group(q.group_id).await?;
    let data: Vec<Value> = rows.iter().map(group_claim_to_json).collect();
    Ok(Json(json!({ "group_claims": data, "total": data.len() })))
}

pub async fn assign_group_claim(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<GroupClaimAssignment>,
) -> Result<(StatusCode, Json<Value>)> {
    require_perm(&state, &claims, Scope::Tenant(req.tenant_id), CLAIMS, ASSIGN).await?;
    let row = state
        .group_claims
        .assign(req.group_id, req.claim_def_id, &req.value)
        .await?;
    Ok((StatusCode::CREATED, Json(group_claim_to_json(&row))))
}

pub async fn revoke_group_claim(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<GroupClaimAssignment>,
) -> Result<StatusCode> {
    require_perm(&state, &claims, Scope::Tenant(req.tenant_id), CLAIMS, REVOKE).await?;
    state
        .group_claims
        .revoke(req.group_id, req.claim_def_id, &req.value)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── User claims ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UserClaimAssignment {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub claim_def_id: Uuid,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct UserClaimsQuery {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}

fn user_claim_to_json(c: &UserClaim) -> Value {
    json!({
        "user_id": c.user_id,
        "claim_def_id": c.claim_def_id,
        "value": c.value,
        "created_at": c.created_at,
    })
}

pub async fn list_user_claims(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<UserClaimsQuery>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(q.tenant_id), CLAIMS, LIST).await?;
    let rows = state.user_claims.list_for_user(q.user_id).await?;
    let data: Vec<Value> = rows.iter().map(user_claim_to_json).collect();
    Ok(Json(json!({ "user_claims": data, "total": data.len() })))
}

pub async fn assign_user_claim(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<UserClaimAssignment>,
) -> Result<(StatusCode, Json<Value>)> {
    require_perm(&state, &claims, Scope::Tenant(req.tenant_id), CLAIMS, ASSIGN).await?;
    let row = state
        .user_claims
        .assign(req.user_id, req.claim_def_id, &req.value)
        .await?;
    Ok((StatusCode::CREATED, Json(user_claim_to_json(&row))))
}

pub async fn revoke_user_claim(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<UserClaimAssignment>,
) -> Result<StatusCode> {
    require_perm(&state, &claims, Scope::Tenant(req.tenant_id), CLAIMS, REVOKE).await?;
    state
        .user_claims
        .revoke(req.user_id, req.claim_def_id, &req.value)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Effective claims preview for a user against an application. Resolves the
/// same way token-mint does, so admins can inspect what the JWT will contain.
#[derive(Debug, Deserialize)]
pub struct EffectiveQuery {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub application_id: Uuid,
}

pub async fn preview_effective_claims(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<EffectiveQuery>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(q.tenant_id), CLAIMS, READ).await?;
    let app = state
        .applications
        .get_by_id(q.application_id, q.tenant_id)
        .await?;
    let resolved = state
        .authz_svc
        .resolve_claims_for_app(q.user_id, app.id, &app.claim_prefix)
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(Json(json!({ "claims": resolved })))
}
