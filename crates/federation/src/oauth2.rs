use async_trait::async_trait;
use irongate_core::{
    errors::IdpError,
    providers::{CallbackParams, IdentityProvider},
    types::FederatedIdentity,
};
use reqwest::{
    header::{HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT},
    Client,
};
use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone)]
pub struct OAuth2Config {
    pub id: String,
    pub name: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    /// When set, called to retrieve the primary verified email if the userinfo
    /// response returns a null email (GitHub hides email by default).
    pub emails_endpoint: Option<String>,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
}

impl OAuth2Config {
    pub fn github(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            id: "github".into(),
            name: "GitHub".into(),
            authorization_endpoint: "https://github.com/login/oauth/authorize".into(),
            token_endpoint: "https://github.com/login/oauth/access_token".into(),
            userinfo_endpoint: "https://api.github.com/user".into(),
            emails_endpoint: Some("https://api.github.com/user/emails".into()),
            client_id,
            client_secret,
            scopes: vec!["read:user".into(), "user:email".into()],
            redirect_uri,
        }
    }
}

pub struct OAuth2Provider {
    config: OAuth2Config,
    client: Client,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct UserinfoResponse {
    /// Numeric ID — converted to string for provider_user_id.
    id: serde_json::Value,
    login: Option<String>,
    name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct EmailEntry {
    email: String,
    verified: bool,
    primary: bool,
}

impl OAuth2Provider {
    pub fn new(config: OAuth2Config) -> Self {
        Self { config, client: Client::new() }
    }

    async fn fetch_access_token(&self, code: &str) -> Result<String, IdpError> {
        let response = self
            .client
            .post(&self.config.token_endpoint)
            // GitHub returns form-encoded by default; JSON required.
            .header(ACCEPT, HeaderValue::from_static("application/json"))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
                ("redirect_uri", self.config.redirect_uri.as_str()),
            ])
            .send()
            .await
            .map_err(|e| IdpError::ProviderUnavailable(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(IdpError::TokenExchangeFailed(format!("HTTP {status}: {body}")));
        }

        let token_resp: TokenResponse = response
            .json()
            .await
            .map_err(|e| IdpError::InvalidResponse(e.to_string()))?;

        Ok(token_resp.access_token)
    }

    async fn fetch_userinfo(&self, access_token: &str) -> Result<UserinfoResponse, IdpError> {
        let bearer = format!("Bearer {access_token}");
        self.client
            .get(&self.config.userinfo_endpoint)
            .header(AUTHORIZATION, HeaderValue::from_str(&bearer).map_err(|e| IdpError::ProviderUnavailable(e.to_string()))?)
            .header(USER_AGENT, HeaderValue::from_static("irongate"))
            .header(ACCEPT, HeaderValue::from_static("application/json"))
            .send()
            .await
            .map_err(|e| IdpError::ProviderUnavailable(e.to_string()))?
            .json()
            .await
            .map_err(|e| IdpError::InvalidResponse(e.to_string()))
    }

    /// Calls the emails endpoint to find the primary verified email.
    async fn fetch_primary_email(&self, access_token: &str, endpoint: &str) -> Result<String, IdpError> {
        let bearer = format!("Bearer {access_token}");
        let emails: Vec<EmailEntry> = self
            .client
            .get(endpoint)
            .header(AUTHORIZATION, HeaderValue::from_str(&bearer).map_err(|e| IdpError::ProviderUnavailable(e.to_string()))?)
            .header(USER_AGENT, HeaderValue::from_static("irongate"))
            .header(ACCEPT, HeaderValue::from_static("application/json"))
            .send()
            .await
            .map_err(|e| IdpError::ProviderUnavailable(e.to_string()))?
            .json()
            .await
            .map_err(|e| IdpError::InvalidResponse(e.to_string()))?;

        emails
            .into_iter()
            .find(|e| e.primary && e.verified)
            .map(|e| e.email)
            .ok_or_else(|| IdpError::InvalidResponse("no primary verified email found".into()))
    }
}

#[async_trait]
impl IdentityProvider for OAuth2Provider {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    async fn authorization_url(&self, state: &str, _nonce: Option<&str>) -> Result<Url, IdpError> {
        let mut url = Url::parse(&self.config.authorization_endpoint)
            .map_err(|e| IdpError::Configuration(e.to_string()))?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("response_type", "code");
            q.append_pair("client_id", &self.config.client_id);
            q.append_pair("redirect_uri", &self.config.redirect_uri);
            q.append_pair("scope", &self.config.scopes.join(" "));
            q.append_pair("state", state);
        }
        Ok(url)
    }

    async fn exchange_callback(
        &self,
        params: CallbackParams,
    ) -> Result<FederatedIdentity, IdpError> {
        let access_token = self.fetch_access_token(&params.code).await?;
        let userinfo = self.fetch_userinfo(&access_token).await?;

        let provider_user_id = userinfo.id.to_string();

        let email = match userinfo.email {
            Some(e) if !e.is_empty() => e,
            _ => match &self.config.emails_endpoint {
                Some(ep) => self.fetch_primary_email(&access_token, ep).await?,
                None => {
                    return Err(IdpError::InvalidResponse(
                        "userinfo returned no email and no emails endpoint is configured".into(),
                    ))
                }
            },
        };

        let raw_claims = serde_json::json!({
            "id": userinfo.id,
            "login": userinfo.login,
            "email": email,
            "avatar_url": userinfo.avatar_url,
        });

        Ok(FederatedIdentity {
            provider_user_id,
            email,
            email_verified: true, // GitHub only returns verified emails via /user/emails
            name: userinfo.name,
            picture: userinfo.avatar_url,
            raw_claims,
        })
    }
}
