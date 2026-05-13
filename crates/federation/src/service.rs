use std::sync::Arc;

use irongate_core::{
    errors::{FederationError, StoreError},
    repositories::{IdentityRepository, UserRepository},
    types::{Identity, User, UserStatus},
    FederatedIdentity,
};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct FederationService {
    users: Arc<dyn UserRepository>,
    identities: Arc<dyn IdentityRepository>,
}

impl FederationService {
    pub fn new(users: Arc<dyn UserRepository>, identities: Arc<dyn IdentityRepository>) -> Self {
        Self { users, identities }
    }

    /// Given a verified federated identity, either return the already-linked user,
    /// link the identity to an existing user (requires `email_verified`), or
    /// JIT-provision a brand-new user.
    pub async fn provision_or_link(
        &self,
        federated: FederatedIdentity,
        provider_id: &str,
        tenant_id: Uuid,
    ) -> Result<(User, Identity), FederationError> {
        // Fast path: identity already linked — just return the user.
        match self
            .identities
            .get_by_provider(provider_id, &federated.provider_user_id, tenant_id)
            .await
        {
            Ok(identity) => {
                let user = self
                    .users
                    .get_by_id(identity.user_id, tenant_id)
                    .await
                    .map_err(FederationError::Store)?;
                return Ok((user, identity));
            }
            Err(StoreError::NotFound(_)) => {}
            Err(e) => return Err(FederationError::Store(e)),
        }

        // Identity not yet linked — resolve or create the user.
        let user = match self.users.get_by_email(&federated.email, tenant_id).await {
            Ok(existing) => {
                // Security gate: never auto-link to an existing account if the
                // provider has not verified the email — prevents account takeover
                // via an unverified claim.
                if !federated.email_verified {
                    return Err(FederationError::LinkConflict(federated.email.clone()));
                }
                existing
            }
            Err(StoreError::NotFound(_)) => {
                let now = OffsetDateTime::now_utc();
                let new_user = User {
                    id: Uuid::new_v4(),
                    tenant_id,
                    email: federated.email.clone(),
                    email_verified: federated.email_verified,
                    name: federated.name.clone(),
                    given_name: None,
                    family_name: None,
                    picture_url: federated.picture.clone(),
                    status: UserStatus::Active,
                    attributes: serde_json::json!({}),
                    created_at: now,
                    updated_at: now,
                    last_login_at: None,
                    deleted_at: None,
                };
                self.users
                    .create(new_user)
                    .await
                    .map_err(FederationError::Store)?
            }
            Err(e) => return Err(FederationError::Store(e)),
        };

        let now = OffsetDateTime::now_utc();
        let identity = Identity {
            id: Uuid::new_v4(),
            user_id: user.id,
            tenant_id,
            provider: provider_id.to_string(),
            provider_user_id: federated.provider_user_id.clone(),
            email: federated.email.clone(),
            raw_claims: federated.raw_claims.clone(),
            created_at: now,
            updated_at: now,
        };

        let identity = self
            .identities
            .create(identity)
            .await
            .map_err(FederationError::Store)?;

        Ok((user, identity))
    }
}
