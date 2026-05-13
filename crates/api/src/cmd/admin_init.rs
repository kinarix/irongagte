use std::sync::Arc;

use anyhow::Context;
use irongate_api::config::Settings;
use irongate_auth::{PasswordService, SessionService};
use irongate_authz::AuthzService;
use irongate_core::{
    repositories::{
        ApplicationRepository, AuditRepository, GroupRepository, IdpConfigRepository,
        IdentityRepository, PasskeyRepository, PermissionRepository, RefreshTokenRepository,
        RoleRepository, TenantRepository, UserCredentialsRepository, UserRepository,
    },
    types::{AppType, Application, Permission, Role, Tenant, User, UserStatus},
};
use irongate_store::PgStore;
use time::OffsetDateTime;
use uuid::Uuid;

const SYSTEM_TENANT_ID: Uuid = Uuid::from_u128(1);
const ADMIN_CLIENT_ID: &str = "irongate-admin";

pub async fn run(args: &[String]) -> anyhow::Result<()> {
    let email = flag(args, "--email").context("--email is required")?;
    let password = flag(args, "--password").context("--password is required")?;
    let extra_redirect_uris: Vec<String> = flags(args, "--extra-redirect-uri")
        .into_iter()
        .map(String::from)
        .collect();

    let settings = Settings::load().context("failed to load configuration")?;

    let pg = PgStore::new(&settings.database.url, settings.database.max_connections)
        .await
        .context("failed to connect to database")?;
    pg.migrate().await.context("failed to run migrations")?;

    let users: Arc<dyn UserRepository> = Arc::new(pg.users());
    let tenants: Arc<dyn TenantRepository> = Arc::new(pg.tenants());
    let applications: Arc<dyn ApplicationRepository> = Arc::new(pg.applications());
    let roles: Arc<dyn RoleRepository> = Arc::new(pg.roles());
    let permissions: Arc<dyn PermissionRepository> = Arc::new(pg.permissions());
    let credentials: Arc<dyn UserCredentialsRepository> = Arc::new(pg.user_credentials());
    let refresh_tokens: Arc<dyn RefreshTokenRepository> = Arc::new(pg.refresh_tokens());
    let _groups: Arc<dyn GroupRepository> = Arc::new(pg.groups());
    let _passkeys: Arc<dyn PasskeyRepository> = Arc::new(pg.passkeys());
    let _identities: Arc<dyn IdentityRepository> = Arc::new(pg.identities());
    let _idp_configs: Arc<dyn IdpConfigRepository> = Arc::new(pg.idp_configs());
    let _audit: Arc<dyn AuditRepository> = Arc::new(pg.audit());
    let redis = irongate_store::RedisSessionStore::new(&settings.redis.url)
        .await
        .context("failed to connect to Redis")?;
    let sessions = Arc::new(redis);

    let password_svc = Arc::new(PasswordService::new(users.clone(), credentials));
    let _session_svc = Arc::new(SessionService::new(sessions, refresh_tokens.clone()));
    let authz_svc = Arc::new(AuthzService::new(roles.clone(), permissions.clone()));

    // 1. Upsert system tenant
    let tenant = upsert_system_tenant(tenants.clone()).await?;
    println!("tenant: {} ({})", tenant.slug, tenant.id);

    // 2. Upsert super_admin role with * permission
    let (role, perm) =
        upsert_super_admin_role(roles.clone(), permissions.clone(), tenant.id).await?;
    println!("role: {} ({})", role.name, role.id);
    println!("permission: {}:{} ({})", perm.resource, perm.action, perm.id);

    // 3. Create admin user
    let user = create_admin_user(users.clone(), tenant.id, &email).await?;
    println!("user: {} ({})", user.email, user.id);

    // 4. Set password
    password_svc.set_password(user.id, tenant.id, &password).await?;
    println!("password: set");

    // 5. Assign role
    authz_svc
        .assign_role(user.id, role.id, tenant.id)
        .await
        .context("failed to assign super_admin role")?;
    println!("role assigned");

    // 6. Register admin UI OAuth2 client
    let mut redirect_uris = vec![format!("{}/admin/callback", settings.base_url)];
    redirect_uris.extend(extra_redirect_uris);
    let app = upsert_admin_app(applications.clone(), tenant.id, redirect_uris).await?;
    println!("application: {} (client_id={})", app.name, app.client_id);

    println!("\nAdmin init complete. Login at {}/admin", settings.base_url);
    Ok(())
}

async fn upsert_system_tenant(tenants: Arc<dyn TenantRepository>) -> anyhow::Result<Tenant> {
    if let Ok(t) = tenants.get_by_id(SYSTEM_TENANT_ID).await {
        return Ok(t);
    }
    if let Ok(t) = tenants.get_by_slug("system").await {
        return Ok(t);
    }
    let now = OffsetDateTime::now_utc();
    let tenant = Tenant {
        id: SYSTEM_TENANT_ID,
        name: "System".into(),
        slug: "system".into(),
        settings: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    tenants.create(tenant).await.context("failed to create system tenant")
}

async fn upsert_super_admin_role(
    roles: Arc<dyn RoleRepository>,
    permissions: Arc<dyn PermissionRepository>,
    tenant_id: Uuid,
) -> anyhow::Result<(Role, Permission)> {
    let now = OffsetDateTime::now_utc();

    let role = match roles.get_by_name("super_admin", tenant_id).await {
        Ok(r) => r,
        Err(_) => {
            let r = Role {
                id: Uuid::new_v4(),
                tenant_id,
                name: "super_admin".into(),
                description: Some("Full system access".into()),
                parent_role_id: None,
                created_at: now,
                updated_at: now,
            };
            roles.create(r).await.context("failed to create super_admin role")?
        }
    };

    let existing_perms = permissions.get_permissions_for_role(role.id, tenant_id).await?;
    let perm = if let Some(p) = existing_perms.into_iter().find(|p| p.resource == "*") {
        p
    } else {
        let all_perms = permissions.list(tenant_id).await?;
        let perm = if let Some(p) = all_perms.into_iter().find(|p| p.resource == "*" && p.action == "*") {
            p
        } else {
            let p = Permission {
                id: Uuid::new_v4(),
                tenant_id,
                resource: "*".into(),
                action: "*".into(),
                description: Some("Wildcard — full access".into()),
                created_at: now,
            };
            permissions.create(p).await.context("failed to create wildcard permission")?
        };
        permissions
            .assign_permission_to_role(role.id, perm.id, tenant_id)
            .await
            .context("failed to assign wildcard permission to super_admin")?;
        perm
    };

    Ok((role, perm))
}

async fn create_admin_user(
    users: Arc<dyn UserRepository>,
    tenant_id: Uuid,
    email: &str,
) -> anyhow::Result<User> {
    if let Ok(u) = users.get_by_email(email, tenant_id).await {
        return Ok(u);
    }
    let now = OffsetDateTime::now_utc();
    let user = User {
        id: Uuid::new_v4(),
        tenant_id,
        email: email.into(),
        email_verified: true,
        name: Some("Admin".into()),
        given_name: None,
        family_name: None,
        picture_url: None,
        status: UserStatus::Active,
        created_at: now,
        updated_at: now,
        last_login_at: None,
        deleted_at: None,
    };
    users.create(user).await.context("failed to create admin user")
}

async fn upsert_admin_app(
    applications: Arc<dyn ApplicationRepository>,
    tenant_id: Uuid,
    redirect_uris: Vec<String>,
) -> anyhow::Result<Application> {
    let now = OffsetDateTime::now_utc();
    if let Ok(mut app) = applications.get_by_client_id(ADMIN_CLIENT_ID, tenant_id).await {
        // Merge in any new redirect URIs without duplicates
        for uri in &redirect_uris {
            if !app.redirect_uris.contains(uri) {
                app.redirect_uris.push(uri.clone());
            }
        }
        app.updated_at = now;
        return applications.update(app).await.context("failed to update admin application");
    }
    let app = Application {
        id: Uuid::new_v4(),
        tenant_id,
        name: "Irongate Admin UI".into(),
        client_id: ADMIN_CLIENT_ID.into(),
        client_secret_hash: None,
        app_type: AppType::Spa,
        redirect_uris,
        allowed_scopes: vec!["openid".into(), "profile".into(), "admin:*".into()],
        grant_types: vec!["authorization_code".into()],
        access_token_ttl: 3600,
        refresh_token_ttl: 86400 * 7,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    applications.create(app).await.context("failed to create admin application")
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].as_str())
}

fn flags<'a>(args: &'a [String], name: &str) -> Vec<&'a str> {
    args.windows(2)
        .filter(|w| w[0] == name)
        .map(|w| w[1].as_str())
        .collect()
}
