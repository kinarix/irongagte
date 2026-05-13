use std::sync::Arc;

use axum::{
    extract::State,
    http::HeaderMap,
    Form, Json,
};
use irongate_core::User;
use irongate_crypto::{jwt::sign, token::hash_token};
use serde::Deserialize;
use serde_json::{json, Value};
use base64ct::{Base64UrlUnpadded, Encoding};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    claims::{make_jti, now_secs, AccessTokenClaims, IdTokenClaims},
    error::{Error, Result},
    handlers::oidc::algo_for_key,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub scope: Option<String>,
    // authorization_code grant
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
}

pub async fn token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(req): Form<TokenRequest>,
) -> Result<Json<Value>> {
    match req.grant_type.as_str() {
        "password" => {
            let tenant_id = extract_tenant_id(&headers)?;
            password_grant(state, tenant_id, req).await
        }
        "refresh_token" => {
            let tenant_id = extract_tenant_id(&headers)?;
            refresh_token_grant(state, tenant_id, req).await
        }
        "authorization_code" => auth_code_grant(state, req).await,
        other => Err(Error::BadRequest(format!("unsupported grant_type: {other}"))),
    }
}

async fn password_grant(
    state: Arc<AppState>,
    tenant_id: Uuid,
    req: TokenRequest,
) -> Result<Json<Value>> {
    let username = req.username.ok_or_else(|| Error::BadRequest("username required".into()))?;
    let password = req.password.ok_or_else(|| Error::BadRequest("password required".into()))?;
    let client_id =
        req.client_id.ok_or_else(|| Error::BadRequest("client_id required".into()))?;

    let app = state
        .applications
        .get_by_client_id(&client_id, tenant_id)
        .await
        .map_err(|_| Error::Unauthorized("unknown client".into()))?;

    let user = state.password_svc.authenticate(&username, &password, tenant_id).await?;

    let scope = req.scope.unwrap_or_else(|| "openid profile email".into());

    let (_session, raw_refresh) = state
        .session_svc
        .create_session(
            user.id,
            tenant_id,
            app.id,
            scope.clone(),
            None,
            None,
            None,
            state.config.session.ttl_seconds as i64,
            app.refresh_token_ttl,
        )
        .await?;

    let access_token =
        mint_access_token(&state, user.id, tenant_id, &app.client_id, &scope, app.access_token_ttl)?;

    let mut response = json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": app.access_token_ttl,
        "refresh_token": raw_refresh,
        "scope": scope,
    });

    if scope.split_whitespace().any(|s| s == "openid") {
        let id_token = mint_id_token(
            &state,
            &user,
            tenant_id,
            &app.client_id,
            state.config.tokens.id_token_ttl_seconds,
        )?;
        response["id_token"] = json!(id_token);
    }

    Ok(Json(response))
}

async fn refresh_token_grant(
    state: Arc<AppState>,
    tenant_id: Uuid,
    req: TokenRequest,
) -> Result<Json<Value>> {
    let raw_token =
        req.refresh_token.ok_or_else(|| Error::BadRequest("refresh_token required".into()))?;

    let hash = hash_token(&raw_token);
    let old_rt = state
        .refresh_tokens
        .get_by_hash(&hash)
        .await
        .map_err(|_| Error::Unauthorized("invalid refresh token".into()))?;

    if old_rt.revoked_at.is_some() || old_rt.expires_at < time::OffsetDateTime::now_utc() {
        return Err(Error::Unauthorized("refresh token expired or revoked".into()));
    }

    let scope = old_rt.scope.clone();
    let app_id = old_rt.application_id;

    let app = state
        .applications
        .get_by_id(app_id, tenant_id)
        .await
        .map_err(|_| Error::Unauthorized("application not found".into()))?;

    let (session, new_raw) = state
        .session_svc
        .rotate_refresh_token(&raw_token, tenant_id, app.refresh_token_ttl)
        .await?;

    let access_token = mint_access_token(
        &state,
        session.user_id,
        tenant_id,
        &app.client_id,
        &scope,
        app.access_token_ttl,
    )?;

    Ok(Json(json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": app.access_token_ttl,
        "refresh_token": new_raw,
        "scope": scope,
    })))
}

async fn auth_code_grant(
    state: Arc<AppState>,
    req: TokenRequest,
) -> Result<Json<Value>> {
    let code = req.code.ok_or_else(|| Error::BadRequest("code required".into()))?;
    let redirect_uri =
        req.redirect_uri.ok_or_else(|| Error::BadRequest("redirect_uri required".into()))?;
    let code_verifier =
        req.code_verifier.ok_or_else(|| Error::BadRequest("code_verifier required".into()))?;
    let client_id =
        req.client_id.ok_or_else(|| Error::BadRequest("client_id required".into()))?;

    // Retrieve-and-delete the auth code (one-time use).
    let data = state
        .auth_codes
        .take_code(&code)
        .await
        .map_err(|e| Error::Internal(e.to_string()))?
        .ok_or_else(|| Error::Unauthorized("invalid or expired authorization code".into()))?;

    // Verify client_id + redirect_uri match what was stored.
    if data.client_id != client_id {
        return Err(Error::Unauthorized("client_id mismatch".into()));
    }
    if data.redirect_uri != redirect_uri {
        return Err(Error::Unauthorized("redirect_uri mismatch".into()));
    }

    // Verify PKCE: SHA-256(code_verifier) must equal stored code_challenge (base64url, no padding).
    let hash = Sha256::digest(code_verifier.as_bytes());
    let computed = Base64UrlUnpadded::encode_string(&hash);
    if computed != data.code_challenge {
        return Err(Error::Unauthorized("PKCE verification failed".into()));
    }

    let tenant_id = data.tenant_id;
    let app = state
        .applications
        .get_by_client_id(&data.client_id, tenant_id)
        .await
        .map_err(|_| Error::Unauthorized("unknown client".into()))?;

    let user = state
        .users
        .get_by_id(data.user_id, tenant_id)
        .await
        .map_err(|_| Error::Unauthorized("user not found".into()))?;

    let (_session, raw_refresh) = state
        .session_svc
        .create_session(
            user.id,
            tenant_id,
            app.id,
            data.scope.clone(),
            None,
            None,
            None,
            state.config.session.ttl_seconds as i64,
            app.refresh_token_ttl,
        )
        .await?;

    let access_token = mint_access_token(
        &state,
        user.id,
        tenant_id,
        &app.client_id,
        &data.scope,
        app.access_token_ttl,
    )?;

    let mut response = json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": app.access_token_ttl,
        "refresh_token": raw_refresh,
        "scope": data.scope,
    });

    if data.scope.split_whitespace().any(|s| s == "openid") {
        let id_token = mint_id_token(
            &state,
            &user,
            tenant_id,
            &app.client_id,
            state.config.tokens.id_token_ttl_seconds,
        )?;
        response["id_token"] = json!(id_token);
    }

    Ok(Json(response))
}

fn mint_access_token(
    state: &AppState,
    user_id: Uuid,
    tenant_id: Uuid,
    aud: &str,
    scope: &str,
    ttl_secs: i64,
) -> Result<String> {
    let now = now_secs();
    let claims = AccessTokenClaims {
        sub: user_id.to_string(),
        iss: state.config.base_url.clone(),
        aud: aud.to_string(),
        exp: now + ttl_secs as u64,
        iat: now,
        jti: make_jti(),
        scope: scope.to_string(),
        tenant_id: tenant_id.to_string(),
    };
    let alg = algo_for_key(&state.signing_key.algorithm);
    sign(&claims, &state.signing_key.private_key_pem, alg, Some(&state.signing_key.id.to_string()))
        .map_err(|e| Error::Internal(e.to_string()))
}

fn mint_id_token(
    state: &AppState,
    user: &User,
    tenant_id: Uuid,
    aud: &str,
    ttl_secs: i64,
) -> Result<String> {
    let now = now_secs();
    let claims = IdTokenClaims {
        sub: user.id.to_string(),
        iss: state.config.base_url.clone(),
        aud: aud.to_string(),
        exp: now + ttl_secs as u64,
        iat: now,
        email: Some(user.email.clone()),
        name: user.name.clone(),
        tenant_id: tenant_id.to_string(),
    };
    let alg = algo_for_key(&state.signing_key.algorithm);
    sign(&claims, &state.signing_key.private_key_pem, alg, Some(&state.signing_key.id.to_string()))
        .map_err(|e| Error::Internal(e.to_string()))
}

fn extract_tenant_id(headers: &HeaderMap) -> Result<Uuid> {
    headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| Error::BadRequest("missing or invalid X-Tenant-ID header".into()))
}
