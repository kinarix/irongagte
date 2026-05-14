use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{extract::State, http::StatusCode, Json};
use irongate_core::{errors::StoreError, types::OperatorStatus};
use irongate_crypto::{hash::verify_password, jwt::sign};
use metrics;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    claims::{OperatorClaims, OPERATOR_ACTOR_TYPE, OPERATOR_AUDIENCE},
    error::{Error, Result},
    handlers::oidc::algo_for_key,
    state::AppState,
};

const OPERATOR_TOKEN_TTL_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<Value>> {
    let op = state
        .operators
        .get_by_email(&req.email)
        .await
        .map_err(|e| match e {
            StoreError::NotFound(_) => {
                metrics::counter!(
                    "irongate_login_attempts_total",
                    "result" => "invalid_credentials",
                )
                .increment(1);
                Error::Unauthorized("invalid credentials".into())
            }
            other => Error::Internal(other.to_string()),
        })?;

    if op.status != OperatorStatus::Active {
        metrics::counter!(
            "irongate_login_attempts_total",
            "result" => "not_active",
        )
        .increment(1);
        return Err(Error::Unauthorized("operator account is not active".into()));
    }

    let creds = state
        .operator_credentials
        .get_by_operator_id(op.id)
        .await
        .map_err(|e| match e {
            StoreError::NotFound(_) => {
                metrics::counter!(
                    "irongate_login_attempts_total",
                    "result" => "invalid_credentials",
                )
                .increment(1);
                Error::Unauthorized("invalid credentials".into())
            }
            other => Error::Internal(other.to_string()),
        })?;

    let valid = verify_password(&req.password, &creds.password_hash)
        .map_err(|_| Error::Internal("password verify failed".into()))?;
    if !valid {
        metrics::counter!(
            "irongate_login_attempts_total",
            "result" => "invalid_credentials",
        )
        .increment(1);
        return Err(Error::Unauthorized("invalid credentials".into()));
    }

    metrics::counter!("irongate_login_attempts_total", "result" => "success").increment(1);

    let _ = state.operators.touch_last_login(op.id).await;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let claims = OperatorClaims {
        sub: op.id.to_string(),
        iss: state.config.base_url.clone(),
        aud: OPERATOR_AUDIENCE.to_string(),
        exp: now + OPERATOR_TOKEN_TTL_SECS,
        iat: now,
        jti: Uuid::new_v4().to_string(),
        email: op.email.clone(),
        actor_type: OPERATOR_ACTOR_TYPE.to_string(),
    };

    let key = state.signing_key.load_full();
    let alg = algo_for_key(&key.algorithm);
    let token = sign(
        &claims,
        &key.private_key_pem,
        alg,
        Some(&key.id.to_string()),
    )
    .map_err(|e| Error::Internal(e.to_string()))?;

    Ok(Json(json!({
        "access_token": token,
        "token_type": "Bearer",
        "expires_in": OPERATOR_TOKEN_TTL_SECS,
        "operator": {
            "id": op.id,
            "email": op.email,
            "name": op.name,
        }
    })))
}

pub async fn me(claims: super::admin_auth::AdminClaims) -> Result<Json<Value>> {
    Ok(Json(json!({
        "id": claims.0.sub,
        "email": claims.0.email,
    })))
}

pub async fn logout() -> StatusCode {
    // Stateless JWT — client just drops the token.
    StatusCode::NO_CONTENT
}
