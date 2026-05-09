use std::sync::Arc;

use irongate_auth::PasswordService;
use irongate_core::{
    errors::{AuthError, StoreError},
    repositories::{UserCredentialsRepository, UserRepository},
    User, UserCredentials, UserStatus,
};
use mockall::mock;
use time::OffsetDateTime;
use uuid::Uuid;

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
    UserCredsRepo {}
    #[async_trait::async_trait]
    impl UserCredentialsRepository for UserCredsRepo {
        async fn create(&self, creds: UserCredentials) -> Result<UserCredentials, StoreError>;
        async fn get_by_user_id(&self, user_id: Uuid, tenant_id: Uuid) -> Result<UserCredentials, StoreError>;
        async fn update(&self, creds: UserCredentials) -> Result<UserCredentials, StoreError>;
        async fn delete(&self, user_id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    }
}

fn make_user(status: UserStatus) -> User {
    User {
        id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        email: "alice@example.com".into(),
        email_verified: true,
        name: None,
        given_name: None,
        family_name: None,
        picture_url: None,
        status,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        last_login_at: None,
        deleted_at: None,
    }
}

fn make_creds_with_password(user: &User, password: &str) -> UserCredentials {
    use irongate_crypto::hash::hash_password;
    UserCredentials {
        id: Uuid::new_v4(),
        tenant_id: user.tenant_id,
        user_id: user.id,
        password_hash: Some(hash_password(password).unwrap()),
        totp_secret: None,
        totp_enabled: false,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    }
}

#[tokio::test]
async fn authenticate_success() {
    let user = make_user(UserStatus::Active);
    let tenant_id = user.tenant_id;
    let creds = make_creds_with_password(&user, "hunter2");

    let mut mock_users = MockUserRepo::new();
    let user_clone = user.clone();
    mock_users
        .expect_get_by_email()
        .once()
        .returning(move |_, _| Ok(user_clone.clone()));

    let mut mock_creds = MockUserCredsRepo::new();
    let creds_clone = creds.clone();
    mock_creds
        .expect_get_by_user_id()
        .once()
        .returning(move |_, _| Ok(creds_clone.clone()));

    let svc = PasswordService::new(Arc::new(mock_users), Arc::new(mock_creds));
    let result = svc.authenticate("alice@example.com", "hunter2", tenant_id).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, user.id);
}

#[tokio::test]
async fn authenticate_wrong_password() {
    let user = make_user(UserStatus::Active);
    let tenant_id = user.tenant_id;
    let creds = make_creds_with_password(&user, "correct");

    let mut mock_users = MockUserRepo::new();
    let user_clone = user.clone();
    mock_users
        .expect_get_by_email()
        .once()
        .returning(move |_, _| Ok(user_clone.clone()));

    let mut mock_creds = MockUserCredsRepo::new();
    let creds_clone = creds.clone();
    mock_creds
        .expect_get_by_user_id()
        .once()
        .returning(move |_, _| Ok(creds_clone.clone()));

    let svc = PasswordService::new(Arc::new(mock_users), Arc::new(mock_creds));
    let result = svc.authenticate("alice@example.com", "wrong", tenant_id).await;
    assert!(matches!(result, Err(AuthError::InvalidCredentials)));
}

#[tokio::test]
async fn authenticate_unknown_user() {
    let tenant_id = Uuid::new_v4();

    let mut mock_users = MockUserRepo::new();
    mock_users
        .expect_get_by_email()
        .once()
        .returning(|_, _| Err(StoreError::NotFound("user".into())));

    let mock_creds = MockUserCredsRepo::new();

    let svc = PasswordService::new(Arc::new(mock_users), Arc::new(mock_creds));
    let result = svc.authenticate("nobody@example.com", "pass", tenant_id).await;
    assert!(matches!(result, Err(AuthError::InvalidCredentials)));
}

#[tokio::test]
async fn authenticate_suspended_user() {
    let user = make_user(UserStatus::Suspended);
    let tenant_id = user.tenant_id;

    let mut mock_users = MockUserRepo::new();
    let user_clone = user.clone();
    mock_users
        .expect_get_by_email()
        .once()
        .returning(move |_, _| Ok(user_clone.clone()));

    let mock_creds = MockUserCredsRepo::new();

    let svc = PasswordService::new(Arc::new(mock_users), Arc::new(mock_creds));
    let result = svc.authenticate("alice@example.com", "pass", tenant_id).await;
    assert!(matches!(result, Err(AuthError::AccountSuspended)));
}

#[tokio::test]
async fn authenticate_no_password_set() {
    let user = make_user(UserStatus::Active);
    let tenant_id = user.tenant_id;

    let creds = UserCredentials {
        id: Uuid::new_v4(),
        tenant_id: user.tenant_id,
        user_id: user.id,
        password_hash: None,
        totp_secret: None,
        totp_enabled: false,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    };

    let mut mock_users = MockUserRepo::new();
    let user_clone = user.clone();
    mock_users
        .expect_get_by_email()
        .once()
        .returning(move |_, _| Ok(user_clone.clone()));

    let mut mock_creds = MockUserCredsRepo::new();
    let creds_clone = creds.clone();
    mock_creds
        .expect_get_by_user_id()
        .once()
        .returning(move |_, _| Ok(creds_clone.clone()));

    let svc = PasswordService::new(Arc::new(mock_users), Arc::new(mock_creds));
    let result = svc.authenticate("alice@example.com", "anypass", tenant_id).await;
    assert!(matches!(result, Err(AuthError::InvalidCredentials)));
}
