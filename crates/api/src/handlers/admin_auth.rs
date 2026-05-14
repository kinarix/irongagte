use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use irongate_crypto::jwt::verify;
use jsonwebtoken::Validation;

use crate::{
    claims::{OperatorClaims, OPERATOR_ACTOR_TYPE, OPERATOR_AUDIENCE},
    error::Error,
    handlers::oidc::algo_for_key,
    state::AppState,
};

/// Extractor that requires a valid Operator JWT. End-user access tokens are
/// rejected — Operators and end-users are strictly separate authentication
/// domains in irongate.
pub struct AdminClaims(pub OperatorClaims);

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
        validation.set_audience(&[OPERATOR_AUDIENCE]);

        let claims =
            verify::<OperatorClaims>(token, &state.signing_key.public_key_pem, algo, &validation)
                .map_err(|_| Error::Unauthorized("invalid or expired token".into()))?
                .claims;

        if claims.actor_type != OPERATOR_ACTOR_TYPE {
            return Err(Error::Forbidden);
        }

        Ok(AdminClaims(claims))
    }
}
