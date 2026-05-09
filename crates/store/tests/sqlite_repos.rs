use irongate_core::{
    repositories::{
        ApplicationRepository, AuditRepository, IdpConfigRepository, IdentityRepository,
        PermissionRepository, RefreshTokenRepository, RoleRepository, TenantRepository,
        UserRepository,
    },
    AppType, AuditEvent, IdpConfig, IdpType, Identity, Permission, RefreshToken, Role, Tenant,
    User, UserStatus,
};
use irongate_store::SqliteStore;
use time::OffsetDateTime;
use uuid::Uuid;

async fn store() -> SqliteStore {
    SqliteStore::new_in_memory().await.expect("in-memory store")
}

// ── Fixtures ──────────────────────────────────────────────────────────────────

fn make_tenant() -> Tenant {
    Tenant {
        id: Uuid::new_v4(),
        name: "Acme Corp".into(),
        slug: format!("acme-{}", Uuid::new_v4()),
        settings: serde_json::json!({}),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        deleted_at: None,
    }
}

fn make_user(tenant_id: Uuid) -> User {
    User {
        id: Uuid::new_v4(),
        tenant_id,
        email: format!("user-{}@example.com", Uuid::new_v4()),
        email_verified: false,
        name: Some("Test User".into()),
        given_name: None,
        family_name: None,
        picture_url: None,
        status: UserStatus::Active,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        last_login_at: None,
        deleted_at: None,
    }
}

fn make_application(tenant_id: Uuid) -> irongate_core::Application {
    irongate_core::Application {
        id: Uuid::new_v4(),
        tenant_id,
        name: format!("app-{}", Uuid::new_v4()),
        client_id: format!("cid-{}", Uuid::new_v4()),
        client_secret_hash: None,
        app_type: AppType::Web,
        redirect_uris: vec!["https://example.com/callback".into()],
        allowed_scopes: vec!["openid".into(), "profile".into()],
        grant_types: vec!["authorization_code".into()],
        access_token_ttl: 3600,
        refresh_token_ttl: 86400,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        deleted_at: None,
    }
}

fn make_identity(user_id: Uuid, tenant_id: Uuid) -> Identity {
    Identity {
        id: Uuid::new_v4(),
        user_id,
        tenant_id,
        provider: "google".into(),
        provider_user_id: format!("gid-{}", Uuid::new_v4()),
        email: format!("federated-{}@gmail.com", Uuid::new_v4()),
        raw_claims: serde_json::json!({"sub": "12345"}),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    }
}

fn make_refresh_token(session_id: Uuid, application_id: Uuid) -> RefreshToken {
    RefreshToken {
        id: Uuid::new_v4(),
        session_id,
        application_id,
        token_hash: format!("hash-{}", Uuid::new_v4()),
        scope: "openid profile".into(),
        previous_id: None,
        created_at: OffsetDateTime::now_utc(),
        expires_at: OffsetDateTime::now_utc() + time::Duration::hours(24),
        revoked_at: None,
    }
}

fn make_role(tenant_id: Uuid) -> Role {
    Role {
        id: Uuid::new_v4(),
        tenant_id,
        name: format!("role-{}", Uuid::new_v4()),
        description: Some("A test role".into()),
        parent_role_id: None,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    }
}

fn make_permission(tenant_id: Uuid) -> Permission {
    Permission {
        id: Uuid::new_v4(),
        tenant_id,
        resource: format!("res-{}", Uuid::new_v4()),
        action: "read".into(),
        description: Some("Read access".into()),
        created_at: OffsetDateTime::now_utc(),
    }
}

fn make_idp_config(tenant_id: Uuid) -> IdpConfig {
    IdpConfig {
        id: Uuid::new_v4(),
        tenant_id,
        provider_type: IdpType::Oidc,
        name: format!("google-{}", Uuid::new_v4()),
        enabled: true,
        config: serde_json::json!({"client_id": "abc", "client_secret": "xyz"}),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    }
}

fn make_audit_event(tenant_id: Uuid) -> AuditEvent {
    AuditEvent {
        id: Uuid::new_v4(),
        tenant_id,
        event_type: "user.login".into(),
        actor_id: None,
        target_id: None,
        ip_address: Some("127.0.0.1".into()),
        metadata: serde_json::json!({}),
        created_at: OffsetDateTime::now_utc(),
    }
}

// ── Tenant tests ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn tenant_create_and_get() {
    let s = store().await;
    let t = make_tenant();
    let created = s.tenants().create(t.clone()).await.expect("create");
    assert_eq!(created.id, t.id);
    assert_eq!(created.slug, t.slug);

    let fetched = s.tenants().get_by_id(t.id).await.expect("get_by_id");
    assert_eq!(fetched.id, t.id);
}

#[tokio::test]
async fn tenant_get_by_slug() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let fetched = s.tenants().get_by_slug(&t.slug).await.expect("get_by_slug");
    assert_eq!(fetched.id, t.id);
}

#[tokio::test]
async fn tenant_duplicate_slug_is_conflict() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let dup = Tenant { id: Uuid::new_v4(), ..t };
    let err = s.tenants().create(dup).await.expect_err("should conflict");
    assert!(matches!(err, irongate_core::errors::StoreError::Conflict(_)));
}

#[tokio::test]
async fn tenant_soft_delete() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    s.tenants().soft_delete(t.id).await.expect("soft_delete");
    let err = s.tenants().get_by_id(t.id).await.expect_err("should be gone");
    assert!(matches!(err, irongate_core::errors::StoreError::NotFound(_)));
}

#[tokio::test]
async fn tenant_list() {
    let s = store().await;
    s.tenants().create(make_tenant()).await.unwrap();
    s.tenants().create(make_tenant()).await.unwrap();
    let list = s.tenants().list(10, 0).await.expect("list");
    assert!(list.len() >= 2);
}

#[tokio::test]
async fn tenant_not_found() {
    let s = store().await;
    let err = s.tenants().get_by_id(Uuid::new_v4()).await.expect_err("not found");
    assert!(matches!(err, irongate_core::errors::StoreError::NotFound(_)));
}

// ── User tests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn user_create_and_get() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();

    let u = make_user(t.id);
    let created = s.users().create(u.clone()).await.expect("create");
    assert_eq!(created.id, u.id);
    assert!(!created.email_verified);

    let fetched = s.users().get_by_id(u.id, t.id).await.expect("get_by_id");
    assert_eq!(fetched.id, u.id);
}

#[tokio::test]
async fn user_get_by_email() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let u = make_user(t.id);
    s.users().create(u.clone()).await.unwrap();

    let fetched = s.users().get_by_email(&u.email, t.id).await.expect("get_by_email");
    assert_eq!(fetched.id, u.id);
}

#[tokio::test]
async fn user_duplicate_email_is_conflict() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let u = make_user(t.id);
    s.users().create(u.clone()).await.unwrap();
    let dup = User { id: Uuid::new_v4(), ..u };
    let err = s.users().create(dup).await.expect_err("conflict");
    assert!(matches!(err, irongate_core::errors::StoreError::Conflict(_)));
}

#[tokio::test]
async fn user_update_email_verified() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let u = make_user(t.id);
    let created = s.users().create(u).await.unwrap();

    let updated = User { email_verified: true, ..created };
    let result = s.users().update(updated).await.expect("update");
    assert!(result.email_verified);
}

#[tokio::test]
async fn user_soft_delete() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let u = make_user(t.id);
    s.users().create(u.clone()).await.unwrap();
    s.users().soft_delete(u.id, t.id).await.expect("soft_delete");
    let err = s.users().get_by_id(u.id, t.id).await.expect_err("should be gone");
    assert!(matches!(err, irongate_core::errors::StoreError::NotFound(_)));
}

// ── Application tests ─────────────────────────────────────────────────────────

#[tokio::test]
async fn application_create_and_get() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let app = make_application(t.id);

    let created = s.applications().create(app.clone()).await.expect("create");
    assert_eq!(created.id, app.id);
    assert_eq!(created.redirect_uris, app.redirect_uris);
    assert_eq!(created.allowed_scopes, app.allowed_scopes);
    assert_eq!(created.grant_types, app.grant_types);

    let by_cid = s
        .applications()
        .get_by_client_id(&app.client_id, t.id)
        .await
        .expect("get_by_client_id");
    assert_eq!(by_cid.id, app.id);
}

#[tokio::test]
async fn application_soft_delete() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let app = make_application(t.id);
    s.applications().create(app.clone()).await.unwrap();
    s.applications().soft_delete(app.id, t.id).await.expect("soft_delete");
    let err = s.applications().get_by_id(app.id, t.id).await.expect_err("gone");
    assert!(matches!(err, irongate_core::errors::StoreError::NotFound(_)));
}

// ── Identity tests ────────────────────────────────────────────────────────────

#[tokio::test]
async fn identity_create_and_get() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let u = make_user(t.id);
    s.users().create(u.clone()).await.unwrap();

    let ident = make_identity(u.id, t.id);
    let created = s.identities().create(ident.clone()).await.expect("create");
    assert_eq!(created.provider_user_id, ident.provider_user_id);
    assert_eq!(created.raw_claims, ident.raw_claims);

    let fetched = s
        .identities()
        .get_by_provider(&ident.provider, &ident.provider_user_id, t.id)
        .await
        .expect("get_by_provider");
    assert_eq!(fetched.id, ident.id);
}

#[tokio::test]
async fn identity_list_for_user() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let u = make_user(t.id);
    s.users().create(u.clone()).await.unwrap();

    s.identities().create(make_identity(u.id, t.id)).await.unwrap();
    s.identities().create(make_identity(u.id, t.id)).await.unwrap();

    let list = s.identities().list_for_user(u.id, t.id).await.expect("list");
    assert_eq!(list.len(), 2);
}

// ── RefreshToken tests ────────────────────────────────────────────────────────

#[tokio::test]
async fn refresh_token_create_and_get() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let app = make_application(t.id);
    s.applications().create(app.clone()).await.unwrap();

    let rt = make_refresh_token(Uuid::new_v4(), app.id);
    let created = s.refresh_tokens().create(rt.clone()).await.expect("create");
    assert_eq!(created.token_hash, rt.token_hash);
    assert!(created.revoked_at.is_none());

    let fetched = s.refresh_tokens().get_by_hash(&rt.token_hash).await.expect("get_by_hash");
    assert_eq!(fetched.id, rt.id);
}

#[tokio::test]
async fn refresh_token_revoke() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let app = make_application(t.id);
    s.applications().create(app.clone()).await.unwrap();

    let rt = make_refresh_token(Uuid::new_v4(), app.id);
    s.refresh_tokens().create(rt.clone()).await.unwrap();
    s.refresh_tokens().revoke(rt.id).await.expect("revoke");

    // get_by_hash filters revoked_at IS NULL, so a revoked token returns NotFound
    let result = s.refresh_tokens().get_by_hash(&rt.token_hash).await;
    assert!(matches!(result, Err(irongate_core::errors::StoreError::NotFound(_))));
}

#[tokio::test]
async fn refresh_token_revoke_all_for_session() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let app = make_application(t.id);
    s.applications().create(app.clone()).await.unwrap();

    let session_id = Uuid::new_v4();
    s.refresh_tokens().create(make_refresh_token(session_id, app.id)).await.unwrap();
    s.refresh_tokens().create(make_refresh_token(session_id, app.id)).await.unwrap();

    let count = s.refresh_tokens().revoke_all_for_session(session_id).await.expect("revoke_all");
    assert_eq!(count, 2);
}

// ── Role tests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn role_create_and_get() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let r = make_role(t.id);

    let created = s.roles().create(r.clone()).await.expect("create");
    assert_eq!(created.id, r.id);
    assert_eq!(created.name, r.name);

    let by_id = s.roles().get_by_id(r.id, t.id).await.expect("get_by_id");
    assert_eq!(by_id.id, r.id);

    let by_name = s.roles().get_by_name(&r.name, t.id).await.expect("get_by_name");
    assert_eq!(by_name.id, r.id);
}

#[tokio::test]
async fn role_assign_and_remove_user() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let u = make_user(t.id);
    s.users().create(u.clone()).await.unwrap();
    let r = make_role(t.id);
    s.roles().create(r.clone()).await.unwrap();

    s.roles().assign_role_to_user(u.id, r.id, t.id).await.expect("assign");
    let roles = s.roles().get_roles_for_user(u.id, t.id).await.expect("get_roles");
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].id, r.id);

    s.roles().remove_role_from_user(u.id, r.id, t.id).await.expect("remove");
    let roles = s.roles().get_roles_for_user(u.id, t.id).await.unwrap();
    assert!(roles.is_empty());
}

#[tokio::test]
async fn role_assign_idempotent() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let u = make_user(t.id);
    s.users().create(u.clone()).await.unwrap();
    let r = make_role(t.id);
    s.roles().create(r.clone()).await.unwrap();

    s.roles().assign_role_to_user(u.id, r.id, t.id).await.unwrap();
    s.roles().assign_role_to_user(u.id, r.id, t.id).await.expect("idempotent second assign");

    let roles = s.roles().get_roles_for_user(u.id, t.id).await.unwrap();
    assert_eq!(roles.len(), 1);
}

// ── Permission tests ──────────────────────────────────────────────────────────

#[tokio::test]
async fn permission_create_and_assign() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let r = make_role(t.id);
    s.roles().create(r.clone()).await.unwrap();
    let p = make_permission(t.id);

    let created = s.permissions().create(p.clone()).await.expect("create");
    assert_eq!(created.id, p.id);

    let by_id = s.permissions().get_by_id(p.id, t.id).await.expect("get_by_id");
    assert_eq!(by_id.resource, p.resource);

    s.permissions().assign_permission_to_role(r.id, p.id, t.id).await.expect("assign");

    let perms = s.permissions().get_permissions_for_role(r.id, t.id).await.expect("get_perms");
    assert_eq!(perms.len(), 1);
    assert_eq!(perms[0].id, p.id);
}

#[tokio::test]
async fn permission_assign_idempotent() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let r = make_role(t.id);
    s.roles().create(r.clone()).await.unwrap();
    let p = make_permission(t.id);
    s.permissions().create(p.clone()).await.unwrap();

    s.permissions().assign_permission_to_role(r.id, p.id, t.id).await.unwrap();
    s.permissions()
        .assign_permission_to_role(r.id, p.id, t.id)
        .await
        .expect("idempotent second assign");

    let perms = s.permissions().get_permissions_for_role(r.id, t.id).await.unwrap();
    assert_eq!(perms.len(), 1);
}

#[tokio::test]
async fn permission_delete() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let p = make_permission(t.id);
    s.permissions().create(p.clone()).await.unwrap();
    s.permissions().delete(p.id, t.id).await.expect("delete");
    let err = s.permissions().get_by_id(p.id, t.id).await.expect_err("gone");
    assert!(matches!(err, irongate_core::errors::StoreError::NotFound(_)));
}

// ── IdpConfig tests ───────────────────────────────────────────────────────────

#[tokio::test]
async fn idp_config_crud() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let cfg = make_idp_config(t.id);

    let created = s.idp_configs().create(cfg.clone()).await.expect("create");
    assert_eq!(created.id, cfg.id);
    assert!(created.enabled);
    assert_eq!(created.config["client_id"], "abc");

    let fetched = s.idp_configs().get_by_id(cfg.id, t.id).await.expect("get_by_id");
    assert_eq!(fetched.provider_type, IdpType::Oidc);

    let updated_cfg = IdpConfig {
        enabled: false,
        config: serde_json::json!({"client_id": "new", "client_secret": "new"}),
        ..fetched
    };
    let updated = s.idp_configs().update(updated_cfg).await.expect("update");
    assert!(!updated.enabled);
    assert_eq!(updated.config["client_id"], "new");

    s.idp_configs().delete(cfg.id, t.id).await.expect("delete");
    let err = s.idp_configs().get_by_id(cfg.id, t.id).await.expect_err("gone");
    assert!(matches!(err, irongate_core::errors::StoreError::NotFound(_)));
}

#[tokio::test]
async fn idp_config_list() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    s.idp_configs().create(make_idp_config(t.id)).await.unwrap();
    s.idp_configs().create(make_idp_config(t.id)).await.unwrap();
    let list = s.idp_configs().list(t.id).await.expect("list");
    assert_eq!(list.len(), 2);
}

// ── Audit tests ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn audit_record_and_list() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();

    s.audit().record(make_audit_event(t.id)).await.expect("record 1");
    s.audit().record(make_audit_event(t.id)).await.expect("record 2");

    let list = s.audit().list(t.id, 10, 0).await.expect("list");
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn audit_with_actor_and_target() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();
    let actor = make_user(t.id);
    s.users().create(actor.clone()).await.unwrap();
    let target = make_user(t.id);
    s.users().create(target.clone()).await.unwrap();

    let event = AuditEvent {
        actor_id: Some(actor.id),
        target_id: Some(target.id),
        metadata: serde_json::json!({"action": "suspend"}),
        ..make_audit_event(t.id)
    };
    s.audit().record(event).await.expect("record");

    let list = s.audit().list(t.id, 10, 0).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].actor_id, Some(actor.id));
    assert_eq!(list[0].target_id, Some(target.id));
}

#[tokio::test]
async fn audit_pagination() {
    let s = store().await;
    let t = make_tenant();
    s.tenants().create(t.clone()).await.unwrap();

    for _ in 0..5 {
        s.audit().record(make_audit_event(t.id)).await.unwrap();
    }

    let page1 = s.audit().list(t.id, 3, 0).await.unwrap();
    let page2 = s.audit().list(t.id, 3, 3).await.unwrap();
    assert_eq!(page1.len(), 3);
    assert_eq!(page2.len(), 2);
    assert_ne!(page1[0].id, page2[0].id);
}

// ── Cross-tenant isolation ────────────────────────────────────────────────────

#[tokio::test]
async fn user_tenant_isolation() {
    let s = store().await;
    let t1 = make_tenant();
    let t2 = make_tenant();
    s.tenants().create(t1.clone()).await.unwrap();
    s.tenants().create(t2.clone()).await.unwrap();

    // Same email, different tenants — both should succeed
    let email = format!("shared-{}@example.com", Uuid::new_v4());
    let u1 = User { email: email.clone(), tenant_id: t1.id, ..make_user(t1.id) };
    let u2 = User { email: email.clone(), tenant_id: t2.id, ..make_user(t2.id) };

    s.users().create(u1.clone()).await.expect("create t1 user");
    s.users().create(u2).await.expect("same email different tenant — no conflict");

    // Cross-tenant lookup returns not found
    let err = s.users().get_by_id(u1.id, t2.id).await.expect_err("cross-tenant");
    assert!(matches!(err, irongate_core::errors::StoreError::NotFound(_)));
}
