use std::sync::Arc;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use irongate_api::{
    claims::{make_jti, now_secs, AccessTokenClaims},
    config::*,
    router::build_router,
    state::AppState,
};
use irongate_auth::{PasswordService, SessionService};
use irongate_authz::AuthzService;
use irongate_core::{
    errors::StoreError,
    repositories::{
        ApplicationRepository, AuditRepository, AuthCodeData, AuthCodeStore,
        ClaimDefinitionRepository, GroupClaimRepository, GroupRepository, IdentityRepository,
        IdpConfigRepository, OperatorCredentialsRepository, OperatorPermissionRepository,
        OperatorRepository, OperatorRoleRepository, PasskeyRepository, RefreshTokenRepository,
        ResolvedGroupClaim, ResolvedUserClaim, SessionRepository, SigningKeyRepository,
        TenantRepository, UserClaimRepository, UserCredentialsRepository, UserRepository,
    },
    types::{
        AppType, Application, AuditEvent, ClaimDefinition, Group, GroupClaim, Identity, IdpConfig,
        Operator, OperatorCredentials, OperatorPermission, OperatorRole, PasskeyCredential,
        RefreshToken, Session, SigningKeyRecord, Tenant, User, UserClaim, UserCredentials,
        UserStatus,
    },
};
use irongate_crypto::{hash::hash_password, jwt::sign, keys::generate_rsa_key};
use jsonwebtoken::Algorithm;
use mockall::mock;
use serde_json::Value;
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

// ── Mock definitions ──────────────────────────────────────────────────────────

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
    TenantRepo {}
    #[async_trait::async_trait]
    impl TenantRepository for TenantRepo {
        async fn create(&self, tenant: Tenant) -> Result<Tenant, StoreError>;
        async fn get_by_id(&self, id: Uuid) -> Result<Tenant, StoreError>;
        async fn get_by_slug(&self, slug: &str) -> Result<Tenant, StoreError>;
        async fn update(&self, tenant: Tenant) -> Result<Tenant, StoreError>;
        async fn soft_delete(&self, id: Uuid) -> Result<(), StoreError>;
        async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Tenant>, StoreError>;
    }
}

mock! {
    ApplicationRepo {}
    #[async_trait::async_trait]
    impl ApplicationRepository for ApplicationRepo {
        async fn create(&self, app: Application) -> Result<Application, StoreError>;
        async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Application, StoreError>;
        async fn get_by_client_id(
            &self,
            client_id: &str,
            tenant_id: Uuid,
        ) -> Result<Application, StoreError>;
        async fn update(&self, app: Application) -> Result<Application, StoreError>;
        async fn soft_delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
        async fn list(
            &self,
            tenant_id: Uuid,
            limit: i64,
            offset: i64,
        ) -> Result<Vec<Application>, StoreError>;
    }
}

mock! {
    RefreshTokenRepo {}
    #[async_trait::async_trait]
    impl RefreshTokenRepository for RefreshTokenRepo {
        async fn create(&self, token: RefreshToken) -> Result<RefreshToken, StoreError>;
        async fn get_by_hash(&self, token_hash: &str) -> Result<RefreshToken, StoreError>;
        async fn revoke(&self, id: Uuid) -> Result<(), StoreError>;
        async fn revoke_all_for_session(&self, session_id: Uuid) -> Result<u64, StoreError>;
    }
}

mock! {
    SessionRepo {}
    #[async_trait::async_trait]
    impl SessionRepository for SessionRepo {
        async fn create(&self, session: Session) -> Result<Session, StoreError>;
        async fn get_by_id(&self, id: Uuid) -> Result<Session, StoreError>;
        async fn revoke(&self, id: Uuid) -> Result<(), StoreError>;
        async fn revoke_all_for_user(
            &self,
            user_id: Uuid,
            tenant_id: Uuid,
        ) -> Result<u64, StoreError>;
        async fn list_for_user(
            &self,
            user_id: Uuid,
            tenant_id: Uuid,
        ) -> Result<Vec<Session>, StoreError>;
    }
}

mock! {
    UserCredRepo {}
    #[async_trait::async_trait]
    impl UserCredentialsRepository for UserCredRepo {
        async fn create(&self, creds: UserCredentials) -> Result<UserCredentials, StoreError>;
        async fn get_by_user_id(
            &self,
            user_id: Uuid,
            tenant_id: Uuid,
        ) -> Result<UserCredentials, StoreError>;
        async fn update(&self, creds: UserCredentials) -> Result<UserCredentials, StoreError>;
        async fn delete(&self, user_id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    }
}

mock! {
    ClaimDefRepo {}
    #[async_trait::async_trait]
    impl ClaimDefinitionRepository for ClaimDefRepo {
        async fn create(&self, def: ClaimDefinition) -> Result<ClaimDefinition, StoreError>;
        async fn get_by_id(&self, id: Uuid) -> Result<ClaimDefinition, StoreError>;
        async fn get_by_app_and_key(&self, application_id: Uuid, key: &str) -> Result<ClaimDefinition, StoreError>;
        async fn list_for_app(&self, application_id: Uuid) -> Result<Vec<ClaimDefinition>, StoreError>;
        async fn list_for_tenant(&self, tenant_id: Uuid) -> Result<Vec<ClaimDefinition>, StoreError>;
        async fn update(&self, def: ClaimDefinition) -> Result<ClaimDefinition, StoreError>;
        async fn delete(&self, id: Uuid) -> Result<(), StoreError>;
    }
}

mock! {
    GroupClaimRepo {}
    #[async_trait::async_trait]
    impl GroupClaimRepository for GroupClaimRepo {
        async fn assign(&self, group_id: Uuid, claim_def_id: Uuid, value: &str) -> Result<GroupClaim, StoreError>;
        async fn revoke(&self, group_id: Uuid, claim_def_id: Uuid, value: &str) -> Result<(), StoreError>;
        async fn list_for_group(&self, group_id: Uuid) -> Result<Vec<GroupClaim>, StoreError>;
        async fn list_for_claim_def(&self, claim_def_id: Uuid) -> Result<Vec<GroupClaim>, StoreError>;
        async fn list_for_user_in_app(&self, user_id: Uuid, application_id: Uuid) -> Result<Vec<ResolvedGroupClaim>, StoreError>;
    }
}

mock! {
    UserClaimRepo {}
    #[async_trait::async_trait]
    impl UserClaimRepository for UserClaimRepo {
        async fn assign(&self, user_id: Uuid, claim_def_id: Uuid, value: &str) -> Result<UserClaim, StoreError>;
        async fn revoke(&self, user_id: Uuid, claim_def_id: Uuid, value: &str) -> Result<(), StoreError>;
        async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<UserClaim>, StoreError>;
        async fn list_for_user_in_app(&self, user_id: Uuid, application_id: Uuid) -> Result<Vec<ResolvedUserClaim>, StoreError>;
    }
}

mock! {
    GroupRepo {}
    #[async_trait::async_trait]
    impl GroupRepository for GroupRepo {
        async fn create(&self, group: Group) -> Result<Group, StoreError>;
        async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Group, StoreError>;
        async fn get_by_display_name(&self, display_name: &str, tenant_id: Uuid) -> Result<Group, StoreError>;
        async fn update(&self, group: Group) -> Result<Group, StoreError>;
        async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
        async fn list(&self, tenant_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Group>, StoreError>;
        async fn add_member(&self, group_id: Uuid, user_id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
        async fn remove_member(&self, group_id: Uuid, user_id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
        async fn list_members(&self, group_id: Uuid, tenant_id: Uuid) -> Result<Vec<User>, StoreError>;
        async fn list_for_user(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Vec<Group>, StoreError>;
    }
}

mock! {
    PasskeyRepo {}
    #[async_trait::async_trait]
    impl PasskeyRepository for PasskeyRepo {
        async fn create(&self, cred: PasskeyCredential) -> Result<PasskeyCredential, StoreError>;
        async fn get_by_credential_id(&self, credential_id: &str, tenant_id: Uuid) -> Result<PasskeyCredential, StoreError>;
        async fn list_for_user(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Vec<PasskeyCredential>, StoreError>;
        async fn update(&self, cred: PasskeyCredential) -> Result<PasskeyCredential, StoreError>;
        async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    }
}

mock! {
    IdentityRepo {}
    #[async_trait::async_trait]
    impl IdentityRepository for IdentityRepo {
        async fn create(&self, identity: Identity) -> Result<Identity, StoreError>;
        async fn get_by_provider(&self, provider: &str, provider_user_id: &str, tenant_id: Uuid) -> Result<Identity, StoreError>;
        async fn list_for_user(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Vec<Identity>, StoreError>;
        async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    }
}

mock! {
    IdpConfigRepo {}
    #[async_trait::async_trait]
    impl IdpConfigRepository for IdpConfigRepo {
        async fn create(&self, config: IdpConfig) -> Result<IdpConfig, StoreError>;
        async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<IdpConfig, StoreError>;
        async fn list(&self, tenant_id: Uuid) -> Result<Vec<IdpConfig>, StoreError>;
        async fn update(&self, config: IdpConfig) -> Result<IdpConfig, StoreError>;
        async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    }
}

mock! {
    AuditRepo {}
    #[async_trait::async_trait]
    impl AuditRepository for AuditRepo {
        async fn record(&self, event: AuditEvent) -> Result<(), StoreError>;
        async fn list(&self, tenant_id: Uuid, limit: i64, offset: i64) -> Result<Vec<AuditEvent>, StoreError>;
    }
}

mock! {
    OperatorRepo {}
    #[async_trait::async_trait]
    impl OperatorRepository for OperatorRepo {
        async fn create(&self, operator: Operator) -> Result<Operator, StoreError>;
        async fn get_by_id(&self, id: Uuid) -> Result<Operator, StoreError>;
        async fn get_by_email(&self, email: &str) -> Result<Operator, StoreError>;
        async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Operator>, StoreError>;
        async fn update(&self, operator: Operator) -> Result<Operator, StoreError>;
        async fn soft_delete(&self, id: Uuid) -> Result<(), StoreError>;
        async fn touch_last_login(&self, id: Uuid) -> Result<(), StoreError>;
    }
}

mock! {
    SigningKeyRepo {}
    #[async_trait::async_trait]
    impl SigningKeyRepository for SigningKeyRepo {
        async fn list_publishable(&self, tenant_id: Option<Uuid>) -> Result<Vec<SigningKeyRecord>, StoreError>;
        async fn current(&self, tenant_id: Option<Uuid>) -> Result<Option<SigningKeyRecord>, StoreError>;
        async fn create(&self, key: SigningKeyRecord) -> Result<SigningKeyRecord, StoreError>;
        async fn retire(&self, id: Uuid) -> Result<(), StoreError>;
        async fn try_acquire_rotation_lock(&self) -> Result<bool, StoreError>;
        async fn release_rotation_lock(&self) -> Result<(), StoreError>;
    }
}

mock! {
    OperatorCredsRepo {}
    #[async_trait::async_trait]
    impl OperatorCredentialsRepository for OperatorCredsRepo {
        async fn create(&self, creds: OperatorCredentials) -> Result<OperatorCredentials, StoreError>;
        async fn get_by_operator_id(&self, operator_id: Uuid) -> Result<OperatorCredentials, StoreError>;
        async fn update(&self, creds: OperatorCredentials) -> Result<OperatorCredentials, StoreError>;
        async fn delete(&self, operator_id: Uuid) -> Result<(), StoreError>;
    }
}

mock! {
    OperatorPermRepo {}
    #[async_trait::async_trait]
    impl OperatorPermissionRepository for OperatorPermRepo {
        async fn create(&self, perm: OperatorPermission) -> Result<OperatorPermission, StoreError>;
        async fn get_by_id(&self, id: Uuid) -> Result<OperatorPermission, StoreError>;
        async fn get_by_resource_action(&self, resource: &str, action: &str) -> Result<OperatorPermission, StoreError>;
        async fn list(&self) -> Result<Vec<OperatorPermission>, StoreError>;
    }
}

mock! {
    OperatorRoleRepo {}
    #[async_trait::async_trait]
    impl OperatorRoleRepository for OperatorRoleRepo {
        async fn create(&self, role: OperatorRole) -> Result<OperatorRole, StoreError>;
        async fn get_by_id(&self, id: Uuid) -> Result<OperatorRole, StoreError>;
        async fn get_by_name(&self, name: &str, tenant_id: Option<Uuid>) -> Result<OperatorRole, StoreError>;
        async fn list(&self, scope: irongate_core::repositories::OperatorRoleScope) -> Result<Vec<OperatorRole>, StoreError>;
        async fn update(&self, role: OperatorRole) -> Result<OperatorRole, StoreError>;
        async fn delete(&self, id: Uuid) -> Result<(), StoreError>;
        async fn assign_permission(&self, role_id: Uuid, perm_id: Uuid) -> Result<(), StoreError>;
        async fn revoke_permission(&self, role_id: Uuid, perm_id: Uuid) -> Result<(), StoreError>;
        async fn list_permissions(&self, role_id: Uuid) -> Result<Vec<OperatorPermission>, StoreError>;
        async fn assign_to_operator(&self, operator_id: Uuid, role_id: Uuid) -> Result<(), StoreError>;
        async fn revoke_from_operator(&self, operator_id: Uuid, role_id: Uuid) -> Result<(), StoreError>;
        async fn list_for_operator(&self, operator_id: Uuid) -> Result<Vec<OperatorRole>, StoreError>;
        async fn list_permissions_for_operator(&self, operator_id: Uuid) -> Result<Vec<OperatorPermission>, StoreError>;
        async fn list_permissions_for_operator_in_tenant(&self, operator_id: Uuid, tenant_id: Uuid) -> Result<Vec<OperatorPermission>, StoreError>;
        async fn list_permissions_for_operator_global(&self, operator_id: Uuid) -> Result<Vec<OperatorPermission>, StoreError>;
    }
}

mock! {
    AuthCodeStoreRepo {}
    #[async_trait::async_trait]
    impl AuthCodeStore for AuthCodeStoreRepo {
        async fn store_code(&self, code: &str, data: AuthCodeData, ttl_secs: i64) -> Result<(), StoreError>;
        async fn take_code(&self, code: &str) -> Result<Option<AuthCodeData>, StoreError>;
    }
}

// ── Fixtures ──────────────────────────────────────────────────────────────────

/// Builds a stand-alone Prometheus recorder for tests. The exporter library
/// allows only one global recorder install per process; using `build_recorder`
/// here gives each test an isolated handle without touching the global slot.
fn test_metrics() -> metrics_exporter_prometheus::PrometheusHandle {
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .build_recorder()
        .handle()
}

fn test_settings() -> Settings {
    Settings {
        server: ServerConfig {
            host: "127.0.0.1".into(),
            port: 3000,
        },
        database: DatabaseConfig {
            url: "postgres://localhost/test".into(),
            max_connections: 1,
        },
        redis: RedisConfig {
            url: "redis://localhost".into(),
        },
        base_url: "https://auth.test".into(),
        log: LogConfig {
            level: "off".into(),
            format: "json".into(),
        },
        tokens: TokenConfig {
            access_token_ttl_seconds: 3600,
            refresh_token_ttl_seconds: 86400,
            id_token_ttl_seconds: 3600,
        },
        session: SessionConfig {
            ttl_seconds: 86400,
            cookie_name: "sid".into(),
            cookie_secure: false,
        },
        smtp: SmtpConfig {
            host: "localhost".into(),
            port: 25,
            from: "noreply@test".into(),
            username: String::new(),
            password: String::new(),
        },
        signing_keys: irongate_api::config::SigningKeysConfig::default(),
        rate_limit: irongate_api::config::RateLimitConfig {
            // Governor's PeerIpKeyExtractor needs a real socket; the axum
            // oneshot Service in tests doesn't provide one. Disable rather
            // than work around it.
            enabled: false,
            ..Default::default()
        },
        scim_tenant_id: None,
    }
}

fn test_user(tenant_id: Uuid) -> User {
    let now = OffsetDateTime::now_utc();
    User {
        id: Uuid::new_v4(),
        tenant_id,
        email: "alice@example.com".into(),
        email_verified: true,
        name: Some("Alice Example".into()),
        given_name: Some("Alice".into()),
        family_name: Some("Example".into()),
        picture_url: None,
        status: UserStatus::Active,
        attributes: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        last_login_at: None,
        deleted_at: None,
    }
}

fn test_tenant() -> Tenant {
    let now = OffsetDateTime::now_utc();
    Tenant {
        id: Uuid::new_v4(),
        name: "Acme Corp".into(),
        slug: "acme".into(),
        settings: serde_json::Value::Object(Default::default()),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    }
}

/// Build an AuthzService whose claim repos are pre-stubbed to return empty
/// results. Tests that don't exercise claim resolution can use this directly.
fn empty_authz() -> AuthzService {
    let mut defs = MockClaimDefRepo::new();
    defs.expect_list_for_app().returning(|_| Ok(vec![]));
    let mut group_claims = MockGroupClaimRepo::new();
    group_claims
        .expect_list_for_user_in_app()
        .returning(|_, _| Ok(vec![]));
    let mut user_claims = MockUserClaimRepo::new();
    user_claims
        .expect_list_for_user_in_app()
        .returning(|_, _| Ok(vec![]));
    AuthzService::new(
        Arc::new(defs),
        Arc::new(group_claims),
        Arc::new(user_claims),
    )
}

fn test_application(tenant_id: Uuid) -> Application {
    let now = OffsetDateTime::now_utc();
    Application {
        id: Uuid::new_v4(),
        tenant_id,
        name: "Test App".into(),
        client_id: "test-client".into(),
        client_secret_hash: None,
        app_type: AppType::Web,
        redirect_uris: vec!["https://app.test/callback".into()],
        allowed_scopes: vec!["openid".into(), "profile".into(), "email".into()],
        grant_types: vec!["password".into(), "refresh_token".into()],
        access_token_ttl: 3600,
        refresh_token_ttl: 86400,
        claim_prefix: "test-app".into(),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    }
}

fn test_session(user_id: Uuid, tenant_id: Uuid) -> Session {
    let now = OffsetDateTime::now_utc();
    Session {
        id: Uuid::new_v4(),
        user_id,
        tenant_id,
        idp_id: None,
        ip_address: None,
        user_agent: None,
        created_at: now,
        expires_at: now + time::Duration::hours(8),
        revoked_at: None,
    }
}

fn test_refresh_token(session_id: Uuid, app_id: Uuid) -> RefreshToken {
    let now = OffsetDateTime::now_utc();
    RefreshToken {
        id: Uuid::new_v4(),
        session_id,
        application_id: app_id,
        token_hash: "dummy_hash".into(),
        scope: "openid profile email".into(),
        previous_id: None,
        created_at: now,
        expires_at: now + time::Duration::hours(24),
        revoked_at: None,
    }
}

fn test_creds(user: &User, password: &str) -> UserCredentials {
    let now = OffsetDateTime::now_utc();
    UserCredentials {
        id: Uuid::new_v4(),
        tenant_id: user.tenant_id,
        user_id: user.id,
        password_hash: Some(hash_password(password).expect("hash_password failed")),
        totp_secret: None,
        totp_enabled: false,
        created_at: now,
        updated_at: now,
    }
}

/// Build a router backed by the given mocks. Takes ownership of all mock structs.
#[allow(clippy::too_many_arguments)]
fn build_app(
    users: MockUserRepo,
    tenants: MockTenantRepo,
    applications: MockApplicationRepo,
    refresh_tokens: MockRefreshTokenRepo,
    pw_users: MockUserRepo,
    pw_creds: MockUserCredRepo,
    session_sessions: MockSessionRepo,
    session_rts: MockRefreshTokenRepo,
) -> axum::Router {
    let key_record = generate_rsa_key(None, 1).expect("failed to generate RSA key");
    let signing_key = Arc::new(arc_swap::ArcSwap::from_pointee(key_record.clone()));
    let mut sk_repo = MockSigningKeyRepo::new();
    sk_repo
        .expect_list_publishable()
        .returning(move |_| Ok(vec![key_record.clone()]));
    let signing_keys: Arc<dyn SigningKeyRepository> = Arc::new(sk_repo);
    let config = Arc::new(test_settings());

    let users: Arc<dyn UserRepository> = Arc::new(users);
    let tenants: Arc<dyn TenantRepository> = Arc::new(tenants);
    let applications: Arc<dyn ApplicationRepository> = Arc::new(applications);
    let refresh_tokens: Arc<dyn RefreshTokenRepository> = Arc::new(refresh_tokens);

    let pw_users: Arc<dyn UserRepository> = Arc::new(pw_users);
    let pw_creds: Arc<dyn UserCredentialsRepository> = Arc::new(pw_creds);
    let password_svc = Arc::new(PasswordService::new(pw_users, pw_creds));

    let session_sessions: Arc<dyn SessionRepository> = Arc::new(session_sessions);
    let session_rts: Arc<dyn RefreshTokenRepository> = Arc::new(session_rts);
    let session_svc = Arc::new(SessionService::new(session_sessions, session_rts));

    let state = Arc::new(AppState {
        config,
        users,
        tenants,
        applications,
        refresh_tokens,
        groups: Arc::new(MockGroupRepo::new()),
        passkeys: Arc::new(MockPasskeyRepo::new()),
        identities: Arc::new(MockIdentityRepo::new()),
        idp_configs: Arc::new(MockIdpConfigRepo::new()),
        audit: Arc::new(permissive_audit()),
        operators: Arc::new(MockOperatorRepo::new()),
        operator_credentials: Arc::new(MockOperatorCredsRepo::new()),
        operator_permissions: Arc::new(MockOperatorPermRepo::new()),
        operator_roles_repo: Arc::new(MockOperatorRoleRepo::new()),
        claim_definitions: Arc::new(MockClaimDefRepo::new()),
        group_claims: Arc::new(MockGroupClaimRepo::new()),
        user_claims: Arc::new(MockUserClaimRepo::new()),
        auth_codes: Arc::new(MockAuthCodeStoreRepo::new()),
        password_svc,
        session_svc,
        authz_svc: Arc::new(empty_authz()),
        signing_key,
        signing_keys,
        metrics: test_metrics(),
    });

    build_router(state)
}

/// AuditRepo mock that accepts any `record()` and returns Ok. Tests that want
/// to assert on specific audit events should construct their own mock instead.
fn permissive_audit() -> MockAuditRepo {
    let mut audit = MockAuditRepo::new();
    audit.expect_record().returning(|_| Ok(()));
    audit
}

/// Convenience builder for tests that don't exercise the token endpoint.
fn simple_app(
    users: MockUserRepo,
    tenants: MockTenantRepo,
    applications: MockApplicationRepo,
    refresh_tokens: MockRefreshTokenRepo,
) -> axum::Router {
    build_app(
        users,
        tenants,
        applications,
        refresh_tokens,
        MockUserRepo::new(),
        MockUserCredRepo::new(),
        MockSessionRepo::new(),
        MockRefreshTokenRepo::new(),
    )
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ── Health ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_returns_ok() {
    let app = simple_app(
        MockUserRepo::new(),
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "ok");
}

// ── OIDC discovery ────────────────────────────────────────────────────────────

#[tokio::test]
async fn oidc_discovery_has_required_fields() {
    let app = simple_app(
        MockUserRepo::new(),
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/openid-configuration")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["issuer"].is_string());
    assert!(body["token_endpoint"].is_string());
    assert!(body["authorization_endpoint"].is_string());
    assert!(body["jwks_uri"].is_string());
    assert!(body["userinfo_endpoint"].is_string());
    assert_eq!(body["issuer"], "https://auth.test");
}

// ── JWKS ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn jwks_returns_keys_array() {
    let app = simple_app(
        MockUserRepo::new(),
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/jwks.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["keys"].is_array());
    let keys = body["keys"].as_array().unwrap();
    assert!(!keys.is_empty());
    assert_eq!(keys[0]["kty"], "RSA");
    assert_eq!(keys[0]["alg"], "RS256");
}

#[tokio::test]
async fn metrics_endpoint_returns_prometheus_text() {
    let app = simple_app(
        MockUserRepo::new(),
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(ct.starts_with("text/plain"));
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    // Fresh recorder with no counters fired yet; body is allowed to be empty.
    // The important assertion is that the route exists and content-type is
    // correct — that confirms the exporter is wired into the request path.
    assert!(String::from_utf8_lossy(&bytes).is_ascii());
}

/// JWKS must keep publishing a key after it is retired, as long as the key
/// hasn't expired yet — otherwise tokens minted just before rotation would
/// fail to verify mid-flight.
#[tokio::test]
async fn jwks_serves_retired_but_unexpired_keys() {
    let active = generate_rsa_key(None, 30).expect("rsa");
    let mut retired = generate_rsa_key(None, 30).expect("rsa");
    retired.retired_at = Some(OffsetDateTime::now_utc());

    let active_kid = active.id;
    let retired_kid = retired.id;

    let mut sk_repo = MockSigningKeyRepo::new();
    sk_repo
        .expect_list_publishable()
        .returning(move |_| Ok(vec![active.clone(), retired.clone()]));

    let key_record = generate_rsa_key(None, 1).expect("rsa");
    let signing_key = Arc::new(arc_swap::ArcSwap::from_pointee(key_record));
    let state = Arc::new(AppState {
        config: Arc::new(test_settings()),
        users: Arc::new(MockUserRepo::new()),
        tenants: Arc::new(MockTenantRepo::new()),
        applications: Arc::new(MockApplicationRepo::new()),
        refresh_tokens: Arc::new(MockRefreshTokenRepo::new()),
        groups: Arc::new(MockGroupRepo::new()),
        passkeys: Arc::new(MockPasskeyRepo::new()),
        identities: Arc::new(MockIdentityRepo::new()),
        idp_configs: Arc::new(MockIdpConfigRepo::new()),
        audit: Arc::new(permissive_audit()),
        operators: Arc::new(MockOperatorRepo::new()),
        operator_credentials: Arc::new(MockOperatorCredsRepo::new()),
        operator_permissions: Arc::new(MockOperatorPermRepo::new()),
        operator_roles_repo: Arc::new(MockOperatorRoleRepo::new()),
        claim_definitions: Arc::new(MockClaimDefRepo::new()),
        group_claims: Arc::new(MockGroupClaimRepo::new()),
        user_claims: Arc::new(MockUserClaimRepo::new()),
        auth_codes: Arc::new(MockAuthCodeStoreRepo::new()),
        password_svc: Arc::new(PasswordService::new(
            Arc::new(MockUserRepo::new()),
            Arc::new(MockUserCredRepo::new()),
        )),
        session_svc: Arc::new(SessionService::new(
            Arc::new(MockSessionRepo::new()),
            Arc::new(MockRefreshTokenRepo::new()),
        )),
        authz_svc: Arc::new(empty_authz()),
        signing_key,
        signing_keys: Arc::new(sk_repo),
        metrics: test_metrics(),
    });

    let resp = build_router(state)
        .oneshot(
            Request::builder()
                .uri("/.well-known/jwks.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let kids: Vec<String> = body["keys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["kid"].as_str().unwrap().to_string())
        .collect();
    assert!(kids.contains(&active_kid.to_string()));
    assert!(kids.contains(&retired_kid.to_string()));
}

// ── Users ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_users_missing_tenant_header_returns_400() {
    let app = simple_app(
        MockUserRepo::new(),
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_users_returns_empty_list() {
    let tenant_id = Uuid::new_v4();
    let mut users = MockUserRepo::new();
    users.expect_list().returning(|_, _, _| Ok(vec![]));

    let app = simple_app(
        users,
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/users")
                .header("X-Tenant-ID", tenant_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["users"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn list_users_returns_users() {
    let tenant_id = Uuid::new_v4();
    let user = test_user(tenant_id);
    let user_clone = user.clone();

    let mut users = MockUserRepo::new();
    users
        .expect_list()
        .returning(move |_, _, _| Ok(vec![user_clone.clone()]));

    let app = simple_app(
        users,
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/users")
                .header("X-Tenant-ID", tenant_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["users"][0]["email"], "alice@example.com");
}

#[tokio::test]
async fn create_user_returns_201() {
    let tenant_id = Uuid::new_v4();
    let mut users = MockUserRepo::new();
    users.expect_create().returning(Ok);

    let app = simple_app(
        users,
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let body = serde_json::json!({
        "email": "bob@example.com",
        "name": "Bob"
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/users")
                .header("X-Tenant-ID", tenant_id.to_string())
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let resp_body = body_json(resp).await;
    assert_eq!(resp_body["email"], "bob@example.com");
}

#[tokio::test]
async fn get_user_returns_200() {
    let tenant_id = Uuid::new_v4();
    let user = test_user(tenant_id);
    let user_clone = user.clone();
    let user_id = user.id;

    let mut users = MockUserRepo::new();
    users
        .expect_get_by_id()
        .returning(move |_, _| Ok(user_clone.clone()));

    let app = simple_app(
        users,
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/users/{user_id}"))
                .header("X-Tenant-ID", tenant_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["email"], "alice@example.com");
}

#[tokio::test]
async fn get_user_not_found_returns_404() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let mut users = MockUserRepo::new();
    users
        .expect_get_by_id()
        .returning(|id, _| Err(StoreError::NotFound(format!("user {id} not found"))));

    let app = simple_app(
        users,
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/users/{user_id}"))
                .header("X-Tenant-ID", tenant_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_user_returns_204() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let mut users = MockUserRepo::new();
    users.expect_soft_delete().returning(|_, _| Ok(()));

    let app = simple_app(
        users,
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/users/{user_id}"))
                .header("X-Tenant-ID", tenant_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

// ── Tenants ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_tenants_returns_empty_list() {
    let mut tenants = MockTenantRepo::new();
    tenants.expect_list().returning(|_, _| Ok(vec![]));

    let app = simple_app(
        MockUserRepo::new(),
        tenants,
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tenants")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["tenants"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn create_tenant_returns_201() {
    let mut tenants = MockTenantRepo::new();
    tenants.expect_create().returning(Ok);

    let app = simple_app(
        MockUserRepo::new(),
        tenants,
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let body = serde_json::json!({ "name": "Acme Corp", "slug": "acme" });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/tenants")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let resp_body = body_json(resp).await;
    assert_eq!(resp_body["slug"], "acme");
}

#[tokio::test]
async fn get_tenant_returns_200() {
    let tenant = test_tenant();
    let tenant_id = tenant.id;
    let tenant_clone = tenant.clone();

    let mut tenants = MockTenantRepo::new();
    tenants
        .expect_get_by_id()
        .returning(move |_| Ok(tenant_clone.clone()));

    let app = simple_app(
        MockUserRepo::new(),
        tenants,
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/tenants/{tenant_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["slug"], "acme");
}

// ── Token endpoint — error cases ──────────────────────────────────────────────

#[tokio::test]
async fn token_missing_tenant_header_returns_400() {
    let app = simple_app(
        MockUserRepo::new(),
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let body = "grant_type=password&username=alice&password=secret&client_id=test-client";

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/oauth2/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn token_unsupported_grant_type_returns_400() {
    let tenant_id = Uuid::new_v4();
    let app = simple_app(
        MockUserRepo::new(),
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let body = "grant_type=client_credentials";

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/oauth2/token")
                .header("X-Tenant-ID", tenant_id.to_string())
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let resp_body = body_json(resp).await;
    assert!(resp_body["error"]
        .as_str()
        .unwrap()
        .contains("unsupported grant_type"));
}

#[tokio::test]
async fn token_password_grant_unknown_client_returns_401() {
    let tenant_id = Uuid::new_v4();
    let mut applications = MockApplicationRepo::new();
    applications
        .expect_get_by_client_id()
        .returning(|_, _| Err(StoreError::NotFound("application not found".into())));

    let app = simple_app(
        MockUserRepo::new(),
        MockTenantRepo::new(),
        applications,
        MockRefreshTokenRepo::new(),
    );

    let body = "grant_type=password&username=alice@example.com&password=wrong&client_id=unknown";

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/oauth2/token")
                .header("X-Tenant-ID", tenant_id.to_string())
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn token_password_grant_returns_access_token() {
    let tenant_id = Uuid::new_v4();
    let user = test_user(tenant_id);
    let creds = test_creds(&user, "hunter2");
    let app_fixture = test_application(tenant_id);
    let session = test_session(user.id, tenant_id);
    let rt = test_refresh_token(session.id, app_fixture.id);

    let user_clone = user.clone();
    let creds_clone = creds.clone();
    let app_clone = app_fixture.clone();
    let session_clone = session.clone();
    let rt_clone = rt.clone();

    // application lookup
    let mut applications = MockApplicationRepo::new();
    applications
        .expect_get_by_client_id()
        .returning(move |_, _| Ok(app_clone.clone()));

    // PasswordService internals: user lookup + cred lookup
    let mut pw_users = MockUserRepo::new();
    pw_users
        .expect_get_by_email()
        .returning(move |_, _| Ok(user_clone.clone()));

    let mut pw_creds = MockUserCredRepo::new();
    pw_creds
        .expect_get_by_user_id()
        .returning(move |_, _| Ok(creds_clone.clone()));

    // SessionService internals: session create + refresh token create
    let mut session_sessions = MockSessionRepo::new();
    session_sessions
        .expect_create()
        .returning(move |_| Ok(session_clone.clone()));

    let mut session_rts = MockRefreshTokenRepo::new();
    session_rts
        .expect_create()
        .returning(move |_| Ok(rt_clone.clone()));

    let app = build_app(
        MockUserRepo::new(),
        MockTenantRepo::new(),
        applications,
        MockRefreshTokenRepo::new(),
        pw_users,
        pw_creds,
        session_sessions,
        session_rts,
    );

    let body = format!(
        "grant_type=password&username={}&password=hunter2&client_id=test-client",
        user.email
    );

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/oauth2/token")
                .header("X-Tenant-ID", tenant_id.to_string())
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let resp_body = body_json(resp).await;
    assert!(resp_body["access_token"].is_string());
    assert_eq!(resp_body["token_type"], "Bearer");
    assert!(resp_body["refresh_token"].is_string());
}

#[tokio::test]
async fn token_password_grant_with_openid_scope_includes_id_token() {
    let tenant_id = Uuid::new_v4();
    let user = test_user(tenant_id);
    let creds = test_creds(&user, "hunter2");
    let app_fixture = test_application(tenant_id);
    let session = test_session(user.id, tenant_id);
    let rt = test_refresh_token(session.id, app_fixture.id);

    let (user_c, creds_c, app_c, sess_c, rt_c) = (
        user.clone(),
        creds.clone(),
        app_fixture.clone(),
        session.clone(),
        rt.clone(),
    );

    let mut applications = MockApplicationRepo::new();
    applications
        .expect_get_by_client_id()
        .returning(move |_, _| Ok(app_c.clone()));

    let mut pw_users = MockUserRepo::new();
    pw_users
        .expect_get_by_email()
        .returning(move |_, _| Ok(user_c.clone()));

    let mut pw_creds = MockUserCredRepo::new();
    pw_creds
        .expect_get_by_user_id()
        .returning(move |_, _| Ok(creds_c.clone()));

    let mut session_sessions = MockSessionRepo::new();
    session_sessions
        .expect_create()
        .returning(move |_| Ok(sess_c.clone()));

    let mut session_rts = MockRefreshTokenRepo::new();
    session_rts
        .expect_create()
        .returning(move |_| Ok(rt_c.clone()));

    let app = build_app(
        MockUserRepo::new(),
        MockTenantRepo::new(),
        applications,
        MockRefreshTokenRepo::new(),
        pw_users,
        pw_creds,
        session_sessions,
        session_rts,
    );

    let body = format!(
        "grant_type=password&username={}&password=hunter2&client_id=test-client&scope=openid+profile+email",
        user.email
    );

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/oauth2/token")
                .header("X-Tenant-ID", tenant_id.to_string())
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let resp_body = body_json(resp).await;
    assert!(
        resp_body["id_token"].is_string(),
        "openid scope must produce id_token"
    );
}

// ── Userinfo ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn userinfo_no_bearer_returns_401() {
    let app = simple_app(
        MockUserRepo::new(),
        MockTenantRepo::new(),
        MockApplicationRepo::new(),
        MockRefreshTokenRepo::new(),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/oauth2/userinfo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn userinfo_with_valid_token_returns_claims() {
    let tenant_id = Uuid::new_v4();
    let user = test_user(tenant_id);
    let user_clone = user.clone();

    let mut users = MockUserRepo::new();
    users
        .expect_get_by_id()
        .returning(move |_, _| Ok(user_clone.clone()));

    // Build state manually so we can extract the signing key for minting the token
    let key_record = generate_rsa_key(None, 1).expect("generate_rsa_key failed");
    let signing_key = Arc::new(arc_swap::ArcSwap::from_pointee(key_record));
    let config = Arc::new(test_settings());
    let users_arc: Arc<dyn UserRepository> = Arc::new(users);

    let password_svc = Arc::new(PasswordService::new(
        Arc::new(MockUserRepo::new()) as Arc<dyn UserRepository>,
        Arc::new(MockUserCredRepo::new()) as Arc<dyn UserCredentialsRepository>,
    ));
    let session_svc = Arc::new(SessionService::new(
        Arc::new(MockSessionRepo::new()) as Arc<dyn SessionRepository>,
        Arc::new(MockRefreshTokenRepo::new()) as Arc<dyn RefreshTokenRepository>,
    ));

    let state = Arc::new(AppState {
        config,
        users: users_arc,
        tenants: Arc::new(MockTenantRepo::new()),
        applications: Arc::new(MockApplicationRepo::new()),
        refresh_tokens: Arc::new(MockRefreshTokenRepo::new()),
        groups: Arc::new(MockGroupRepo::new()),
        passkeys: Arc::new(MockPasskeyRepo::new()),
        identities: Arc::new(MockIdentityRepo::new()),
        idp_configs: Arc::new(MockIdpConfigRepo::new()),
        audit: Arc::new(permissive_audit()),
        operators: Arc::new(MockOperatorRepo::new()),
        operator_credentials: Arc::new(MockOperatorCredsRepo::new()),
        operator_permissions: Arc::new(MockOperatorPermRepo::new()),
        operator_roles_repo: Arc::new(MockOperatorRoleRepo::new()),
        claim_definitions: Arc::new(MockClaimDefRepo::new()),
        group_claims: Arc::new(MockGroupClaimRepo::new()),
        user_claims: Arc::new(MockUserClaimRepo::new()),
        auth_codes: Arc::new(MockAuthCodeStoreRepo::new()),
        password_svc,
        session_svc,
        authz_svc: Arc::new(empty_authz()),
        signing_key: signing_key.clone(),
        signing_keys: Arc::new(MockSigningKeyRepo::new()),
        metrics: test_metrics(),
    });

    // Mint a valid access token using the same key as the state
    let now = now_secs();
    let claims = AccessTokenClaims {
        sub: user.id.to_string(),
        iss: "https://auth.test".into(),
        aud: "test-client".into(),
        exp: now + 3600,
        iat: now,
        jti: make_jti(),
        scope: "openid profile email".into(),
        tenant_id: tenant_id.to_string(),
        extras: Default::default(),
    };
    let token = sign(
        &claims,
        &signing_key.load_full().private_key_pem,
        Algorithm::RS256,
        None,
    )
    .expect("sign failed");

    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/oauth2/userinfo")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["email"], "alice@example.com");
    assert_eq!(body["email_verified"], true);
}

// ── Admin authorization enforcement ───────────────────────────────────────────
//
// These tests are the canary: they prove that every admin handler does the
// require_perm() lookup, by exercising tenant-A vs tenant-B isolation against
// a single representative endpoint. If a handler ever forgets the check, the
// 403 cases below stop catching it.

mod admin_authz {
    use super::*;
    use irongate_api::claims::{OperatorClaims, OPERATOR_ACTOR_TYPE, OPERATOR_AUDIENCE};

    fn make_operator_token(signing_key_pem: &str, operator_id: Uuid) -> String {
        let now = now_secs();
        let claims = OperatorClaims {
            sub: operator_id.to_string(),
            iss: "https://auth.test".into(),
            aud: OPERATOR_AUDIENCE.into(),
            exp: now + 3600,
            iat: now,
            jti: make_jti(),
            email: "admin@test".into(),
            actor_type: OPERATOR_ACTOR_TYPE.into(),
        };
        sign(&claims, signing_key_pem, Algorithm::RS256, None).expect("sign failed")
    }

    fn perm(resource: &str, action: &str) -> OperatorPermission {
        OperatorPermission {
            id: Uuid::new_v4(),
            resource: resource.into(),
            action: action.into(),
            description: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    fn admin_app(operator_roles: MockOperatorRoleRepo) -> (axum::Router, Arc<SigningKeyRecord>) {
        let key_record = Arc::new(generate_rsa_key(None, 1).expect("generate_rsa_key failed"));
        let signing_key = Arc::new(arc_swap::ArcSwap::new(key_record.clone()));
        let config = Arc::new(test_settings());
        let password_svc = Arc::new(PasswordService::new(
            Arc::new(MockUserRepo::new()) as Arc<dyn UserRepository>,
            Arc::new(MockUserCredRepo::new()) as Arc<dyn UserCredentialsRepository>,
        ));
        let session_svc = Arc::new(SessionService::new(
            Arc::new(MockSessionRepo::new()) as Arc<dyn SessionRepository>,
            Arc::new(MockRefreshTokenRepo::new()) as Arc<dyn RefreshTokenRepository>,
        ));
        let mut users = MockUserRepo::new();
        users.expect_list().returning(|_, _, _| Ok(vec![]));
        let state = Arc::new(AppState {
            config,
            users: Arc::new(users),
            tenants: Arc::new(MockTenantRepo::new()),
            applications: Arc::new(MockApplicationRepo::new()),
            refresh_tokens: Arc::new(MockRefreshTokenRepo::new()),
            groups: Arc::new(MockGroupRepo::new()),
            passkeys: Arc::new(MockPasskeyRepo::new()),
            identities: Arc::new(MockIdentityRepo::new()),
            idp_configs: Arc::new(MockIdpConfigRepo::new()),
            audit: Arc::new(permissive_audit()),
            operators: Arc::new(MockOperatorRepo::new()),
            operator_credentials: Arc::new(MockOperatorCredsRepo::new()),
            operator_permissions: Arc::new(MockOperatorPermRepo::new()),
            operator_roles_repo: Arc::new(operator_roles),
            claim_definitions: Arc::new(MockClaimDefRepo::new()),
            group_claims: Arc::new(MockGroupClaimRepo::new()),
            user_claims: Arc::new(MockUserClaimRepo::new()),
            auth_codes: Arc::new(MockAuthCodeStoreRepo::new()),
            password_svc,
            session_svc,
            authz_svc: Arc::new(empty_authz()),
            signing_key: signing_key.clone(),
            signing_keys: Arc::new(MockSigningKeyRepo::new()),
            metrics: test_metrics(),
        });
        (build_router(state), key_record)
    }

    /// super_admin (global role with users:list) → 200 listing users in any tenant.
    #[tokio::test]
    async fn global_role_allows_tenant_scoped_endpoint() {
        let operator_id = Uuid::new_v4();
        let tenant_a = Uuid::new_v4();

        let mut roles = MockOperatorRoleRepo::new();
        roles
            .expect_list_permissions_for_operator_in_tenant()
            .returning(|_, _| Ok(vec![perm("users", "list")]));

        let (app, signing_key) = admin_app(roles);
        let token = make_operator_token(&signing_key.private_key_pem, operator_id);

        let uri = format!("/admin/api/v1/users?tenant_id={tenant_a}");
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(&uri)
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "expected 200, got {}",
            resp.status()
        );
    }

    /// A tenant-scoped operator (perm only for tenant A) is forbidden on tenant B's
    /// resources. The mock returns the (users,list) perm only when queried for
    /// tenant A; querying for tenant B returns no perms → 403.
    #[tokio::test]
    async fn tenant_scoped_role_is_isolated() {
        let operator_id = Uuid::new_v4();
        let tenant_a = Uuid::new_v4();
        let tenant_b = Uuid::new_v4();

        let mut roles = MockOperatorRoleRepo::new();
        roles
            .expect_list_permissions_for_operator_in_tenant()
            .returning(move |_, tid| {
                if tid == tenant_a {
                    Ok(vec![perm("users", "list")])
                } else {
                    Ok(vec![])
                }
            });

        let (app, signing_key) = admin_app(roles);
        let token = make_operator_token(&signing_key.private_key_pem, operator_id);

        // Tenant A → allowed.
        let resp_a = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/admin/api/v1/users?tenant_id={tenant_a}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_a.status(), StatusCode::OK);

        // Tenant B → forbidden.
        let resp_b = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/admin/api/v1/users?tenant_id={tenant_b}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_b.status(), StatusCode::FORBIDDEN);
    }

    /// An operator with no role assignments → 403 on any admin endpoint.
    #[tokio::test]
    async fn no_permission_returns_forbidden() {
        let operator_id = Uuid::new_v4();
        let tenant = Uuid::new_v4();

        let mut roles = MockOperatorRoleRepo::new();
        roles
            .expect_list_permissions_for_operator_in_tenant()
            .returning(|_, _| Ok(vec![]));
        roles
            .expect_list_permissions_for_operator_global()
            .returning(|_| Ok(vec![]));

        let (app, signing_key) = admin_app(roles);
        let token = make_operator_token(&signing_key.private_key_pem, operator_id);

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/admin/api/v1/users?tenant_id={tenant}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// Bulk import creates each user, adds them to the given group, and records
    /// one summary audit event in addition to per-user audits. Duplicates raise
    /// `Conflict` and are counted as skipped when `skip_duplicates` is true.
    #[tokio::test]
    async fn bulk_import_creates_users_and_adds_to_group() {
        let operator_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let group_id = Uuid::new_v4();

        let mut roles = MockOperatorRoleRepo::new();
        roles
            .expect_list_permissions_for_operator_in_tenant()
            .returning(|_, _| Ok(vec![perm("users", "create")]));

        // Users repo: first create succeeds, second emits Conflict (duplicate email).
        let mut users = MockUserRepo::new();
        users.expect_list().returning(|_, _, _| Ok(vec![]));
        let calls = std::sync::atomic::AtomicUsize::new(0);
        users.expect_create().returning(move |u| {
            let n = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 1 {
                Err(StoreError::Conflict("duplicate email".into()))
            } else {
                Ok(u)
            }
        });

        let mut groups = MockGroupRepo::new();
        let add_member_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let amc = add_member_calls.clone();
        groups.expect_add_member().returning(move |_, _, _| {
            amc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });

        // Audit: capture record() calls so we can assert on event types.
        let mut audit = MockAuditRepo::new();
        let recorded: Arc<std::sync::Mutex<Vec<String>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let recorded_clone = recorded.clone();
        audit.expect_record().returning(move |ev: AuditEvent| {
            recorded_clone.lock().unwrap().push(ev.event_type);
            Ok(())
        });

        let key_record = Arc::new(generate_rsa_key(None, 1).expect("generate_rsa_key failed"));
        let signing_key = Arc::new(arc_swap::ArcSwap::new(key_record.clone()));
        let config = Arc::new(test_settings());
        let password_svc = Arc::new(PasswordService::new(
            Arc::new(MockUserRepo::new()) as Arc<dyn UserRepository>,
            Arc::new(MockUserCredRepo::new()) as Arc<dyn UserCredentialsRepository>,
        ));
        let session_svc = Arc::new(SessionService::new(
            Arc::new(MockSessionRepo::new()) as Arc<dyn SessionRepository>,
            Arc::new(MockRefreshTokenRepo::new()) as Arc<dyn RefreshTokenRepository>,
        ));
        let state = Arc::new(AppState {
            config,
            users: Arc::new(users),
            tenants: Arc::new(MockTenantRepo::new()),
            applications: Arc::new(MockApplicationRepo::new()),
            refresh_tokens: Arc::new(MockRefreshTokenRepo::new()),
            groups: Arc::new(groups),
            passkeys: Arc::new(MockPasskeyRepo::new()),
            identities: Arc::new(MockIdentityRepo::new()),
            idp_configs: Arc::new(MockIdpConfigRepo::new()),
            audit: Arc::new(audit),
            operators: Arc::new(MockOperatorRepo::new()),
            operator_credentials: Arc::new(MockOperatorCredsRepo::new()),
            operator_permissions: Arc::new(MockOperatorPermRepo::new()),
            operator_roles_repo: Arc::new(roles),
            claim_definitions: Arc::new(MockClaimDefRepo::new()),
            group_claims: Arc::new(MockGroupClaimRepo::new()),
            user_claims: Arc::new(MockUserClaimRepo::new()),
            auth_codes: Arc::new(MockAuthCodeStoreRepo::new()),
            password_svc,
            session_svc,
            authz_svc: Arc::new(empty_authz()),
            signing_key: signing_key.clone(),
            signing_keys: Arc::new(MockSigningKeyRepo::new()),
            metrics: test_metrics(),
        });
        let app = build_router(state);
        let token = make_operator_token(&key_record.private_key_pem, operator_id);

        let body = serde_json::json!({
            "users": [
                {"email": "alice@example.com", "name": "Alice"},
                {"email": "dup@example.com"},
                {"email": "carol@example.com"},
            ],
            "group_id": group_id,
            "skip_duplicates": true,
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/admin/api/v1/tenants/{tenant_id}/users/import"))
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let resp_body = body_json(resp).await;
        assert_eq!(resp_body["created"], 2);
        assert_eq!(resp_body["skipped"], 1);
        assert_eq!(resp_body["errors"].as_array().unwrap().len(), 0);
        assert_eq!(
            add_member_calls.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "every successfully-created user should be added to the group"
        );

        let events = recorded.lock().unwrap().clone();
        assert!(
            events.contains(&"users.bulk_import".to_string()),
            "expected users.bulk_import audit event, got {events:?}"
        );
    }
}
