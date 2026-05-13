use std::sync::Arc;

use irongate_core::{
    errors::{FederationError, StoreError},
    providers::IdentityProvider,
    repositories::{IdentityRepository, UserRepository},
    types::{Identity, User, UserStatus},
    FederatedIdentity,
};
use irongate_federation::{
    ldap::{LdapConfig, LdapProvider},
    oauth2::{OAuth2Config, OAuth2Provider},
    oidc::{OidcConfig, OidcProvider},
    FederationService,
};
use mockall::mock;
use time::OffsetDateTime;
use uuid::Uuid;

// ── Mocks ─────────────────────────────────────────────────────────────────────

mock! {
    UserRepo {}
    #[async_trait::async_trait]
    impl UserRepository for UserRepo {
        async fn create(&self, user: User) -> Result<User, StoreError>;
        async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<User, StoreError>;
        async fn get_by_email(&self, email: &str, tenant_id: Uuid) -> Result<User, StoreError>;
        async fn update(&self, user: User) -> Result<User, StoreError>;
        async fn soft_delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
        async fn list(&self, tenant_id: Uuid, limit: i64, offset: i64) -> Result<Vec<User>, StoreError>;
    }
}

mock! {
    IdentityRepo {}
    #[async_trait::async_trait]
    impl IdentityRepository for IdentityRepo {
        async fn create(&self, identity: Identity) -> Result<Identity, StoreError>;
        async fn get_by_provider(
            &self,
            provider: &str,
            provider_user_id: &str,
            tenant_id: Uuid,
        ) -> Result<Identity, StoreError>;
        async fn list_for_user(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Vec<Identity>, StoreError>;
        async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_service(users: MockUserRepo, identities: MockIdentityRepo) -> FederationService {
    FederationService::new(Arc::new(users), Arc::new(identities))
}

fn make_user(tenant_id: Uuid) -> User {
    User {
        id: Uuid::new_v4(),
        tenant_id,
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

fn make_identity(user_id: Uuid, tenant_id: Uuid) -> Identity {
    Identity {
        id: Uuid::new_v4(),
        user_id,
        tenant_id,
        provider: "google".into(),
        provider_user_id: "google-sub-123".into(),
        email: "alice@example.com".into(),
        raw_claims: serde_json::json!({"sub": "google-sub-123"}),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    }
}

fn make_federated(email_verified: bool) -> FederatedIdentity {
    FederatedIdentity {
        provider_user_id: "google-sub-123".into(),
        email: "alice@example.com".into(),
        email_verified,
        name: Some("Alice".into()),
        picture: None,
        raw_claims: serde_json::json!({"sub": "google-sub-123"}),
    }
}

fn make_oidc_provider() -> OidcProvider {
    OidcProvider::new(OidcConfig {
        id: "google".into(),
        name: "Google".into(),
        issuer: "https://accounts.google.com".into(),
        authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".into(),
        token_endpoint: "https://oauth2.googleapis.com/token".into(),
        jwks_uri: "https://www.googleapis.com/oauth2/v3/certs".into(),
        client_id: "my-client-id".into(),
        client_secret: "my-client-secret".into(),
        scopes: vec!["openid".into(), "email".into(), "profile".into()],
        redirect_uri: "https://app.example.com/callback".into(),
    })
}

// ── FederationService tests ───────────────────────────────────────────────────

#[tokio::test]
async fn provision_or_link_returns_existing_user_when_identity_already_linked() {
    let tenant_id = Uuid::new_v4();
    let user = make_user(tenant_id);
    let identity = make_identity(user.id, tenant_id);

    let user_clone = user.clone();
    let identity_clone = identity.clone();

    let mut users = MockUserRepo::new();
    users
        .expect_get_by_id()
        .once()
        .returning(move |_, _| Ok(user_clone.clone()));

    let mut identities = MockIdentityRepo::new();
    identities
        .expect_get_by_provider()
        .once()
        .returning(move |_, _, _| Ok(identity_clone.clone()));

    let svc = make_service(users, identities);
    let federated = make_federated(true);
    let (returned_user, returned_identity) =
        svc.provision_or_link(federated, "google", tenant_id).await.unwrap();

    assert_eq!(returned_user.id, user.id);
    assert_eq!(returned_identity.id, identity.id);
}

#[tokio::test]
async fn provision_or_link_jit_provisions_new_user_for_unknown_email() {
    let tenant_id = Uuid::new_v4();

    let mut users = MockUserRepo::new();
    users
        .expect_get_by_email()
        .once()
        .returning(|_, _| Err(StoreError::NotFound("user".into())));
    users
        .expect_create()
        .once()
        .returning(Ok);

    let mut identities = MockIdentityRepo::new();
    identities
        .expect_get_by_provider()
        .once()
        .returning(|_, _, _| Err(StoreError::NotFound("identity".into())));
    identities
        .expect_create()
        .once()
        .returning(Ok);

    let svc = make_service(users, identities);
    let federated = make_federated(true);
    let (user, identity) = svc.provision_or_link(federated, "google", tenant_id).await.unwrap();

    assert_eq!(user.email, "alice@example.com");
    assert!(user.email_verified);
    assert_eq!(identity.provider, "google");
}

#[tokio::test]
async fn provision_or_link_links_identity_to_existing_user_with_verified_email() {
    let tenant_id = Uuid::new_v4();
    let existing_user = make_user(tenant_id);
    let existing_clone = existing_user.clone();

    let mut users = MockUserRepo::new();
    users
        .expect_get_by_email()
        .once()
        .returning(move |_, _| Ok(existing_clone.clone()));

    let mut identities = MockIdentityRepo::new();
    identities
        .expect_get_by_provider()
        .once()
        .returning(|_, _, _| Err(StoreError::NotFound("identity".into())));
    identities
        .expect_create()
        .once()
        .returning(Ok);

    let svc = make_service(users, identities);
    let federated = make_federated(true);
    let (user, identity) =
        svc.provision_or_link(federated, "google", tenant_id).await.unwrap();

    assert_eq!(user.id, existing_user.id);
    assert_eq!(identity.user_id, existing_user.id);
}

#[tokio::test]
async fn provision_or_link_rejects_unverified_email_when_user_exists() {
    let tenant_id = Uuid::new_v4();
    let existing_user = make_user(tenant_id);

    let mut users = MockUserRepo::new();
    users
        .expect_get_by_email()
        .once()
        .returning(move |_, _| Ok(existing_user.clone()));

    let mut identities = MockIdentityRepo::new();
    identities
        .expect_get_by_provider()
        .once()
        .returning(|_, _, _| Err(StoreError::NotFound("identity".into())));

    let svc = make_service(users, identities);
    let federated = make_federated(false); // email not verified

    let result = svc.provision_or_link(federated, "google", tenant_id).await;
    assert!(
        matches!(result, Err(FederationError::LinkConflict(_))),
        "expected LinkConflict, got {result:?}"
    );
}

#[tokio::test]
async fn provision_or_link_propagates_identity_repo_error() {
    let tenant_id = Uuid::new_v4();

    let users = MockUserRepo::new();
    let mut identities = MockIdentityRepo::new();
    identities
        .expect_get_by_provider()
        .once()
        .returning(|_, _, _| Err(StoreError::Database("connection refused".into())));

    let svc = make_service(users, identities);
    let result = svc.provision_or_link(make_federated(true), "google", tenant_id).await;
    assert!(
        matches!(result, Err(FederationError::Store(_))),
        "expected Store error, got {result:?}"
    );
}

#[tokio::test]
async fn provision_or_link_propagates_user_create_error() {
    let tenant_id = Uuid::new_v4();

    let mut users = MockUserRepo::new();
    users
        .expect_get_by_email()
        .once()
        .returning(|_, _| Err(StoreError::NotFound("user".into())));
    users
        .expect_create()
        .once()
        .returning(|_| Err(StoreError::Database("disk full".into())));

    let mut identities = MockIdentityRepo::new();
    identities
        .expect_get_by_provider()
        .once()
        .returning(|_, _, _| Err(StoreError::NotFound("identity".into())));

    let svc = make_service(users, identities);
    let result = svc.provision_or_link(make_federated(true), "google", tenant_id).await;
    assert!(
        matches!(result, Err(FederationError::Store(_))),
        "expected Store error, got {result:?}"
    );
}

// ── OidcProvider tests ────────────────────────────────────────────────────────

#[test]
fn oidc_provider_builds_with_valid_config() {
    let svc = make_oidc_provider();
    assert_eq!(svc.id(), "google");
    assert_eq!(svc.name(), "Google");
}

#[tokio::test]
async fn oidc_authorization_url_includes_required_params() {
    let svc = make_oidc_provider();
    let url = svc.authorization_url("my-state-token", None).await.unwrap();
    let query: std::collections::HashMap<String, String> =
        url.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();

    assert_eq!(query.get("response_type").map(|s| s.as_str()), Some("code"));
    assert_eq!(query.get("client_id").map(|s| s.as_str()), Some("my-client-id"));
    assert_eq!(query.get("state").map(|s| s.as_str()), Some("my-state-token"));
    assert!(query.contains_key("scope"), "URL should include scope param");
    assert!(query.contains_key("redirect_uri"), "URL should include redirect_uri");
    assert!(!query.contains_key("nonce"), "nonce should not appear when not provided");
}

#[tokio::test]
async fn oidc_authorization_url_includes_nonce_when_provided() {
    let svc = make_oidc_provider();
    let url = svc.authorization_url("state-xyz", Some("my-nonce")).await.unwrap();
    let query: std::collections::HashMap<String, String> =
        url.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();

    assert_eq!(query.get("nonce").map(|s| s.as_str()), Some("my-nonce"));
}

// ── OAuth2Provider tests ──────────────────────────────────────────────────────

#[test]
fn oauth2_github_preset_sets_correct_endpoints() {
    let svc = OAuth2Provider::new(OAuth2Config::github(
        "gh-id".into(),
        "gh-secret".into(),
        "https://app.example.com/callback".into(),
    ));
    assert_eq!(svc.id(), "github");
    assert_eq!(svc.name(), "GitHub");
}

#[tokio::test]
async fn oauth2_authorization_url_includes_required_params() {
    let svc = OAuth2Provider::new(OAuth2Config::github(
        "gh-id".into(),
        "gh-secret".into(),
        "https://app.example.com/callback".into(),
    ));
    let url = svc.authorization_url("csrf-token", None).await.unwrap();
    let query: std::collections::HashMap<String, String> =
        url.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();

    assert_eq!(query.get("response_type").map(|s| s.as_str()), Some("code"));
    assert_eq!(query.get("client_id").map(|s| s.as_str()), Some("gh-id"));
    assert_eq!(query.get("state").map(|s| s.as_str()), Some("csrf-token"));
    assert!(query.contains_key("scope"));
    assert!(query.contains_key("redirect_uri"));
}

// ── LdapProvider tests ────────────────────────────────────────────────────────

#[tokio::test]
async fn ldap_authorization_url_returns_configuration_error() {
    use irongate_core::errors::IdpError;

    let svc = LdapProvider::new(LdapConfig {
        id: "ldap".into(),
        name: "LDAP".into(),
        url: "ldap://ldap.example.com:389".into(),
        bind_dn_template: "uid={username},ou=users,dc=example,dc=com".into(),
        base_dn: "dc=example,dc=com".into(),
        uid_attr: "uid".into(),
        mail_attr: "mail".into(),
        name_attr: "cn".into(),
    });

    let result = svc.authorization_url("state", None).await;
    assert!(
        matches!(result, Err(IdpError::Configuration(_))),
        "expected Configuration error, got {result:?}"
    );
}

#[tokio::test]
async fn ldap_exchange_callback_returns_configuration_error() {
    use irongate_core::{errors::IdpError, providers::CallbackParams};

    let svc = LdapProvider::new(LdapConfig {
        id: "ldap".into(),
        name: "LDAP".into(),
        url: "ldap://ldap.example.com:389".into(),
        bind_dn_template: "uid={username},ou=users,dc=example,dc=com".into(),
        base_dn: "dc=example,dc=com".into(),
        uid_attr: "uid".into(),
        mail_attr: "mail".into(),
        name_attr: "cn".into(),
    });

    let params = CallbackParams { code: "x".into(), state: "y".into(), nonce: None };
    let result = svc.exchange_callback(params).await;
    assert!(
        matches!(result, Err(IdpError::Configuration(_))),
        "expected Configuration error, got {result:?}"
    );
}
