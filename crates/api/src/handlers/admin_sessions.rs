use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::types::{
    op_action::{DELETE, LIST},
    op_resource::SESSIONS,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    authz_op::{require_perm, Scope},
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
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionQuery>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Tenant(q.tenant_id), SESSIONS, LIST).await?;
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
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    // Look up the session before authorizing so we check against the
    // session's owning tenant — never trust the URL alone for scope.
    let session = state.session_svc.sessions.get_by_id(id).await?;
    require_perm(
        &state,
        &claims,
        Scope::Tenant(session.tenant_id),
        SESSIONS,
        DELETE,
    )
    .await?;
    state
        .session_svc
        .revoke_session(id)
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
