use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("key generation failed: {0}")]
    KeyGeneration(String),
    #[error("signing failed: {0}")]
    Signing(String),
    #[error("verification failed: {0}")]
    Verification(String),
    #[error("token expired")]
    TokenExpired,
    #[error("invalid token: {0}")]
    InvalidToken(String),
    #[error("hashing failed: {0}")]
    Hashing(String),
    #[error("invalid key: {0}")]
    InvalidKey(String),
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("cache error: {0}")]
    Cache(String),
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("account locked")]
    AccountLocked,
    #[error("account suspended")]
    AccountSuspended,
    #[error("token expired")]
    TokenExpired,
    #[error("token already used")]
    TokenAlreadyUsed,
    #[error("mfa required")]
    MfaRequired,
    #[error("invalid mfa code")]
    InvalidMfaCode,
    #[error("session expired")]
    SessionExpired,
    #[error("session not found")]
    SessionNotFound,
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum FederationError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("token exchange failed: {0}")]
    TokenExchange(String),
    #[error("user provisioning failed: {0}")]
    Provisioning(String),
    #[error("account link conflict: email {0} already linked to another identity")]
    LinkConflict(String),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("http error: {0}")]
    Http(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum AuthzError {
    #[error("permission denied")]
    PermissionDenied,
    #[error("role not found: {0}")]
    RoleNotFound(String),
    #[error("invalid policy: {0}")]
    InvalidPolicy(String),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
}

#[derive(Debug, Error)]
pub enum ScimError {
    #[error("invalid filter: {0}")]
    InvalidFilter(String),
    #[error("unsupported operation: {0}")]
    UnsupportedOperation(String),
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
}

#[derive(Debug, Error)]
pub enum WebAuthnError {
    #[error("registration failed: {0}")]
    Registration(String),
    #[error("authentication failed: {0}")]
    Authentication(String),
    #[error("challenge mismatch")]
    ChallengeMismatch,
    #[error("credential not found")]
    CredentialNotFound,
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum IdpError {
    #[error("authorization failed: {0}")]
    AuthorizationFailed(String),
    #[error("token exchange failed: {0}")]
    TokenExchangeFailed(String),
    #[error("provider unavailable: {0}")]
    ProviderUnavailable(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("configuration error: {0}")]
    Configuration(String),
}
