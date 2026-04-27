use async_trait::async_trait;
use url::Url;

use crate::errors::IdpError;
use crate::types::FederatedIdentity;

/// Parameters passed to `IdentityProvider::exchange_callback`.
#[derive(Debug, Clone)]
pub struct CallbackParams {
    pub code: String,
    pub state: String,
    pub nonce: Option<String>,
}

/// Every identity provider — local, OIDC, OAuth2, LDAP, and future SAML —
/// implements this trait. The rest of the system only talks to this interface.
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    /// Stable, unique identifier for this provider instance (e.g. "google", "github").
    fn id(&self) -> &str;

    /// Human-readable display name (e.g. "Google", "GitHub").
    fn name(&self) -> &str;

    /// Build the URL the browser should be redirected to in order to authenticate.
    async fn authorization_url(&self, state: &str, nonce: Option<&str>) -> Result<Url, IdpError>;

    /// Exchange the authorization code for a verified federated identity.
    async fn exchange_callback(
        &self,
        params: CallbackParams,
    ) -> Result<FederatedIdentity, IdpError>;
}
