use std::sync::Arc;

use base64ct::{Base64UrlUnpadded, Encoding};
use irongate_core::{
    errors::{StoreError, WebAuthnError},
    repositories::PasskeyRepository,
    PasskeyCredential, User,
};
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;
use webauthn_rs::prelude::{
    AuthenticationResult, CreationChallengeResponse, Passkey, PasskeyAuthentication,
    PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Webauthn, WebauthnBuilder,
};

pub struct WebAuthnService {
    webauthn: Arc<Webauthn>,
    passkeys: Arc<dyn PasskeyRepository>,
}

impl WebAuthnService {
    pub fn new(
        rp_id: &str,
        rp_origin: &Url,
        rp_name: &str,
        passkeys: Arc<dyn PasskeyRepository>,
    ) -> Result<Self, WebAuthnError> {
        let webauthn = WebauthnBuilder::new(rp_id, rp_origin)
            .map_err(|e| WebAuthnError::Internal(e.to_string()))?
            .rp_name(rp_name)
            .build()
            .map_err(|e| WebAuthnError::Internal(e.to_string()))?;
        Ok(Self {
            webauthn: Arc::new(webauthn),
            passkeys,
        })
    }

    /// Begin passkey registration for a user. Returns the challenge to send to the browser
    /// and an opaque state blob to store in the session.
    pub fn start_registration(
        &self,
        user: &User,
    ) -> Result<(CreationChallengeResponse, serde_json::Value), WebAuthnError> {
        let display_name = user.name.as_deref().unwrap_or(&user.email);
        let (challenge, reg_state) = self
            .webauthn
            .start_passkey_registration(user.id, &user.email, display_name, None)
            .map_err(|e| WebAuthnError::Registration(e.to_string()))?;
        let state_json =
            serde_json::to_value(&reg_state).map_err(|e| WebAuthnError::Internal(e.to_string()))?;
        Ok((challenge, state_json))
    }

    /// Complete passkey registration. `state_json` is the value returned by `start_registration`
    /// (retrieved from the session). Persists the new credential to the repository.
    pub async fn finish_registration(
        &self,
        user: &User,
        friendly_name: Option<String>,
        state_json: serde_json::Value,
        credential: &RegisterPublicKeyCredential,
    ) -> Result<PasskeyCredential, WebAuthnError> {
        let state: PasskeyRegistration =
            serde_json::from_value(state_json).map_err(|_| WebAuthnError::ChallengeMismatch)?;

        let passkey = self
            .webauthn
            .finish_passkey_registration(credential, &state)
            .map_err(|e| WebAuthnError::Registration(e.to_string()))?;

        let credential_id = Base64UrlUnpadded::encode_string(passkey.cred_id());
        let passkey_json =
            serde_json::to_value(&passkey).map_err(|e| WebAuthnError::Internal(e.to_string()))?;

        let now = OffsetDateTime::now_utc();
        let cred = PasskeyCredential {
            id: Uuid::new_v4(),
            tenant_id: user.tenant_id,
            user_id: user.id,
            credential_id,
            friendly_name,
            passkey_json,
            created_at: now,
            last_used_at: None,
        };

        self.passkeys.create(cred).await.map_err(|e| match e {
            StoreError::Conflict(_) => {
                WebAuthnError::Registration("credential already registered".into())
            }
            other => WebAuthnError::Store(other),
        })
    }

    /// Begin passkey authentication for a known user (username-first flow).
    /// Returns the challenge to send to the browser and an opaque state blob for the session.
    pub async fn start_authentication(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(RequestChallengeResponse, serde_json::Value), WebAuthnError> {
        let stored = self
            .passkeys
            .list_for_user(user_id, tenant_id)
            .await
            .map_err(WebAuthnError::Store)?;

        if stored.is_empty() {
            return Err(WebAuthnError::CredentialNotFound);
        }

        let passkeys: Vec<Passkey> = stored
            .iter()
            .map(|c| {
                serde_json::from_value(c.passkey_json.clone())
                    .map_err(|e| WebAuthnError::Internal(e.to_string()))
            })
            .collect::<Result<_, _>>()?;

        let (challenge, auth_state) = self
            .webauthn
            .start_passkey_authentication(&passkeys)
            .map_err(|e| WebAuthnError::Authentication(e.to_string()))?;

        let state_json = serde_json::to_value(&auth_state)
            .map_err(|e| WebAuthnError::Internal(e.to_string()))?;

        Ok((challenge, state_json))
    }

    /// Complete passkey authentication. Updates the credential counter and `last_used_at`.
    pub async fn finish_authentication(
        &self,
        tenant_id: Uuid,
        state_json: serde_json::Value,
        credential: &PublicKeyCredential,
    ) -> Result<AuthenticationResult, WebAuthnError> {
        let state: PasskeyAuthentication =
            serde_json::from_value(state_json).map_err(|_| WebAuthnError::ChallengeMismatch)?;

        let auth_result = self
            .webauthn
            .finish_passkey_authentication(credential, &state)
            .map_err(|e| WebAuthnError::Authentication(e.to_string()))?;

        let cred_id = Base64UrlUnpadded::encode_string(auth_result.cred_id());
        let mut stored = self
            .passkeys
            .get_by_credential_id(&cred_id, tenant_id)
            .await
            .map_err(|e| match e {
                StoreError::NotFound(_) => WebAuthnError::CredentialNotFound,
                other => WebAuthnError::Store(other),
            })?;

        let mut passkey: Passkey = serde_json::from_value(stored.passkey_json.clone())
            .map_err(|e| WebAuthnError::Internal(e.to_string()))?;

        passkey.update_credential(&auth_result);

        stored.passkey_json =
            serde_json::to_value(&passkey).map_err(|e| WebAuthnError::Internal(e.to_string()))?;
        stored.last_used_at = Some(OffsetDateTime::now_utc());

        self.passkeys
            .update(stored)
            .await
            .map_err(WebAuthnError::Store)?;

        Ok(auth_result)
    }
}
