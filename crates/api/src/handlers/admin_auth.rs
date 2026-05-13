use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use irongate_crypto::jwt::verify;
use jsonwebtoken::Validation;

use crate::{
    claims::AccessTokenClaims,
    error::Error,
    handlers::oidc::algo_for_key,
    state::AppState,
};

pub struct AdminClaims(pub AccessTokenClaims);

impl FromRequestParts<Arc<AppState>> for AdminClaims {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| Error::Unauthorized("missing Bearer token".into()))?;

        let algo = algo_for_key(&state.signing_key.algorithm);
        let mut validation = Validation::new(algo);
        validation.validate_aud = false;

        let claims = verify::<AccessTokenClaims>(
            token,
            &state.signing_key.public_key_pem,
            algo,
            &validation,
        )
        .map_err(|_| Error::Unauthorized("invalid or expired token".into()))?
        .claims;

        if !claims.scope.split_whitespace().any(|s| s == "admin:*") {
            return Err(Error::Forbidden);
        }

        Ok(AdminClaims(claims))
    }
}
