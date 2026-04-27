use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation error: {0}")]
    Validation(String),
}

#[derive(Debug, Error)]
pub enum CryptoError {}

#[derive(Debug, Error)]
pub enum StoreError {}

#[derive(Debug, Error)]
pub enum AuthError {}

#[derive(Debug, Error)]
pub enum FederationError {}

#[derive(Debug, Error)]
pub enum AuthzError {}

#[derive(Debug, Error)]
pub enum ScimError {}

#[derive(Debug, Error)]
pub enum WebAuthnError {}

#[derive(Debug, Error)]
pub enum ApiError {}

#[derive(Debug, Error)]
pub enum IdpError {
    #[error("authorization failed: {0}")]
    AuthorizationFailed(String),
    #[error("token exchange failed: {0}")]
    TokenExchangeFailed(String),
    #[error("provider error: {0}")]
    ProviderError(String),
}
