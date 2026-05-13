use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::Result,
    handlers::admin_auth::AdminClaims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub tenant_id: Uuid,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}
fn default_limit() -> i64 {
    50
}

fn audit_to_json(e: &irongate_core::types::AuditEvent) -> Value {
    json!({
        "id": e.id,
        "tenant_id": e.tenant_id,
        "event_type": e.event_type,
        "actor_id": e.actor_id,
        "target_id": e.target_id,
        "ip_address": e.ip_address,
        "metadata": e.metadata,
        "created_at": e.created_at,
    })
}

pub async fn list_audit_events(
    _claims: AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Value>> {
    let items = state.audit.list(q.tenant_id, q.limit, q.offset).await?;
    let data: Vec<Value> = items.iter().map(audit_to_json).collect();
    Ok(Json(json!({ "events": data, "total": data.len() })))
}
