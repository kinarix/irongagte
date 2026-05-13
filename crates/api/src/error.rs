use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use irongate_core::errors::{AuthError, CryptoError, StoreError};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden")]
    Forbidden,

    #[error("not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            Error::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            Error::Forbidden => (StatusCode::FORBIDDEN, "forbidden".into()),
            Error::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            Error::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            Error::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<StoreError> for Error {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::NotFound(m) => Error::NotFound(m),
            StoreError::Conflict(m) => Error::Conflict(m),
            e => Error::Internal(e.to_string()),
        }
    }
}

impl From<AuthError> for Error {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::InvalidCredentials => Error::Unauthorized("invalid credentials".into()),
            AuthError::AccountSuspended => Error::Unauthorized("account suspended".into()),
            AuthError::TokenExpired => Error::Unauthorized("token expired".into()),
            AuthError::SessionExpired => Error::Unauthorized("session expired".into()),
            e => Error::Internal(e.to_string()),
        }
    }
}

impl From<CryptoError> for Error {
    fn from(e: CryptoError) -> Self {
        Error::Internal(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
