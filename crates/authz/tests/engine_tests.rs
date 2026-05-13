use std::{collections::HashMap, sync::Arc};

use irongate_authz::{
    engine::AuthzService,
    policy::{AbacPolicy, Condition, EvaluationContext, Operator, PolicyEffect},
    scope::{resolve_scopes, scopes_grant},
    policy::{evaluate, policy_allows, policy_denies},
};
use irongate_core::{
    errors::StoreError,
    repositories::{PermissionRepository, RoleRepository},
    types::{Permission, Role},
};
use mockall::mock;
use time::OffsetDateTime;
use uuid::Uuid;

// ── Mocks ─────────────────────────────────────────────────────────────────────

mock! {
    RoleRepo {}
    #[async_trait::async_trait]
    impl RoleRepository for RoleRepo {
        async fn create(&self, role: Role) -> Result<Role, StoreError>;
        async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Role, StoreError>;
        async fn get_by_name(&self, name: &str, tenant_id: Uuid) -> Result<Role, StoreError>;
        async fn update(&self, role: Role) -> Result<Role, StoreError>;
        async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
        async fn list(&self, tenant_id: Uuid) -> Result<Vec<Role>, StoreError>;
        async fn get_roles_for_user(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Vec<Role>, StoreError>;
        async fn assign_role_to_user(&self, user_id: Uuid, role_id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
        async fn remove_role_from_user(&self, user_id: Uuid, role_id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    }
}

mock! {
    PermRepo {}
    #[async_trait::async_trait]
    impl PermissionRepository for PermRepo {
        async fn create(&self, permission: Permission) -> Result<Permission, StoreError>;
        async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Permission, StoreError>;
        async fn list(&self, tenant_id: Uuid) -> Result<Vec<Permission>, StoreError>;
        async fn get_permissions_for_role(&self, role_id: Uuid, tenant_id: Uuid) -> Result<Vec<Permission>, StoreError>;
        async fn assign_permission_to_role(&self, role_id: Uuid, permission_id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
        async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_service(roles: MockRoleRepo, perms: MockPermRepo) -> AuthzService {
    AuthzService::new(Arc::new(roles), Arc::new(perms))
}

fn make_role(name: &str, tenant_id: Uuid) -> Role {
    Role {
        id: Uuid::new_v4(),
        tenant_id,
        name: name.into(),
        description: None,
        parent_role_id: None,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    }
}

fn make_permission(resource: &str, action: &str, tenant_id: Uuid) -> Permission {
    Permission {
        id: Uuid::new_v4(),
        tenant_id,
        resource: resource.into(),
        action: action.into(),
        description: None,
        created_at: OffsetDateTime::now_utc(),
    }
}

fn make_policy(resource: &str, action: &str, effect: PolicyEffect, conditions: Vec<Condition>) -> AbacPolicy {
    AbacPolicy {
        id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        name: "test-policy".into(),
        resource: resource.into(),
        action: action.into(),
        effect,
        conditions,
    }
}

// ── AuthzService — RBAC ───────────────────────────────────────────────────────

#[tokio::test]
async fn check_permission_returns_true_when_role_has_permission() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let role = make_role("admin", tenant_id);
    let perm = make_permission("users", "read", tenant_id);

    let role_clone = role.clone();
    let perm_clone = perm.clone();

    let mut roles = MockRoleRepo::new();
    roles
        .expect_get_roles_for_user()
        .once()
        .returning(move |_, _| Ok(vec![role_clone.clone()]));
    roles.expect_get_by_id().never();

    let mut perms = MockPermRepo::new();
    perms
        .expect_get_permissions_for_role()
        .once()
        .returning(move |_, _| Ok(vec![perm_clone.clone()]));

    let svc = make_service(roles, perms);
    assert!(svc.check_permission(user_id, tenant_id, "users", "read").await.unwrap());
}

#[tokio::test]
async fn check_permission_returns_false_when_permission_does_not_match() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let role = make_role("viewer", tenant_id);
    let perm = make_permission("users", "read", tenant_id);

    let role_clone = role.clone();
    let perm_clone = perm.clone();

    let mut roles = MockRoleRepo::new();
    roles
        .expect_get_roles_for_user()
        .once()
        .returning(move |_, _| Ok(vec![role_clone.clone()]));

    let mut perms = MockPermRepo::new();
    perms
        .expect_get_permissions_for_role()
        .once()
        .returning(move |_, _| Ok(vec![perm_clone.clone()]));

    let svc = make_service(roles, perms);
    assert!(!svc.check_permission(user_id, tenant_id, "users", "write").await.unwrap());
}

#[tokio::test]
async fn check_permission_returns_false_when_user_has_no_roles() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let mut roles = MockRoleRepo::new();
    roles
        .expect_get_roles_for_user()
        .once()
        .returning(|_, _| Ok(vec![]));

    let mut perms = MockPermRepo::new();
    perms.expect_get_permissions_for_role().never();

    let svc = make_service(roles, perms);
    assert!(!svc.check_permission(user_id, tenant_id, "users", "read").await.unwrap());
}

#[tokio::test]
async fn check_permission_inherits_parent_role_permissions() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let parent_role = make_role("base", tenant_id);
    let parent_id = parent_role.id;

    let mut child_role = make_role("editor", tenant_id);
    child_role.parent_role_id = Some(parent_id);
    let child_clone = child_role.clone();

    let parent_perm = make_permission("articles", "publish", tenant_id);
    let child_perm = make_permission("articles", "read", tenant_id);

    let parent_role_clone = parent_role.clone();
    let parent_perm_clone = parent_perm.clone();
    let child_perm_clone = child_perm.clone();

    let mut roles = MockRoleRepo::new();
    roles
        .expect_get_roles_for_user()
        .once()
        .returning(move |_, _| Ok(vec![child_clone.clone()]));
    roles
        .expect_get_by_id()
        .withf(move |id, _| *id == parent_id)
        .once()
        .returning(move |_, _| Ok(parent_role_clone.clone()));

    let mut perms = MockPermRepo::new();
    perms
        .expect_get_permissions_for_role()
        .withf(move |id, _| *id == parent_id)
        .once()
        .returning(move |_, _| Ok(vec![parent_perm_clone.clone()]));
    perms
        .expect_get_permissions_for_role()
        .once()
        .returning(move |_, _| Ok(vec![child_perm_clone.clone()]));

    let svc = make_service(roles, perms);

    // Parent's permission is inherited through the child role
    assert!(svc.check_permission(user_id, tenant_id, "articles", "publish").await.unwrap());
}

#[tokio::test]
async fn get_user_permissions_deduplicates_across_roles() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let role_a = make_role("role-a", tenant_id);
    let role_b = make_role("role-b", tenant_id);
    let shared_perm = make_permission("reports", "read", tenant_id);

    let a_clone = role_a.clone();
    let b_clone = role_b.clone();
    let perm1 = shared_perm.clone();
    let perm2 = shared_perm.clone();

    let mut roles = MockRoleRepo::new();
    roles
        .expect_get_roles_for_user()
        .once()
        .returning(move |_, _| Ok(vec![a_clone.clone(), b_clone.clone()]));

    let mut perms = MockPermRepo::new();
    perms
        .expect_get_permissions_for_role()
        .returning(move |_, _| Ok(vec![perm1.clone()]));
    let _ = perm2; // unused — same UUID so dedup kicks in

    let svc = make_service(roles, perms);
    let result = svc.get_user_permissions(user_id, tenant_id).await.unwrap();
    assert_eq!(result.len(), 1, "duplicate permission should be deduplicated");
}

#[tokio::test]
async fn assign_role_delegates_to_repository() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();

    let mut roles = MockRoleRepo::new();
    roles
        .expect_assign_role_to_user()
        .once()
        .returning(|_, _, _| Ok(()));

    let perms = MockPermRepo::new();
    let svc = make_service(roles, perms);
    svc.assign_role(user_id, role_id, tenant_id).await.unwrap();
}

#[tokio::test]
async fn remove_role_delegates_to_repository() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();

    let mut roles = MockRoleRepo::new();
    roles
        .expect_remove_role_from_user()
        .once()
        .returning(|_, _, _| Ok(()));

    let perms = MockPermRepo::new();
    let svc = make_service(roles, perms);
    svc.remove_role(user_id, role_id, tenant_id).await.unwrap();
}

#[tokio::test]
async fn check_permission_propagates_store_error() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let mut roles = MockRoleRepo::new();
    roles
        .expect_get_roles_for_user()
        .once()
        .returning(|_, _| Err(StoreError::Database("connection lost".into())));

    let perms = MockPermRepo::new();
    let svc = make_service(roles, perms);
    let result = svc.check_permission(user_id, tenant_id, "x", "y").await;
    assert!(
        matches!(result, Err(irongate_core::errors::AuthzError::Store(_))),
        "expected Store error, got {result:?}"
    );
}

// ── Scope resolution ──────────────────────────────────────────────────────────

#[test]
fn resolve_scopes_parses_resource_action_pairs() {
    let scopes = vec!["users:read".into(), "reports:write".into()];
    let parsed = resolve_scopes(&scopes);
    assert_eq!(parsed, vec![
        ("users".into(), "read".into()),
        ("reports".into(), "write".into()),
    ]);
}

#[test]
fn resolve_scopes_skips_malformed_entries() {
    let scopes = vec!["openid".into(), "profile".into(), "users:read".into()];
    let parsed = resolve_scopes(&scopes);
    assert_eq!(parsed, vec![("users".into(), "read".into())]);
}

#[test]
fn scopes_grant_matches_exact() {
    let scopes = vec!["users:read".into(), "reports:write".into()];
    assert!(scopes_grant(&scopes, "users", "read"));
    assert!(!scopes_grant(&scopes, "users", "write"));
}

#[test]
fn scopes_grant_matches_wildcard_action() {
    let scopes = vec!["users:*".into()];
    assert!(scopes_grant(&scopes, "users", "read"));
    assert!(scopes_grant(&scopes, "users", "delete"));
    assert!(!scopes_grant(&scopes, "reports", "read"));
}

#[test]
fn scopes_grant_matches_wildcard_resource_and_action() {
    let scopes = vec!["*:*".into()];
    assert!(scopes_grant(&scopes, "users", "read"));
    assert!(scopes_grant(&scopes, "reports", "write"));
}

// ── ABAC policy evaluation ────────────────────────────────────────────────────

#[test]
fn evaluate_policy_returns_true_when_no_conditions() {
    let policy = make_policy("users", "read", PolicyEffect::Allow, vec![]);
    let ctx = EvaluationContext::default();
    assert!(evaluate(&policy, &ctx));
}

#[test]
fn evaluate_policy_user_attribute_eq_matches() {
    let policy = make_policy(
        "reports",
        "read",
        PolicyEffect::Allow,
        vec![Condition::UserAttribute {
            attribute: "department".into(),
            operator: Operator::Eq,
            value: serde_json::json!("engineering"),
        }],
    );

    let mut ctx = EvaluationContext::default();
    ctx.user_attributes
        .insert("department".into(), serde_json::json!("engineering"));

    assert!(evaluate(&policy, &ctx));
}

#[test]
fn evaluate_policy_user_attribute_eq_does_not_match() {
    let policy = make_policy(
        "reports",
        "read",
        PolicyEffect::Allow,
        vec![Condition::UserAttribute {
            attribute: "department".into(),
            operator: Operator::Eq,
            value: serde_json::json!("engineering"),
        }],
    );

    let mut ctx = EvaluationContext::default();
    ctx.user_attributes
        .insert("department".into(), serde_json::json!("marketing"));

    assert!(!evaluate(&policy, &ctx));
}

#[test]
fn evaluate_policy_in_operator_matches() {
    let policy = make_policy(
        "admin",
        "access",
        PolicyEffect::Allow,
        vec![Condition::UserAttribute {
            attribute: "role".into(),
            operator: Operator::In,
            value: serde_json::json!(["admin", "superuser"]),
        }],
    );

    let mut ctx = EvaluationContext::default();
    ctx.user_attributes
        .insert("role".into(), serde_json::json!("admin"));

    assert!(evaluate(&policy, &ctx));
}

#[test]
fn evaluate_policy_time_range_condition() {
    use time::{Date, Month, PrimitiveDateTime, Time};

    let policy = make_policy(
        "reports",
        "export",
        PolicyEffect::Allow,
        vec![Condition::TimeRange { start_hour: 9, end_hour: 17 }],
    );

    // 10:00 UTC — inside business hours
    let inside = PrimitiveDateTime::new(
        Date::from_calendar_date(2026, Month::January, 15).unwrap(),
        Time::from_hms(10, 0, 0).unwrap(),
    )
    .assume_utc();

    let ctx_inside = EvaluationContext {
        user_attributes: HashMap::new(),
        request_time: inside,
    };
    assert!(evaluate(&policy, &ctx_inside));

    // 22:00 UTC — outside business hours
    let outside = PrimitiveDateTime::new(
        Date::from_calendar_date(2026, Month::January, 15).unwrap(),
        Time::from_hms(22, 0, 0).unwrap(),
    )
    .assume_utc();

    let ctx_outside = EvaluationContext {
        user_attributes: HashMap::new(),
        request_time: outside,
    };
    assert!(!evaluate(&policy, &ctx_outside));
}

#[test]
fn policy_allows_returns_true_for_allow_effect() {
    let policy = make_policy("users", "read", PolicyEffect::Allow, vec![]);
    let ctx = EvaluationContext::default();
    assert!(policy_allows(&policy, &ctx));
    assert!(!policy_denies(&policy, &ctx));
}

#[test]
fn policy_denies_returns_true_for_deny_effect() {
    let policy = make_policy("users", "delete", PolicyEffect::Deny, vec![]);
    let ctx = EvaluationContext::default();
    assert!(policy_denies(&policy, &ctx));
    assert!(!policy_allows(&policy, &ctx));
}
