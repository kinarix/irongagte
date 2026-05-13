use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use irongate_core::{
    errors::StoreError,
    repositories::{GroupRepository, UserRepository},
    types::{Group, User, UserStatus},
};
use irongate_scim::{groups::GroupState, router::scim_router, users::UserState};
use mockall::mock;
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

// ── Mocks ──────────────────────────────────────────────────────────────────────

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
    GroupRepo {}
    #[async_trait::async_trait]
    impl GroupRepository for GroupRepo {
        async fn create(&self, group: Group) -> Result<Group, StoreError>;
        async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Group, StoreError>;
        async fn get_by_display_name(
            &self,
            display_name: &str,
            tenant_id: Uuid,
        ) -> Result<Group, StoreError>;
        async fn update(&self, group: Group) -> Result<Group, StoreError>;
        async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
        async fn list(
            &self,
            tenant_id: Uuid,
            limit: i64,
            offset: i64,
        ) -> Result<Vec<Group>, StoreError>;
        async fn add_member(
            &self,
            group_id: Uuid,
            user_id: Uuid,
            tenant_id: Uuid,
        ) -> Result<(), StoreError>;
        async fn remove_member(
            &self,
            group_id: Uuid,
            user_id: Uuid,
            tenant_id: Uuid,
        ) -> Result<(), StoreError>;
        async fn list_members(
            &self,
            group_id: Uuid,
            tenant_id: Uuid,
        ) -> Result<Vec<User>, StoreError>;
        async fn list_for_user(
            &self,
            user_id: Uuid,
            tenant_id: Uuid,
        ) -> Result<Vec<Group>, StoreError>;
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn make_user() -> User {
    let now = OffsetDateTime::now_utc();
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
        created_at: now,
        updated_at: now,
        last_login_at: None,
        deleted_at: None,
    }
}

fn make_group() -> Group {
    let now = OffsetDateTime::now_utc();
    Group {
        id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        display_name: "Engineering".into(),
        external_id: None,
        created_at: now,
        updated_at: now,
    }
}

/// Build a test app with the given user/group repos for their respective states.
/// Pass separate mocks for each slot — they are independent Arc<dyn ...>.
fn build_app(
    user_state_users: MockUserRepo,
    user_state_groups: MockGroupRepo,
    group_state_groups: MockGroupRepo,
    group_state_users: MockUserRepo,
    tenant_id: Uuid,
) -> Router {
    let base_url = "http://localhost:8080".to_string();

    let user_state = Arc::new(UserState {
        users: Arc::new(user_state_users),
        groups: Arc::new(user_state_groups),
        base_url: base_url.clone(),
        tenant_id,
    });

    let group_state = Arc::new(GroupState {
        groups: Arc::new(group_state_groups),
        users: Arc::new(group_state_users),
        base_url,
        tenant_id,
    });

    Router::new().nest("/scim/v2", scim_router(user_state, group_state))
}

// ── User tests ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_users_empty() {
    let tenant_id = Uuid::new_v4();

    let mut user_repo = MockUserRepo::new();
    user_repo.expect_list().returning(|_, _, _| Ok(vec![]));

    let app = build_app(
        user_repo,
        MockGroupRepo::new(),
        MockGroupRepo::new(),
        MockUserRepo::new(),
        tenant_id,
    );

    let resp = app
        .oneshot(Request::builder().uri("/scim/v2/Users").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["totalResults"], 0);
    assert_eq!(json["Resources"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_users_returns_resources() {
    let tenant_id = Uuid::new_v4();
    let user = make_user();
    let user_clone = user.clone();

    let mut user_repo = MockUserRepo::new();
    user_repo
        .expect_list()
        .returning(move |_, _, _| Ok(vec![user_clone.clone()]));

    let app = build_app(
        user_repo,
        MockGroupRepo::new(),
        MockGroupRepo::new(),
        MockUserRepo::new(),
        tenant_id,
    );

    let resp = app
        .oneshot(Request::builder().uri("/scim/v2/Users").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["totalResults"], 1);
    assert_eq!(json["Resources"][0]["userName"], "alice@example.com");
}

#[tokio::test]
async fn get_user_not_found() {
    let tenant_id = Uuid::new_v4();

    let mut user_repo = MockUserRepo::new();
    user_repo
        .expect_get_by_id()
        .returning(|_, _| Err(StoreError::NotFound("user".into())));

    let app = build_app(
        user_repo,
        MockGroupRepo::new(),
        MockGroupRepo::new(),
        MockUserRepo::new(),
        tenant_id,
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/scim/v2/Users/{}", Uuid::new_v4()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_user_returns_201() {
    let tenant_id = Uuid::new_v4();
    let user = make_user();
    let user_clone = user.clone();

    let mut user_repo = MockUserRepo::new();
    user_repo
        .expect_create()
        .returning(move |_| Ok(user_clone.clone()));

    let app = build_app(
        user_repo,
        MockGroupRepo::new(),
        MockGroupRepo::new(),
        MockUserRepo::new(),
        tenant_id,
    );

    let body = serde_json::json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",
        "active": true
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/scim/v2/Users")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["userName"], "alice@example.com");
    assert!(json["active"].as_bool().unwrap());
}

#[tokio::test]
async fn delete_user_returns_204() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let mut user_repo = MockUserRepo::new();
    user_repo.expect_soft_delete().returning(|_, _| Ok(()));

    let app = build_app(
        user_repo,
        MockGroupRepo::new(),
        MockGroupRepo::new(),
        MockUserRepo::new(),
        tenant_id,
    );

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/scim/v2/Users/{user_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn invalid_uuid_returns_400() {
    let tenant_id = Uuid::new_v4();

    let app = build_app(
        MockUserRepo::new(),
        MockGroupRepo::new(),
        MockGroupRepo::new(),
        MockUserRepo::new(),
        tenant_id,
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/scim/v2/Users/not-a-valid-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ── Group tests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_groups_empty() {
    let tenant_id = Uuid::new_v4();

    let mut group_repo = MockGroupRepo::new();
    group_repo.expect_list().returning(|_, _, _| Ok(vec![]));

    let app = build_app(
        MockUserRepo::new(),
        MockGroupRepo::new(),
        group_repo,
        MockUserRepo::new(),
        tenant_id,
    );

    let resp = app
        .oneshot(Request::builder().uri("/scim/v2/Groups").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["totalResults"], 0);
}

#[tokio::test]
async fn create_group_returns_201() {
    let tenant_id = Uuid::new_v4();
    let group = make_group();
    let group_clone = group.clone();

    let mut group_repo = MockGroupRepo::new();
    group_repo
        .expect_create()
        .returning(move |_| Ok(group_clone.clone()));
    group_repo
        .expect_list_members()
        .returning(|_, _| Ok(vec![]));

    let app = build_app(
        MockUserRepo::new(),
        MockGroupRepo::new(),
        group_repo,
        MockUserRepo::new(),
        tenant_id,
    );

    let body = serde_json::json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Engineering",
        "members": []
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/scim/v2/Groups")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["displayName"], "Engineering");
}

#[tokio::test]
async fn delete_group_returns_204() {
    let tenant_id = Uuid::new_v4();
    let group_id = Uuid::new_v4();

    let mut group_repo = MockGroupRepo::new();
    group_repo.expect_delete().returning(|_, _| Ok(()));

    let app = build_app(
        MockUserRepo::new(),
        MockGroupRepo::new(),
        group_repo,
        MockUserRepo::new(),
        tenant_id,
    );

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/scim/v2/Groups/{group_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn get_group_with_members() {
    let tenant_id = Uuid::new_v4();
    let group = make_group();
    let group_clone = group.clone();
    let group_id = group.id;
    let member = make_user();
    let member_clone = member.clone();

    let mut group_repo = MockGroupRepo::new();
    group_repo
        .expect_get_by_id()
        .returning(move |_, _| Ok(group_clone.clone()));
    group_repo
        .expect_list_members()
        .returning(move |_, _| Ok(vec![member_clone.clone()]));

    let app = build_app(
        MockUserRepo::new(),
        MockGroupRepo::new(),
        group_repo,
        MockUserRepo::new(),
        tenant_id,
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/scim/v2/Groups/{group_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["displayName"], "Engineering");
    assert_eq!(json["members"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn patch_user_active_field() {
    let tenant_id = Uuid::new_v4();
    let mut user = make_user();
    user.status = UserStatus::Suspended;
    let user_id = user.id;
    let user_clone = user.clone();
    let mut updated = user.clone();
    updated.status = UserStatus::Active;
    let updated_clone = updated.clone();

    let mut user_repo = MockUserRepo::new();
    user_repo
        .expect_get_by_id()
        .returning(move |_, _| Ok(user_clone.clone()));
    user_repo
        .expect_update()
        .returning(move |_| Ok(updated_clone.clone()));

    let app = build_app(
        user_repo,
        MockGroupRepo::new(),
        MockGroupRepo::new(),
        MockUserRepo::new(),
        tenant_id,
    );

    let body = serde_json::json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
        "Operations": [
            { "op": "replace", "path": "active", "value": true }
        ]
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/scim/v2/Users/{user_id}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["active"].as_bool().unwrap());
}
