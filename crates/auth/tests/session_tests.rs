use std::sync::Arc;

use irongate_auth::SessionService;
use irongate_core::{
    errors::{AuthError, StoreError},
    repositories::{RefreshTokenRepository, SessionRepository},
    RefreshToken, Session,
};
use mockall::mock;
use time::OffsetDateTime;
use uuid::Uuid;

mock! {
    SessionRepo {}
    #[async_trait::async_trait]
    impl SessionRepository for SessionRepo {
        async fn create(&self, session: Session) -> Result<Session, StoreError>;
        async fn get_by_id(&self, id: Uuid) -> Result<Session, StoreError>;
        async fn revoke(&self, id: Uuid) -> Result<(), StoreError>;
        async fn revoke_all_for_user(&self, user_id: Uuid, tenant_id: Uuid) -> Result<u64, StoreError>;
        async fn list_for_user(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Vec<Session>, StoreError>;
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

fn active_session(user_id: Uuid, tenant_id: Uuid) -> Session {
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

fn expired_session(user_id: Uuid, tenant_id: Uuid) -> Session {
    let past = OffsetDateTime::now_utc() - time::Duration::hours(2);
    Session {
        id: Uuid::new_v4(),
        user_id,
        tenant_id,
        idp_id: None,
        ip_address: None,
        user_agent: None,
        created_at: past - time::Duration::hours(8),
        expires_at: past,
        revoked_at: None,
    }
}

#[tokio::test]
async fn validate_session_ok() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let session = active_session(user_id, tenant_id);
    let session_id = session.id;

    let mut mock_sessions = MockSessionRepo::new();
    let session_clone = session.clone();
    mock_sessions
        .expect_get_by_id()
        .once()
        .returning(move |_| Ok(session_clone.clone()));

    let mock_tokens = MockRefreshTokenRepo::new();

    let svc = SessionService::new(Arc::new(mock_sessions), Arc::new(mock_tokens));
    let result = svc.validate_session(session_id).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, session_id);
}

#[tokio::test]
async fn validate_expired_session() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let session = expired_session(user_id, tenant_id);

    let mut mock_sessions = MockSessionRepo::new();
    let session_clone = session.clone();
    mock_sessions
        .expect_get_by_id()
        .once()
        .returning(move |_| Ok(session_clone.clone()));

    let mock_tokens = MockRefreshTokenRepo::new();

    let svc = SessionService::new(Arc::new(mock_sessions), Arc::new(mock_tokens));
    let result = svc.validate_session(session.id).await;
    assert!(matches!(result, Err(AuthError::SessionExpired)));
}

#[tokio::test]
async fn validate_session_not_found() {
    let mut mock_sessions = MockSessionRepo::new();
    mock_sessions
        .expect_get_by_id()
        .once()
        .returning(|_| Err(StoreError::NotFound("session".into())));

    let mock_tokens = MockRefreshTokenRepo::new();

    let svc = SessionService::new(Arc::new(mock_sessions), Arc::new(mock_tokens));
    let result = svc.validate_session(Uuid::new_v4()).await;
    assert!(matches!(result, Err(AuthError::SessionNotFound)));
}

#[tokio::test]
async fn rotate_refresh_token_success() {
    use irongate_crypto::token::{generate_token, hash_token};

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let session = active_session(user_id, tenant_id);

    let raw_token = generate_token();
    let token_hash = hash_token(&raw_token);

    let now = OffsetDateTime::now_utc();
    let old_rt = RefreshToken {
        id: Uuid::new_v4(),
        session_id: session.id,
        application_id: Uuid::new_v4(),
        token_hash: token_hash.clone(),
        scope: "openid".into(),
        previous_id: None,
        created_at: now,
        expires_at: now + time::Duration::hours(24),
        revoked_at: None,
    };

    let mut mock_sessions = MockSessionRepo::new();
    let session_clone = session.clone();
    mock_sessions
        .expect_get_by_id()
        .once()
        .returning(move |_| Ok(session_clone.clone()));

    let mut mock_tokens = MockRefreshTokenRepo::new();
    let old_rt_clone = old_rt.clone();
    mock_tokens
        .expect_get_by_hash()
        .once()
        .returning(move |_| Ok(old_rt_clone.clone()));
    mock_tokens.expect_revoke().once().returning(|_| Ok(()));
    mock_tokens.expect_create().once().returning(Ok);

    let svc = SessionService::new(Arc::new(mock_sessions), Arc::new(mock_tokens));
    let result = svc.rotate_refresh_token(&raw_token, tenant_id, 86400).await;
    assert!(result.is_ok());
    let (returned_session, new_raw) = result.unwrap();
    assert_eq!(returned_session.id, session.id);
    assert!(!new_raw.is_empty());
    // New token must differ from the old one.
    assert_ne!(new_raw, raw_token);
}

#[tokio::test]
async fn rotate_refresh_token_not_found() {
    let tenant_id = Uuid::new_v4();

    let mock_sessions = MockSessionRepo::new();
    let mut mock_tokens = MockRefreshTokenRepo::new();
    mock_tokens
        .expect_get_by_hash()
        .once()
        .returning(|_| Err(StoreError::NotFound("token".into())));

    let svc = SessionService::new(Arc::new(mock_sessions), Arc::new(mock_tokens));
    let result = svc
        .rotate_refresh_token("bogus_token_value", tenant_id, 86400)
        .await;
    assert!(matches!(result, Err(AuthError::TokenExpired)));
}

#[tokio::test]
async fn revoke_session_ok() {
    let session_id = Uuid::new_v4();

    let mut mock_sessions = MockSessionRepo::new();
    mock_sessions.expect_revoke().once().returning(|_| Ok(()));

    let mut mock_tokens = MockRefreshTokenRepo::new();
    mock_tokens
        .expect_revoke_all_for_session()
        .once()
        .returning(|_| Ok(0));

    let svc = SessionService::new(Arc::new(mock_sessions), Arc::new(mock_tokens));
    let result = svc.revoke_session(session_id).await;
    assert!(result.is_ok());
}
