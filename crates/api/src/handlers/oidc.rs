use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use irongate_crypto::{
    jwks::{build_jwks, jwks_to_json},
    jwt::verify,
    keys::KeyAlgorithm,
};
use jsonwebtoken::{Algorithm, Validation};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    claims::AccessTokenClaims,
    error::{Error, Result},
    state::AppState,
};

pub async fn discovery(State(state): State<Arc<AppState>>) -> Json<Value> {
    let base = &state.config.base_url;
    Json(json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/oauth2/authorize"),
        "token_endpoint": format!("{base}/oauth2/token"),
        "userinfo_endpoint": format!("{base}/oauth2/userinfo"),
        "jwks_uri": format!("{base}/.well-known/jwks.json"),
        "revocation_endpoint": format!("{base}/oauth2/revoke"),
        "introspection_endpoint": format!("{base}/oauth2/introspect"),
        "response_types_supported": ["code"],
        "grant_types_supported": [
            "authorization_code",
            "refresh_token",
            "password",
            "client_credentials"
        ],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "scopes_supported": ["openid", "profile", "email"],
        "token_endpoint_auth_methods_supported": [
            "client_secret_post",
            "client_secret_basic"
        ],
        "claims_supported": ["sub", "iss", "aud", "exp", "iat", "email", "name"],
    }))
}

pub async fn jwks(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse> {
    let key = state.signing_key.as_ref().clone();
    let jwks = build_jwks(&[key]).map_err(|e| Error::Internal(e.to_string()))?;
    let body = jwks_to_json(&jwks).map_err(|e| Error::Internal(e.to_string()))?;
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body,
    ))
}

pub async fn userinfo(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let token = extract_bearer(&headers)?;

    let algorithm = algo_for_key(&state.signing_key.algorithm);
    let mut validation = Validation::new(algorithm);
    validation.required_spec_claims.clear();
    validation.validate_aud = false;

    let claims = verify::<AccessTokenClaims>(
        token,
        &state.signing_key.public_key_pem,
        algorithm,
        &validation,
    )
    .map_err(|_| Error::Unauthorized("invalid access token".into()))?
    .claims;

    let user_id =
        Uuid::parse_str(&claims.sub).map_err(|_| Error::Unauthorized("invalid subject".into()))?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| Error::Unauthorized("invalid tenant_id claim".into()))?;

    let user = state.users.get_by_id(user_id, tenant_id).await?;

    Ok(Json(json!({
        "sub": user.id,
        "email": user.email,
        "email_verified": user.email_verified,
        "name": user.name,
        "given_name": user.given_name,
        "family_name": user.family_name,
        "picture": user.picture_url,
    })))
}

pub fn extract_bearer(headers: &HeaderMap) -> Result<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| Error::Unauthorized("missing Bearer token".into()))
}

pub fn algo_for_key(alg: &KeyAlgorithm) -> Algorithm {
    match alg {
        KeyAlgorithm::Rs256 => Algorithm::RS256,
        KeyAlgorithm::Es256 => Algorithm::ES256,
    }
}
