use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use irongate_core::errors::StoreError;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScimError {
    #[error("resource not found")]
    NotFound,
    #[error("resource already exists")]
    Conflict,
    #[error("unsupported operation: {0}")]
    UnsupportedOperation(String),
    #[error("invalid filter: {0}")]
    InvalidFilter(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<StoreError> for ScimError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::NotFound(_) => ScimError::NotFound,
            StoreError::Conflict(_) => ScimError::Conflict,
            other => ScimError::Internal(other.to_string()),
        }
    }
}

impl IntoResponse for ScimError {
    fn into_response(self) -> Response {
        let (status, detail) = match &self {
            ScimError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            ScimError::Conflict => (StatusCode::CONFLICT, self.to_string()),
            ScimError::UnsupportedOperation(s) => (StatusCode::NOT_IMPLEMENTED, s.clone()),
            ScimError::InvalidFilter(s) => (StatusCode::BAD_REQUEST, s.clone()),
            ScimError::BadRequest(s) => (StatusCode::BAD_REQUEST, s.clone()),
            ScimError::Internal(s) => (StatusCode::INTERNAL_SERVER_ERROR, s.clone()),
        };
        let body = json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "status": status.as_u16().to_string(),
            "detail": detail,
        });
        (status, Json(body)).into_response()
    }
}
