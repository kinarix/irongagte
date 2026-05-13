use std::sync::Arc;

use axum::{extract::State, Json};
use irongate_core::types::{
    op_action::LIST,
    op_resource::OPERATOR_PERMISSIONS,
    OperatorPermission,
};
use serde_json::{json, Value};

use crate::{
    authz_op::{require_perm, Scope},
    error::Result,
    handlers::admin_auth::AdminClaims,
    state::AppState,
};

fn permission_to_json(p: &OperatorPermission) -> Value {
    json!({
        "id": p.id,
        "resource": p.resource,
        "action": p.action,
        "description": p.description,
        "created_at": p.created_at,
    })
}

pub async fn list_operator_permissions(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>> {
    require_perm(
        &state,
        &claims,
        Scope::Global,
        OPERATOR_PERMISSIONS,
        LIST,
    )
    .await?;
    let items = state.operator_permissions.list().await?;
    let data: Vec<Value> = items.iter().map(permission_to_json).collect();
    Ok(Json(json!({ "permissions": data, "total": data.len() })))
}
