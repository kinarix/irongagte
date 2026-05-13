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
use irongate_core::{
    errors::StoreError,
    repositories::{
        ApplicationRepository, RefreshTokenRepository, SessionRepository,
        TenantRepository, UserCredentialsRepository, UserRepository,
    },
    Application, AppType, RefreshToken, Session, Tenant, User, UserCredentials, UserStatus,
};
use irongate_crypto::{
    hash::hash_password,
    jwt::sign,
    keys::{generate_rsa_key, KeyAlgorithm},
};
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

// ── Fixtures ──────────────────────────────────────────────────────────────────

fn test_settings() -> Settings {
    Settings {
        server: ServerConfig { host: "127.0.0.1".into(), port: 3000 },
        database: DatabaseConfig {
            url: "postgres://localhost/test".into(),
            max_connections: 1,
        },
        redis: RedisConfig { url: "redis://localhost".into() },
        base_url: "https://auth.test".into(),
        log: LogConfig { level: "off".into(), format: "json".into() },
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
    let signing_key = Arc::new(
        generate_rsa_key(Uuid::nil(), 1).expect("failed to generate RSA key"),
    );
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
        password_svc,
        session_svc,
        signing_key,
    });

    build_router(state)
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
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
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
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
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
        .oneshot(Request::builder().uri("/api/v1/users").body(Body::empty()).unwrap())
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
    users.expect_create().returning(|u| Ok(u));

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
    users.expect_get_by_id().returning(|id, _| {
        Err(StoreError::NotFound(format!("user {id} not found")))
    });

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
        .oneshot(Request::builder().uri("/api/v1/tenants").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["tenants"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn create_tenant_returns_201() {
    let mut tenants = MockTenantRepo::new();
    tenants.expect_create().returning(|t| Ok(t));

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
    tenants.expect_get_by_id().returning(move |_| Ok(tenant_clone.clone()));

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
    applications.expect_get_by_client_id().returning(|_, _| {
        Err(StoreError::NotFound("application not found".into()))
    });

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

    let (user_c, creds_c, app_c, sess_c, rt_c) =
        (user.clone(), creds.clone(), app_fixture.clone(), session.clone(), rt.clone());

    let mut applications = MockApplicationRepo::new();
    applications.expect_get_by_client_id().returning(move |_, _| Ok(app_c.clone()));

    let mut pw_users = MockUserRepo::new();
    pw_users.expect_get_by_email().returning(move |_, _| Ok(user_c.clone()));

    let mut pw_creds = MockUserCredRepo::new();
    pw_creds.expect_get_by_user_id().returning(move |_, _| Ok(creds_c.clone()));

    let mut session_sessions = MockSessionRepo::new();
    session_sessions.expect_create().returning(move |_| Ok(sess_c.clone()));

    let mut session_rts = MockRefreshTokenRepo::new();
    session_rts.expect_create().returning(move |_| Ok(rt_c.clone()));

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
    assert!(resp_body["id_token"].is_string(), "openid scope must produce id_token");
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
    let signing_key = Arc::new(
        generate_rsa_key(Uuid::nil(), 1).expect("generate_rsa_key failed"),
    );
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
        password_svc,
        session_svc,
        signing_key: signing_key.clone(),
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
    };
    let token = sign(&claims, &signing_key.private_key_pem, Algorithm::RS256, None)
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
