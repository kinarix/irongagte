use std::sync::Arc;

use irongate_core::{
    errors::StoreError,
    repositories::PasskeyRepository,
    PasskeyCredential, User, UserStatus,
};
use irongate_webauthn::WebAuthnService;
use mockall::mock;
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

mock! {
    PasskeyRepo {}
    #[async_trait::async_trait]
    impl PasskeyRepository for PasskeyRepo {
        async fn create(&self, cred: PasskeyCredential) -> Result<PasskeyCredential, StoreError>;
        async fn get_by_credential_id(
            &self,
            credential_id: &str,
            tenant_id: Uuid,
        ) -> Result<PasskeyCredential, StoreError>;
        async fn list_for_user(
            &self,
            user_id: Uuid,
            tenant_id: Uuid,
        ) -> Result<Vec<PasskeyCredential>, StoreError>;
        async fn update(&self, cred: PasskeyCredential) -> Result<PasskeyCredential, StoreError>;
        async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    }
}

fn make_service(repo: MockPasskeyRepo) -> WebAuthnService {
    let origin = Url::parse("http://localhost:8080").unwrap();
    WebAuthnService::new("localhost", &origin, "Test Relying Party", Arc::new(repo)).unwrap()
}

fn make_user() -> User {
    User {
        id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        email: "alice@example.com".into(),
        email_verified: true,
        name: Some("Alice".into()),
        given_name: None,
        family_name: None,
        picture_url: None,
        status: UserStatus::Active,
        attributes: serde_json::json!({}),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        last_login_at: None,
        deleted_at: None,
    }
}

// ── Construction ──────────────────────────────────────────────────────────────

#[test]
fn service_builds_with_valid_params() {
    let repo = MockPasskeyRepo::new();
    let origin = Url::parse("http://localhost:8080").unwrap();
    let result = WebAuthnService::new("localhost", &origin, "Test", Arc::new(repo));
    assert!(result.is_ok());
}

#[test]
fn service_rejects_mismatched_rp_id_and_origin() {
    let repo = MockPasskeyRepo::new();
    let origin = Url::parse("http://example.com").unwrap();
    // rp_id "other.com" is not a suffix of "example.com"
    let result = WebAuthnService::new("other.com", &origin, "Test", Arc::new(repo));
    assert!(result.is_err());
}

// ── Registration ──────────────────────────────────────────────────────────────

#[test]
fn start_registration_returns_challenge_and_state() {
    let repo = MockPasskeyRepo::new();
    let svc = make_service(repo);
    let user = make_user();

    let result = svc.start_registration(&user);
    assert!(result.is_ok());
    let (_challenge, state) = result.unwrap();
    assert!(state.is_object(), "state should be a JSON object");
}

#[test]
fn start_registration_uses_email_as_display_name_when_no_name() {
    let repo = MockPasskeyRepo::new();
    let svc = make_service(repo);
    let mut user = make_user();
    user.name = None;

    // Should not panic — email is used as the display name fallback
    let result = svc.start_registration(&user);
    assert!(result.is_ok());
}

#[tokio::test]
async fn finish_registration_with_corrupt_state_returns_challenge_mismatch() {
    use irongate_core::errors::WebAuthnError;
    use webauthn_rs::prelude::RegisterPublicKeyCredential;

    let repo = MockPasskeyRepo::new();
    let svc = make_service(repo);
    let user = make_user();

    let bad_state = serde_json::json!({"this": "is not a PasskeyRegistration"});

    // Construct a structurally-plausible credential JSON (real ceremony not needed;
    // ChallengeMismatch is returned before the credential is inspected).
    let cred_json = serde_json::json!({
        "id": "dGVzdA",
        "rawId": "dGVzdA",
        "response": {
            "attestationObject": "dGVzdA",
            "clientDataJSON": "dGVzdA"
        },
        "type": "public-key"
    });
    let cred: RegisterPublicKeyCredential = serde_json::from_value(cred_json).unwrap();

    let result = svc.finish_registration(&user, None, bad_state, &cred).await;
    assert!(
        matches!(result, Err(WebAuthnError::ChallengeMismatch)),
        "expected ChallengeMismatch, got {result:?}"
    );
}

// ── Authentication ────────────────────────────────────────────────────────────

#[tokio::test]
async fn start_authentication_with_no_credentials_returns_not_found() {
    use irongate_core::errors::WebAuthnError;

    let mut repo = MockPasskeyRepo::new();
    repo.expect_list_for_user()
        .once()
        .returning(|_, _| Ok(vec![]));

    let svc = make_service(repo);
    let result = svc.start_authentication(Uuid::new_v4(), Uuid::new_v4()).await;
    assert!(
        matches!(result, Err(WebAuthnError::CredentialNotFound)),
        "expected CredentialNotFound, got {result:?}"
    );
}

#[tokio::test]
async fn start_authentication_propagates_repo_error() {
    use irongate_core::errors::WebAuthnError;

    let mut repo = MockPasskeyRepo::new();
    repo.expect_list_for_user()
        .once()
        .returning(|_, _| Err(StoreError::Database("connection lost".into())));

    let svc = make_service(repo);
    let result = svc.start_authentication(Uuid::new_v4(), Uuid::new_v4()).await;
    assert!(
        matches!(result, Err(WebAuthnError::Store(_))),
        "expected Store error, got {result:?}"
    );
}

#[tokio::test]
async fn finish_authentication_with_corrupt_state_returns_challenge_mismatch() {
    use irongate_core::errors::WebAuthnError;
    use webauthn_rs::prelude::PublicKeyCredential;

    let repo = MockPasskeyRepo::new();
    let svc = make_service(repo);

    let bad_state = serde_json::json!({"garbage": true});
    let cred_json = serde_json::json!({
        "id": "dGVzdA",
        "rawId": "dGVzdA",
        "response": {
            "authenticatorData": "dGVzdA",
            "clientDataJSON": "dGVzdA",
            "signature": "dGVzdA"
        },
        "type": "public-key"
    });
    let cred: PublicKeyCredential = serde_json::from_value(cred_json).unwrap();

    let result = svc.finish_authentication(Uuid::new_v4(), bad_state, &cred).await;
    assert!(
        matches!(result, Err(WebAuthnError::ChallengeMismatch)),
        "expected ChallengeMismatch, got {result:?}"
    );
}
