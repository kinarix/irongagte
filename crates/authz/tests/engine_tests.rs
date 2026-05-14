use std::{collections::HashMap, sync::Arc};

use irongate_authz::{
    engine::AuthzService,
    policy::{
        evaluate, policy_allows, policy_denies, AbacPolicy, Condition, EvaluationContext, Operator,
        PolicyEffect,
    },
    scope::{resolve_scopes, scopes_grant},
};
use irongate_core::{
    errors::StoreError,
    repositories::{
        ClaimDefinitionRepository, GroupClaimRepository, ResolvedGroupClaim, ResolvedUserClaim,
        UserClaimRepository,
    },
    types::{ClaimDefinition, ClaimType, GroupClaim, UserClaim},
};
use mockall::mock;
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

// ── Mocks ─────────────────────────────────────────────────────────────────────

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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_def(key: &str, claim_type: ClaimType) -> ClaimDefinition {
    ClaimDefinition {
        id: Uuid::new_v4(),
        application_id: Uuid::new_v4(),
        key: key.into(),
        claim_type,
        description: None,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    }
}

fn make_service(
    defs: MockClaimDefRepo,
    group_claims: MockGroupClaimRepo,
    user_claims: MockUserClaimRepo,
) -> AuthzService {
    AuthzService::new(
        Arc::new(defs),
        Arc::new(group_claims),
        Arc::new(user_claims),
    )
}

fn make_policy(
    resource: &str,
    action: &str,
    effect: PolicyEffect,
    conditions: Vec<Condition>,
) -> AbacPolicy {
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

// ── Claim resolution ──────────────────────────────────────────────────────────

#[tokio::test]
async fn resolve_claims_emits_nothing_when_no_defs() {
    let mut defs = MockClaimDefRepo::new();
    defs.expect_list_for_app().returning(|_| Ok(vec![]));
    let group_claims = MockGroupClaimRepo::new();
    let user_claims = MockUserClaimRepo::new();
    let svc = make_service(defs, group_claims, user_claims);

    let out = svc
        .resolve_claims_for_app(Uuid::new_v4(), Uuid::new_v4(), "billing")
        .await
        .unwrap();
    assert!(out.is_empty());
}

#[tokio::test]
async fn resolve_claims_merges_multi_across_groups_and_user() {
    let def = make_def("roles", ClaimType::Multi);
    let def_id = def.id;
    let app_id = def.application_id;
    let user_id = Uuid::new_v4();

    let mut defs = MockClaimDefRepo::new();
    let def_clone = def.clone();
    defs.expect_list_for_app()
        .returning(move |_| Ok(vec![def_clone.clone()]));

    let mut group_claims = MockGroupClaimRepo::new();
    group_claims
        .expect_list_for_user_in_app()
        .returning(move |_, _| {
            let now = OffsetDateTime::now_utc();
            Ok(vec![
                ResolvedGroupClaim {
                    claim_def_id: def_id,
                    claim_key: "roles".into(),
                    claim_type: ClaimType::Multi,
                    group_id: Uuid::new_v4(),
                    group_priority: 5,
                    group_created_at: now,
                    value: "admin".into(),
                },
                ResolvedGroupClaim {
                    claim_def_id: def_id,
                    claim_key: "roles".into(),
                    claim_type: ClaimType::Multi,
                    group_id: Uuid::new_v4(),
                    group_priority: 1,
                    group_created_at: now,
                    value: "viewer".into(),
                },
            ])
        });

    let mut user_claims = MockUserClaimRepo::new();
    user_claims
        .expect_list_for_user_in_app()
        .returning(move |_, _| {
            Ok(vec![ResolvedUserClaim {
                claim_def_id: def_id,
                claim_key: "roles".into(),
                claim_type: ClaimType::Multi,
                value: "owner".into(),
            }])
        });

    let svc = make_service(defs, group_claims, user_claims);
    let out = svc
        .resolve_claims_for_app(user_id, app_id, "billing")
        .await
        .unwrap();

    let v = out.get("billing:roles").unwrap();
    let mut got: Vec<String> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();
    got.sort();
    assert_eq!(got, vec!["admin", "owner", "viewer"]);
}

#[tokio::test]
async fn resolve_claims_scalar_user_direct_overrides_groups() {
    let def = make_def("plan", ClaimType::Scalar);
    let def_id = def.id;
    let mut defs = MockClaimDefRepo::new();
    let def_clone = def.clone();
    defs.expect_list_for_app()
        .returning(move |_| Ok(vec![def_clone.clone()]));

    let mut group_claims = MockGroupClaimRepo::new();
    group_claims
        .expect_list_for_user_in_app()
        .returning(move |_, _| {
            Ok(vec![ResolvedGroupClaim {
                claim_def_id: def_id,
                claim_key: "plan".into(),
                claim_type: ClaimType::Scalar,
                group_id: Uuid::new_v4(),
                group_priority: 10,
                group_created_at: OffsetDateTime::now_utc(),
                value: "free".into(),
            }])
        });

    let mut user_claims = MockUserClaimRepo::new();
    user_claims
        .expect_list_for_user_in_app()
        .returning(move |_, _| {
            Ok(vec![ResolvedUserClaim {
                claim_def_id: def_id,
                claim_key: "plan".into(),
                claim_type: ClaimType::Scalar,
                value: "enterprise".into(),
            }])
        });

    let svc = make_service(defs, group_claims, user_claims);
    let out = svc
        .resolve_claims_for_app(Uuid::new_v4(), Uuid::new_v4(), "billing")
        .await
        .unwrap();
    assert_eq!(out.get("billing:plan"), Some(&json!("enterprise")));
}

#[tokio::test]
async fn resolve_claims_scalar_highest_priority_group_wins() {
    let def = make_def("plan", ClaimType::Scalar);
    let def_id = def.id;
    let mut defs = MockClaimDefRepo::new();
    let def_clone = def.clone();
    defs.expect_list_for_app()
        .returning(move |_| Ok(vec![def_clone.clone()]));

    // Repo returns rows already ordered by priority DESC, created_at ASC.
    let mut group_claims = MockGroupClaimRepo::new();
    group_claims
        .expect_list_for_user_in_app()
        .returning(move |_, _| {
            Ok(vec![
                ResolvedGroupClaim {
                    claim_def_id: def_id,
                    claim_key: "plan".into(),
                    claim_type: ClaimType::Scalar,
                    group_id: Uuid::new_v4(),
                    group_priority: 100,
                    group_created_at: OffsetDateTime::now_utc(),
                    value: "enterprise".into(),
                },
                ResolvedGroupClaim {
                    claim_def_id: def_id,
                    claim_key: "plan".into(),
                    claim_type: ClaimType::Scalar,
                    group_id: Uuid::new_v4(),
                    group_priority: 5,
                    group_created_at: OffsetDateTime::now_utc(),
                    value: "free".into(),
                },
            ])
        });

    let mut user_claims = MockUserClaimRepo::new();
    user_claims
        .expect_list_for_user_in_app()
        .returning(move |_, _| Ok(vec![]));

    let svc = make_service(defs, group_claims, user_claims);
    let out = svc
        .resolve_claims_for_app(Uuid::new_v4(), Uuid::new_v4(), "billing")
        .await
        .unwrap();
    assert_eq!(out.get("billing:plan"), Some(&json!("enterprise")));
}

#[tokio::test]
async fn resolve_claims_omits_unassigned_claims() {
    let def = make_def("region", ClaimType::Scalar);
    let mut defs = MockClaimDefRepo::new();
    let def_clone = def.clone();
    defs.expect_list_for_app()
        .returning(move |_| Ok(vec![def_clone.clone()]));

    let mut group_claims = MockGroupClaimRepo::new();
    group_claims
        .expect_list_for_user_in_app()
        .returning(move |_, _| Ok(vec![]));
    let mut user_claims = MockUserClaimRepo::new();
    user_claims
        .expect_list_for_user_in_app()
        .returning(move |_, _| Ok(vec![]));

    let svc = make_service(defs, group_claims, user_claims);
    let out = svc
        .resolve_claims_for_app(Uuid::new_v4(), Uuid::new_v4(), "billing")
        .await
        .unwrap();
    assert!(out.is_empty());
}

// ── Scope resolution ──────────────────────────────────────────────────────────

#[test]
fn resolve_scopes_parses_resource_action_pairs() {
    let scopes = vec!["users:read".into(), "reports:write".into()];
    let parsed = resolve_scopes(&scopes);
    assert_eq!(
        parsed,
        vec![
            ("users".into(), "read".into()),
            ("reports".into(), "write".into()),
        ]
    );
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
        vec![Condition::TimeRange {
            start_hour: 9,
            end_hour: 17,
        }],
    );

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
