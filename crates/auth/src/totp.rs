use std::sync::Arc;

use irongate_core::{
    errors::{AuthError, StoreError},
    repositories::UserCredentialsRepository,
};
use irongate_crypto::totp::{generate_totp_secret, verify_totp};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct TotpService {
    pub credentials: Arc<dyn UserCredentialsRepository>,
}

impl TotpService {
    pub fn new(credentials: Arc<dyn UserCredentialsRepository>) -> Self {
        Self { credentials }
    }

    /// Begin TOTP enrollment: generates a secret and returns (base32_secret, otpauth_uri).
    /// The secret is stored immediately (but totp_enabled stays false until verify_enrollment).
    pub async fn begin_enrollment(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        issuer: &str,
        account_name: &str,
    ) -> Result<(String, String), AuthError> {
        let (secret, uri) = generate_totp_secret(issuer, account_name)
            .map_err(|_| AuthError::Internal("TOTP secret generation failed".into()))?;

        let mut creds = self
            .credentials
            .get_by_user_id(user_id, tenant_id)
            .await
            .map_err(|e| match e {
                StoreError::NotFound(_) => AuthError::InvalidCredentials,
                other => AuthError::Store(other),
            })?;

        creds.totp_secret = Some(secret.clone());
        creds.updated_at = OffsetDateTime::now_utc();
        self.credentials.update(creds).await.map_err(AuthError::Store)?;

        Ok((secret, uri))
    }

    /// Confirm TOTP enrollment by verifying the user's first code.
    pub async fn confirm_enrollment(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        code: &str,
    ) -> Result<(), AuthError> {
        let mut creds = self
            .credentials
            .get_by_user_id(user_id, tenant_id)
            .await
            .map_err(AuthError::Store)?;

        let secret = creds.totp_secret.as_deref().ok_or(AuthError::InvalidMfaCode)?;

        let valid = verify_totp(secret, code)
            .map_err(|_| AuthError::Internal("TOTP verify failed".into()))?;

        if !valid {
            return Err(AuthError::InvalidMfaCode);
        }

        creds.totp_enabled = true;
        creds.updated_at = OffsetDateTime::now_utc();
        self.credentials.update(creds).await.map_err(AuthError::Store)?;

        Ok(())
    }

    /// Verify a TOTP code during login (called when totp_enabled == true).
    pub async fn verify_code(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        code: &str,
    ) -> Result<(), AuthError> {
        let creds = self
            .credentials
            .get_by_user_id(user_id, tenant_id)
            .await
            .map_err(AuthError::Store)?;

        if !creds.totp_enabled {
            return Ok(());
        }

        let secret = creds.totp_secret.as_deref().ok_or(AuthError::InvalidMfaCode)?;

        let valid = verify_totp(secret, code)
            .map_err(|_| AuthError::Internal("TOTP verify failed".into()))?;

        if !valid {
            return Err(AuthError::InvalidMfaCode);
        }

        Ok(())
    }

    /// Disable TOTP for a user.
    pub async fn disable(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), AuthError> {
        let mut creds = self
            .credentials
            .get_by_user_id(user_id, tenant_id)
            .await
            .map_err(AuthError::Store)?;

        creds.totp_enabled = false;
        creds.totp_secret = None;
        creds.updated_at = OffsetDateTime::now_utc();
        self.credentials.update(creds).await.map_err(AuthError::Store)?;

        Ok(())
    }
}
