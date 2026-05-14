use crate::errors::*;

// ── CoreError ─────────────────────────────────────────────────────────────────

#[test]
fn core_error_display() {
    assert_eq!(
        CoreError::NotFound("user".into()).to_string(),
        "not found: user"
    );
    assert_eq!(
        CoreError::Validation("email required".into()).to_string(),
        "validation error: email required"
    );
    assert_eq!(
        CoreError::Internal("db down".into()).to_string(),
        "internal error: db down"
    );
}

// ── CryptoError ───────────────────────────────────────────────────────────────

#[test]
fn crypto_error_display() {
    assert_eq!(
        CryptoError::KeyGeneration("rng failed".into()).to_string(),
        "key generation failed: rng failed"
    );
    assert_eq!(
        CryptoError::Signing("bad key".into()).to_string(),
        "signing failed: bad key"
    );
    assert_eq!(
        CryptoError::Verification("sig mismatch".into()).to_string(),
        "verification failed: sig mismatch"
    );
    assert_eq!(CryptoError::TokenExpired.to_string(), "token expired");
    assert_eq!(
        CryptoError::InvalidToken("malformed".into()).to_string(),
        "invalid token: malformed"
    );
    assert_eq!(
        CryptoError::Hashing("argon2 error".into()).to_string(),
        "hashing failed: argon2 error"
    );
    assert_eq!(
        CryptoError::InvalidKey("bad pem".into()).to_string(),
        "invalid key: bad pem"
    );
}

// ── StoreError ────────────────────────────────────────────────────────────────

#[test]
fn store_error_display() {
    assert_eq!(
        StoreError::Database("conn refused".into()).to_string(),
        "database error: conn refused"
    );
    assert_eq!(
        StoreError::NotFound("tenant".into()).to_string(),
        "not found: tenant"
    );
    assert_eq!(
        StoreError::Conflict("duplicate email".into()).to_string(),
        "conflict: duplicate email"
    );
    assert_eq!(
        StoreError::Cache("redis timeout".into()).to_string(),
        "cache error: redis timeout"
    );
}

// ── AuthError ─────────────────────────────────────────────────────────────────

#[test]
fn auth_error_display() {
    assert_eq!(
        AuthError::InvalidCredentials.to_string(),
        "invalid credentials"
    );
    assert_eq!(AuthError::AccountLocked.to_string(), "account locked");
    assert_eq!(AuthError::AccountSuspended.to_string(), "account suspended");
    assert_eq!(AuthError::TokenExpired.to_string(), "token expired");
    assert_eq!(
        AuthError::TokenAlreadyUsed.to_string(),
        "token already used"
    );
    assert_eq!(AuthError::MfaRequired.to_string(), "mfa required");
    assert_eq!(AuthError::InvalidMfaCode.to_string(), "invalid mfa code");
    assert_eq!(AuthError::SessionExpired.to_string(), "session expired");
    assert_eq!(AuthError::SessionNotFound.to_string(), "session not found");
    assert_eq!(
        AuthError::Internal("oops".into()).to_string(),
        "internal error: oops"
    );
}

#[test]
fn auth_error_from_store_error() {
    let store_err = StoreError::NotFound("user-123".into());
    let auth_err = AuthError::from(store_err);
    assert!(matches!(auth_err, AuthError::Store(_)));
    assert!(auth_err.to_string().contains("not found: user-123"));
}

// ── FederationError ───────────────────────────────────────────────────────────

#[test]
fn federation_error_display() {
    assert_eq!(
        FederationError::Provider("google down".into()).to_string(),
        "provider error: google down"
    );
    assert_eq!(
        FederationError::TokenExchange("bad code".into()).to_string(),
        "token exchange failed: bad code"
    );
    assert_eq!(
        FederationError::Provisioning("missing email".into()).to_string(),
        "user provisioning failed: missing email"
    );
    assert_eq!(
        FederationError::LinkConflict("alice@example.com".into()).to_string(),
        "account link conflict: email alice@example.com already linked to another identity"
    );
    assert_eq!(
        FederationError::Http("timeout".into()).to_string(),
        "http error: timeout"
    );
    assert_eq!(
        FederationError::Internal("crash".into()).to_string(),
        "internal error: crash"
    );
}

#[test]
fn federation_error_from_store_error() {
    let store_err = StoreError::Database("pg gone".into());
    let fed_err = FederationError::from(store_err);
    assert!(matches!(fed_err, FederationError::Store(_)));
    assert!(fed_err.to_string().contains("database error: pg gone"));
}

// ── AuthzError ────────────────────────────────────────────────────────────────

#[test]
fn authz_error_display() {
    assert_eq!(
        AuthzError::PermissionDenied.to_string(),
        "permission denied"
    );
    assert_eq!(
        AuthzError::RoleNotFound("superadmin".into()).to_string(),
        "role not found: superadmin"
    );
    assert_eq!(
        AuthzError::InvalidPolicy("bad syntax".into()).to_string(),
        "invalid policy: bad syntax"
    );
}

#[test]
fn authz_error_from_store_error() {
    let store_err = StoreError::Conflict("dup role".into());
    let authz_err = AuthzError::from(store_err);
    assert!(matches!(authz_err, AuthzError::Store(_)));
}

// ── ScimError ─────────────────────────────────────────────────────────────────

#[test]
fn scim_error_display() {
    assert_eq!(
        ScimError::InvalidFilter("bad attr".into()).to_string(),
        "invalid filter: bad attr"
    );
    assert_eq!(
        ScimError::UnsupportedOperation("bulk".into()).to_string(),
        "unsupported operation: bulk"
    );
    assert_eq!(
        ScimError::NotFound("user-abc".into()).to_string(),
        "resource not found: user-abc"
    );
    assert_eq!(
        ScimError::Conflict("duplicate userName".into()).to_string(),
        "conflict: duplicate userName"
    );
}

#[test]
fn scim_error_from_store_error() {
    let store_err = StoreError::Cache("redis down".into());
    let scim_err = ScimError::from(store_err);
    assert!(matches!(scim_err, ScimError::Store(_)));
}

// ── WebAuthnError ─────────────────────────────────────────────────────────────

#[test]
fn webauthn_error_display() {
    assert_eq!(
        WebAuthnError::Registration("bad cbor".into()).to_string(),
        "registration failed: bad cbor"
    );
    assert_eq!(
        WebAuthnError::Authentication("sig invalid".into()).to_string(),
        "authentication failed: sig invalid"
    );
    assert_eq!(
        WebAuthnError::ChallengeMismatch.to_string(),
        "challenge mismatch"
    );
    assert_eq!(
        WebAuthnError::CredentialNotFound.to_string(),
        "credential not found"
    );
    assert_eq!(
        WebAuthnError::Internal("panic".into()).to_string(),
        "internal error: panic"
    );
}

#[test]
fn webauthn_error_from_store_error() {
    let store_err = StoreError::NotFound("credential-xyz".into());
    let wa_err = WebAuthnError::from(store_err);
    assert!(matches!(wa_err, WebAuthnError::Store(_)));
}

// ── ApiError ──────────────────────────────────────────────────────────────────

#[test]
fn api_error_display() {
    assert_eq!(
        ApiError::BadRequest("missing field".into()).to_string(),
        "bad request: missing field"
    );
    assert_eq!(ApiError::Unauthorized.to_string(), "unauthorized");
    assert_eq!(ApiError::Forbidden.to_string(), "forbidden");
    assert_eq!(
        ApiError::NotFound("resource".into()).to_string(),
        "not found: resource"
    );
    assert_eq!(
        ApiError::Conflict("slug taken".into()).to_string(),
        "conflict: slug taken"
    );
    assert_eq!(
        ApiError::Internal("unexpected".into()).to_string(),
        "internal error: unexpected"
    );
}

// ── IdpError ──────────────────────────────────────────────────────────────────

#[test]
fn idp_error_display() {
    assert_eq!(
        IdpError::AuthorizationFailed("bad state".into()).to_string(),
        "authorization failed: bad state"
    );
    assert_eq!(
        IdpError::TokenExchangeFailed("401".into()).to_string(),
        "token exchange failed: 401"
    );
    assert_eq!(
        IdpError::ProviderUnavailable("google".into()).to_string(),
        "provider unavailable: google"
    );
    assert_eq!(
        IdpError::InvalidResponse("missing sub".into()).to_string(),
        "invalid response: missing sub"
    );
    assert_eq!(
        IdpError::Configuration("no client_id".into()).to_string(),
        "configuration error: no client_id"
    );
}
