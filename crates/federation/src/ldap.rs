use async_trait::async_trait;
use irongate_core::{
    errors::IdpError,
    providers::{CallbackParams, IdentityProvider},
    types::FederatedIdentity,
};
use ldap3::{LdapConnAsync, Scope, SearchEntry};
use url::Url;

#[derive(Debug, Clone)]
pub struct LdapConfig {
    pub id: String,
    pub name: String,
    /// LDAP server URL, e.g. `ldap://ldap.example.com:389` or `ldaps://...`
    pub url: String,
    /// Template for the bind DN; `{username}` is replaced at runtime.
    /// Example: `uid={username},ou=users,dc=example,dc=com`
    pub bind_dn_template: String,
    pub base_dn: String,
    /// LDAP attributes to fetch. Must include the uid, mail, and cn attributes.
    pub uid_attr: String,
    pub mail_attr: String,
    pub name_attr: String,
}

impl LdapConfig {
    pub fn default_attrs() -> (String, String, String) {
        ("uid".into(), "mail".into(), "cn".into())
    }
}

pub struct LdapProvider {
    config: LdapConfig,
}

impl LdapProvider {
    pub fn new(config: LdapConfig) -> Self {
        Self { config }
    }

    /// Direct credential authentication — used by the auth crate for LDAP login.
    /// The `IdentityProvider` trait methods return errors for LDAP because
    /// LDAP does not use the browser-redirect flow.
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<FederatedIdentity, IdpError> {
        let bind_dn = self.config.bind_dn_template.replace("{username}", username);

        let (conn, mut ldap) = LdapConnAsync::new(&self.config.url)
            .await
            .map_err(|e| IdpError::ProviderUnavailable(e.to_string()))?;
        tokio::spawn(async move { conn.drive().await });

        ldap.simple_bind(&bind_dn, password)
            .await
            .map_err(|e| IdpError::AuthorizationFailed(e.to_string()))?
            .success()
            .map_err(|_| IdpError::AuthorizationFailed("invalid credentials".into()))?;

        let filter = format!("({}={})", self.config.uid_attr, ldap_escape(username));
        let attrs = vec![
            self.config.uid_attr.as_str(),
            self.config.mail_attr.as_str(),
            self.config.name_attr.as_str(),
        ];

        let (rs, _) = ldap
            .search(&self.config.base_dn, Scope::Subtree, &filter, attrs)
            .await
            .map_err(|e| IdpError::ProviderUnavailable(e.to_string()))?
            .success()
            .map_err(|e| IdpError::ProviderUnavailable(e.to_string()))?;

        let entry = rs
            .into_iter()
            .next()
            .map(SearchEntry::construct)
            .ok_or_else(|| IdpError::InvalidResponse("user not found in LDAP directory".into()))?;

        let email = first_attr(&entry, &self.config.mail_attr)
            .ok_or_else(|| IdpError::InvalidResponse("LDAP entry missing mail attribute".into()))?;

        let name = first_attr(&entry, &self.config.name_attr);
        let provider_user_id =
            first_attr(&entry, &self.config.uid_attr).unwrap_or_else(|| username.to_string());

        ldap.unbind().await.ok();

        Ok(FederatedIdentity {
            provider_user_id,
            email,
            email_verified: true, // LDAP directory is the authoritative source
            name,
            picture: None,
            raw_claims: serde_json::json!({ "uid": username, "dn": bind_dn }),
        })
    }
}

fn first_attr(entry: &SearchEntry, attr: &str) -> Option<String> {
    entry.attrs.get(attr)?.first().cloned()
}

/// Escapes special characters in LDAP filter values per RFC 4515.
fn ldap_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' => out.push_str("\\5c"),
            '*' => out.push_str("\\2a"),
            '(' => out.push_str("\\28"),
            ')' => out.push_str("\\29"),
            '\0' => out.push_str("\\00"),
            c => out.push(c),
        }
    }
    out
}

#[async_trait]
impl IdentityProvider for LdapProvider {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    async fn authorization_url(&self, _state: &str, _nonce: Option<&str>) -> Result<Url, IdpError> {
        Err(IdpError::Configuration(
            "LDAP does not use browser-redirect flow; call LdapProvider::authenticate directly"
                .into(),
        ))
    }

    async fn exchange_callback(
        &self,
        _params: CallbackParams,
    ) -> Result<FederatedIdentity, IdpError> {
        Err(IdpError::Configuration(
            "LDAP does not use browser-redirect flow; call LdapProvider::authenticate directly"
                .into(),
        ))
    }
}
