use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use irongate_core::types::{
    op_action::{CREATE, DELETE, LIST, READ, UPDATE},
    op_resource::OPERATORS,
    Operator, OperatorCredentials, OperatorStatus,
};
use irongate_crypto::hash::hash_password;
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
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

#[derive(Debug, Deserialize)]
pub struct CreateOperatorRequest {
    pub email: String,
    pub name: Option<String>,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOperatorRequest {
    pub email: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PasswordChangeRequest {
    pub password: String,
}

fn operator_to_json(o: &Operator) -> Value {
    json!({
        "id": o.id,
        "email": o.email,
        "name": o.name,
        "status": o.status.as_str(),
        "created_at": o.created_at,
        "updated_at": o.updated_at,
        "last_login_at": o.last_login_at,
    })
}

pub async fn list_operators(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Global, OPERATORS, LIST).await?;
    let items = state.operators.list(q.limit, q.offset).await?;
    let data: Vec<Value> = items.iter().map(operator_to_json).collect();
    Ok(Json(json!({ "operators": data, "total": data.len() })))
}

pub async fn create_operator(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateOperatorRequest>,
) -> Result<(StatusCode, Json<Value>)> {
    require_perm(&state, &claims, Scope::Global, OPERATORS, CREATE).await?;
    let now = OffsetDateTime::now_utc();
    let op = Operator {
        id: Uuid::new_v4(),
        email: req.email,
        name: req.name,
        status: OperatorStatus::Active,
        created_at: now,
        updated_at: now,
        last_login_at: None,
        deleted_at: None,
    };
    let created = state.operators.create(op).await?;

    let hash = hash_password(&req.password)
        .map_err(|e| Error::Internal(format!("password hash failed: {e}")))?;
    let creds = OperatorCredentials {
        operator_id: created.id,
        password_hash: hash,
        created_at: now,
        updated_at: now,
    };
    state.operator_credentials.create(creds).await?;

    Ok((StatusCode::CREATED, Json(operator_to_json(&created))))
}

pub async fn get_operator(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Global, OPERATORS, READ).await?;
    let op = state.operators.get_by_id(id).await?;
    Ok(Json(operator_to_json(&op)))
}

pub async fn update_operator(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOperatorRequest>,
) -> Result<Json<Value>> {
    require_perm(&state, &claims, Scope::Global, OPERATORS, UPDATE).await?;
    let mut op = state.operators.get_by_id(id).await?;
    if let Some(email) = req.email {
        op.email = email;
    }
    if let Some(name) = req.name {
        op.name = Some(name);
    }
    if let Some(status_str) = req.status {
        op.status = status_str
            .parse()
            .map_err(|_| Error::BadRequest(format!("invalid status '{status_str}'")))?;
    }
    op.updated_at = OffsetDateTime::now_utc();
    let updated = state.operators.update(op).await?;
    Ok(Json(operator_to_json(&updated)))
}

pub async fn delete_operator(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    require_perm(&state, &claims, Scope::Global, OPERATORS, DELETE).await?;
    if claims.sub == id.to_string() {
        return Err(Error::BadRequest(
            "operators cannot delete their own account".into(),
        ));
    }
    state.operators.soft_delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn change_password(
    AdminClaims(claims): AdminClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<PasswordChangeRequest>,
) -> Result<StatusCode> {
    require_perm(&state, &claims, Scope::Global, OPERATORS, UPDATE).await?;
    let hash = hash_password(&req.password)
        .map_err(|e| Error::Internal(format!("password hash failed: {e}")))?;
    let now = OffsetDateTime::now_utc();
    match state.operator_credentials.get_by_operator_id(id).await {
        Ok(mut existing) => {
            existing.password_hash = hash;
            existing.updated_at = now;
            state.operator_credentials.update(existing).await?;
        }
        Err(_) => {
            let c = OperatorCredentials {
                operator_id: id,
                password_hash: hash,
                created_at: now,
                updated_at: now,
            };
            state.operator_credentials.create(c).await?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}
