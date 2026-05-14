use async_trait::async_trait;
use irongate_core::{
    errors::IdpError,
    providers::{CallbackParams, IdentityProvider},
    types::FederatedIdentity,
};
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub id: String,
    pub name: String,
    /// Must match the `iss` claim in tokens — validated during verification.
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
}

pub struct OidcProvider {
    config: OidcConfig,
    client: Client,
}

#[derive(Deserialize)]
struct TokenResponse {
    #[allow(dead_code)]
    access_token: String,
    id_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IdTokenClaims {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    picture: Option<String>,
    nonce: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

impl OidcProvider {
    pub fn new(config: OidcConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Convenience async constructor: fetches the OIDC discovery document and
    /// fills in the endpoints automatically.
    pub async fn discover(
        issuer_url: &str,
        id: impl Into<String>,
        name: impl Into<String>,
        client_id: String,
        client_secret: String,
        redirect_uri: String,
        scopes: Vec<String>,
    ) -> Result<Self, IdpError> {
        #[derive(Deserialize)]
        struct Discovery {
            issuer: String,
            authorization_endpoint: String,
            token_endpoint: String,
            jwks_uri: String,
        }

        let discovery_url = format!(
            "{}/.well-known/openid-configuration",
            issuer_url.trim_end_matches('/')
        );
        let client = Client::new();
        let disc: Discovery = client
            .get(&discovery_url)
            .send()
            .await
            .map_err(|e| IdpError::ProviderUnavailable(e.to_string()))?
            .json()
            .await
            .map_err(|e| IdpError::InvalidResponse(e.to_string()))?;

        Ok(Self {
            config: OidcConfig {
                id: id.into(),
                name: name.into(),
                issuer: disc.issuer,
                authorization_endpoint: disc.authorization_endpoint,
                token_endpoint: disc.token_endpoint,
                jwks_uri: disc.jwks_uri,
                client_id,
                client_secret,
                redirect_uri,
                scopes,
            },
            client,
        })
    }

    async fn fetch_jwks(&self) -> Result<JwkSet, IdpError> {
        self.client
            .get(&self.config.jwks_uri)
            .send()
            .await
            .map_err(|e| IdpError::ProviderUnavailable(e.to_string()))?
            .json()
            .await
            .map_err(|e| IdpError::InvalidResponse(e.to_string()))
    }

    fn verify_id_token(
        &self,
        id_token: &str,
        jwks: &JwkSet,
        nonce: Option<&str>,
    ) -> Result<IdTokenClaims, IdpError> {
        let header =
            decode_header(id_token).map_err(|e| IdpError::InvalidResponse(e.to_string()))?;

        let jwk = if let Some(kid) = &header.kid {
            jwks.find(kid)
                .ok_or_else(|| IdpError::InvalidResponse(format!("no JWK matching kid={kid}")))?
        } else if jwks.keys.len() == 1 {
            &jwks.keys[0]
        } else {
            return Err(IdpError::InvalidResponse(
                "id_token header missing kid and JWKS has multiple keys".into(),
            ));
        };

        let key =
            DecodingKey::from_jwk(jwk).map_err(|e| IdpError::InvalidResponse(e.to_string()))?;

        let alg = match header.alg {
            Algorithm::RS256 => Algorithm::RS256,
            Algorithm::ES256 => Algorithm::ES256,
            other => {
                return Err(IdpError::InvalidResponse(format!(
                    "unsupported algorithm: {other:?}"
                )))
            }
        };

        let mut validation = Validation::new(alg);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.client_id]);

        let token_data = decode::<IdTokenClaims>(id_token, &key, &validation)
            .map_err(|e| IdpError::AuthorizationFailed(e.to_string()))?;

        let claims = token_data.claims;

        if let Some(expected) = nonce {
            match &claims.nonce {
                Some(actual) if actual == expected => {}
                _ => return Err(IdpError::AuthorizationFailed("nonce mismatch".into())),
            }
        }

        Ok(claims)
    }
}

#[async_trait]
impl IdentityProvider for OidcProvider {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    async fn authorization_url(&self, state: &str, nonce: Option<&str>) -> Result<Url, IdpError> {
        let mut url = Url::parse(&self.config.authorization_endpoint)
            .map_err(|e| IdpError::Configuration(e.to_string()))?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("response_type", "code");
            q.append_pair("client_id", &self.config.client_id);
            q.append_pair("redirect_uri", &self.config.redirect_uri);
            q.append_pair("scope", &self.config.scopes.join(" "));
            q.append_pair("state", state);
            if let Some(n) = nonce {
                q.append_pair("nonce", n);
            }
        }
        Ok(url)
    }

    async fn exchange_callback(
        &self,
        params: CallbackParams,
    ) -> Result<FederatedIdentity, IdpError> {
        let response = self
            .client
            .post(&self.config.token_endpoint)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", params.code.as_str()),
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
            return Err(IdpError::TokenExchangeFailed(format!(
                "HTTP {status}: {body}"
            )));
        }

        let token_resp: TokenResponse = response
            .json()
            .await
            .map_err(|e| IdpError::InvalidResponse(e.to_string()))?;

        let id_token = token_resp
            .id_token
            .ok_or_else(|| IdpError::InvalidResponse("token response missing id_token".into()))?;

        let jwks = self.fetch_jwks().await?;
        let claims = self.verify_id_token(&id_token, &jwks, params.nonce.as_deref())?;

        let raw_claims = serde_json::to_value(&claims).unwrap_or(serde_json::Value::Null);

        let email = claims
            .email
            .ok_or_else(|| IdpError::InvalidResponse("id_token missing email claim".into()))?;

        Ok(FederatedIdentity {
            provider_user_id: claims.sub,
            email,
            email_verified: claims.email_verified.unwrap_or(false),
            name: claims.name,
            picture: claims.picture,
            raw_claims,
        })
    }
}
