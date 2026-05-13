use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    handlers::admin_auth::AdminClaims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
}

fn session_to_json(s: &irongate_core::types::Session) -> Value {
    json!({
        "id": s.id,
        "user_id": s.user_id,
        "tenant_id": s.tenant_id,
        "idp_id": s.idp_id,
        "ip_address": s.ip_address,
        "user_agent": s.user_agent,
        "created_at": s.created_at,
        "expires_at": s.expires_at,
        "revoked_at": s.revoked_at,
    })
}

pub async fn list_sessions(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
) -> Result<Json<Value>> {
    let items = state
        .session_svc
        .sessions
        .list_for_user(q.user_id, q.tenant_id)
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    let data: Vec<Value> = items.iter().map(session_to_json).collect();
    Ok(Json(json!({ "sessions": data, "total": data.len() })))
}

pub async fn delete_session(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    state
        .session_svc
        .revoke_session(id)
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
