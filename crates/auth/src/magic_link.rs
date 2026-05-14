use std::sync::Arc;

use irongate_core::{
    errors::{AuthError, StoreError},
    repositories::{MagicLinkRepository, UserRepository},
    MagicLink, User, UserStatus,
};
use irongate_crypto::token::{generate_token, hash_token};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct MagicLinkService {
    pub users: Arc<dyn UserRepository>,
    pub magic_links: Arc<dyn MagicLinkRepository>,
}

impl MagicLinkService {
    pub fn new(users: Arc<dyn UserRepository>, magic_links: Arc<dyn MagicLinkRepository>) -> Self {
        Self { users, magic_links }
    }

    /// Generate a magic link token for the given email.
    /// Returns (opaque_token, magic_link_record).
    /// The caller is responsible for sending the token to the user via email.
    pub async fn create_link(
        &self,
        email: &str,
        tenant_id: Uuid,
        ttl_secs: i64,
    ) -> Result<(String, MagicLink), AuthError> {
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
            UserStatus::Suspended | UserStatus::Pending => return Err(AuthError::AccountSuspended),
        }

        let now = OffsetDateTime::now_utc();
        let raw_token = generate_token();
        let token_hash = hash_token(&raw_token);

        let link = MagicLink {
            id: Uuid::new_v4(),
            tenant_id,
            user_id: user.id,
            token_hash,
            expires_at: now + time::Duration::seconds(ttl_secs),
            used_at: None,
            created_at: now,
        };
        let link = self
            .magic_links
            .create(link)
            .await
            .map_err(AuthError::Store)?;

        Ok((raw_token, link))
    }

    /// Consume a magic link token and return the authenticated user.
    pub async fn consume_link(&self, raw_token: &str, tenant_id: Uuid) -> Result<User, AuthError> {
        let token_hash = hash_token(raw_token);

        let link = self
            .magic_links
            .get_by_token_hash(&token_hash, tenant_id)
            .await
            .map_err(|e| match e {
                StoreError::NotFound(_) => AuthError::TokenAlreadyUsed,
                other => AuthError::Store(other),
            })?;

        if link.expires_at < OffsetDateTime::now_utc() {
            return Err(AuthError::TokenExpired);
        }

        self.magic_links
            .mark_used(link.id)
            .await
            .map_err(|e| match e {
                StoreError::NotFound(_) => AuthError::TokenAlreadyUsed,
                other => AuthError::Store(other),
            })?;

        let user = self
            .users
            .get_by_id(link.user_id, tenant_id)
            .await
            .map_err(AuthError::Store)?;

        Ok(user)
    }
}
