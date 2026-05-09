use std::sync::Arc;

use irongate_core::{
    errors::{AuthError, StoreError},
    repositories::{UserCredentialsRepository, UserRepository},
    UserCredentials, UserStatus,
};
use irongate_crypto::hash::{hash_password, verify_password};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PasswordService {
    pub users: Arc<dyn UserRepository>,
    pub credentials: Arc<dyn UserCredentialsRepository>,
}

impl PasswordService {
    pub fn new(
        users: Arc<dyn UserRepository>,
        credentials: Arc<dyn UserCredentialsRepository>,
    ) -> Self {
        Self { users, credentials }
    }

    /// Verify a user's password and return their user record.
    /// Guards against suspended/pending accounts.
    pub async fn authenticate(
        &self,
        email: &str,
        password: &str,
        tenant_id: Uuid,
    ) -> Result<irongate_core::User, AuthError> {
        let user = self
            .users
            .get_by_email(email, tenant_id)
            .await
            .map_err(|e| match e {
                StoreError::NotFound(_) => AuthError::InvalidCredentials,
                other => AuthError::Store(other),
            })?;

        match user.status {
            UserStatus::Active => {}
            UserStatus::Suspended => return Err(AuthError::AccountSuspended),
            UserStatus::Pending => return Err(AuthError::AccountSuspended),
        }

        let creds = self
            .credentials
            .get_by_user_id(user.id, tenant_id)
            .await
            .map_err(|e| match e {
                StoreError::NotFound(_) => AuthError::InvalidCredentials,
                other => AuthError::Store(other),
            })?;

        let hash = creds.password_hash.as_deref().ok_or(AuthError::InvalidCredentials)?;

        let valid = verify_password(password, hash)
            .map_err(|_| AuthError::Internal("password verify failed".into()))?;

        if !valid {
            return Err(AuthError::InvalidCredentials);
        }

        Ok(user)
    }

    /// Set (or replace) the password for a user.
    pub async fn set_password(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        new_password: &str,
    ) -> Result<(), AuthError> {
        let new_hash = hash_password(new_password)
            .map_err(|_| AuthError::Internal("password hash failed".into()))?;

        match self.credentials.get_by_user_id(user_id, tenant_id).await {
            Ok(mut creds) => {
                creds.password_hash = Some(new_hash);
                creds.updated_at = OffsetDateTime::now_utc();
                self.credentials
                    .update(creds)
                    .await
                    .map_err(AuthError::Store)?;
            }
            Err(StoreError::NotFound(_)) => {
                let now = OffsetDateTime::now_utc();
                let creds = UserCredentials {
                    id: Uuid::new_v4(),
                    tenant_id,
                    user_id,
                    password_hash: Some(new_hash),
                    totp_secret: None,
                    totp_enabled: false,
                    created_at: now,
                    updated_at: now,
                };
                self.credentials.create(creds).await.map_err(AuthError::Store)?;
            }
            Err(e) => return Err(AuthError::Store(e)),
        }

        Ok(())
    }
}
