use std::sync::Arc;

use irongate_core::{
    errors::{AuthError, StoreError},
    repositories::{RefreshTokenRepository, SessionRepository},
    RefreshToken, Session,
};
use irongate_crypto::token::{generate_token, hash_token};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct SessionService {
    pub sessions: Arc<dyn SessionRepository>,
    pub refresh_tokens: Arc<dyn RefreshTokenRepository>,
}

impl SessionService {
    pub fn new(
        sessions: Arc<dyn SessionRepository>,
        refresh_tokens: Arc<dyn RefreshTokenRepository>,
    ) -> Self {
        Self {
            sessions,
            refresh_tokens,
        }
    }

    /// Create a new session and an initial refresh token.
    /// Returns (session, opaque_refresh_token).
    #[allow(clippy::too_many_arguments)]
    pub async fn create_session(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        application_id: Uuid,
        scope: String,
        idp_id: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
        session_ttl_secs: i64,
        refresh_token_ttl_secs: i64,
    ) -> Result<(Session, String), AuthError> {
        let now = OffsetDateTime::now_utc();

        let session = Session {
            id: Uuid::new_v4(),
            user_id,
            tenant_id,
            idp_id,
            ip_address,
            user_agent,
            created_at: now,
            expires_at: now + time::Duration::seconds(session_ttl_secs),
            revoked_at: None,
        };
        let session = self
            .sessions
            .create(session)
            .await
            .map_err(AuthError::Store)?;

        let raw_token = generate_token();
        let token_hash = hash_token(&raw_token);

        let refresh_token = RefreshToken {
            id: Uuid::new_v4(),
            session_id: session.id,
            application_id,
            token_hash,
            scope,
            previous_id: None,
            created_at: now,
            expires_at: now + time::Duration::seconds(refresh_token_ttl_secs),
            revoked_at: None,
        };
        self.refresh_tokens
            .create(refresh_token)
            .await
            .map_err(AuthError::Store)?;

        Ok((session, raw_token))
    }

    /// Validate a session by ID. Returns the session if valid.
    pub async fn validate_session(&self, session_id: Uuid) -> Result<Session, AuthError> {
        let session = self
            .sessions
            .get_by_id(session_id)
            .await
            .map_err(|e| match e {
                StoreError::NotFound(_) => AuthError::SessionNotFound,
                other => AuthError::Store(other),
            })?;

        if !session.is_valid() {
            return Err(AuthError::SessionExpired);
        }

        Ok(session)
    }

    /// Rotate a refresh token.
    ///
    /// If a revoked token is presented (reuse attack), revoke the entire session.
    /// Returns (session, new_opaque_token) on success.
    pub async fn rotate_refresh_token(
        &self,
        raw_token: &str,
        tenant_id: Uuid,
        refresh_token_ttl_secs: i64,
    ) -> Result<(Session, String), AuthError> {
        let token_hash = hash_token(raw_token);

        // Try to find an active token.
        let old_token = match self.refresh_tokens.get_by_hash(&token_hash).await {
            Ok(t) => t,
            Err(StoreError::NotFound(_)) => {
                // Token is either expired or was already revoked.
                // We can't detect reuse here since we don't know the session_id,
                // so surface a generic expired error.
                return Err(AuthError::TokenExpired);
            }
            Err(e) => return Err(AuthError::Store(e)),
        };

        // Check whether the token itself has expired.
        if old_token.expires_at < OffsetDateTime::now_utc() {
            return Err(AuthError::TokenExpired);
        }

        // Validate the associated session.
        let session = self
            .sessions
            .get_by_id(old_token.session_id)
            .await
            .map_err(|e| match e {
                StoreError::NotFound(_) => AuthError::SessionNotFound,
                other => AuthError::Store(other),
            })?;

        if session.tenant_id != tenant_id {
            return Err(AuthError::SessionNotFound);
        }

        if !session.is_valid() {
            return Err(AuthError::SessionExpired);
        }

        // Revoke the old token and issue a new one.
        self.refresh_tokens
            .revoke(old_token.id)
            .await
            .map_err(AuthError::Store)?;

        let now = OffsetDateTime::now_utc();
        let raw_new = generate_token();
        let new_hash = hash_token(&raw_new);

        let new_token = RefreshToken {
            id: Uuid::new_v4(),
            session_id: session.id,
            application_id: old_token.application_id,
            token_hash: new_hash,
            scope: old_token.scope.clone(),
            previous_id: Some(old_token.id),
            created_at: now,
            expires_at: now + time::Duration::seconds(refresh_token_ttl_secs),
            revoked_at: None,
        };
        self.refresh_tokens
            .create(new_token)
            .await
            .map_err(AuthError::Store)?;

        Ok((session, raw_new))
    }

    /// Revoke a session and all its refresh tokens.
    pub async fn revoke_session(&self, session_id: Uuid) -> Result<(), AuthError> {
        self.refresh_tokens
            .revoke_all_for_session(session_id)
            .await
            .map_err(AuthError::Store)?;
        self.sessions.revoke(session_id).await.map_err(|e| match e {
            StoreError::NotFound(_) => AuthError::SessionNotFound,
            other => AuthError::Store(other),
        })
    }

    /// Revoke all sessions for a user (e.g. on password change or account suspension).
    pub async fn revoke_all_user_sessions(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<u64, AuthError> {
        self.sessions
            .revoke_all_for_user(user_id, tenant_id)
            .await
            .map_err(AuthError::Store)
    }
}
