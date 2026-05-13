use std::sync::Arc;

use anyhow::Context;
use irongate_api::config::Settings;
use irongate_core::{
    repositories::{
        OperatorCredentialsRepository, OperatorPermissionRepository, OperatorRepository,
        OperatorRoleRepository, TenantRepository,
    },
    types::{
        Operator, OperatorCredentials, OperatorPermission, OperatorRole, OperatorStatus, Tenant,
        ALLOWED_OPERATOR_ACTIONS, ALLOWED_OPERATOR_RESOURCES,
    },
};
use irongate_crypto::hash::hash_password;
use irongate_store::PgStore;
use time::OffsetDateTime;
use uuid::Uuid;

/// Bootstrap an Irongate operator (dashboard user) and seed a Default tenant
/// with its permission catalog. Operators are entirely separate from end users
/// and live outside any tenant.
///
/// Usage:
///   irongate admin init --email admin@example.com --password '...'
pub async fn run(args: &[String]) -> anyhow::Result<()> {
    let email = flag(args, "--email").context("--email is required")?;
    let password = flag(args, "--password").context("--password is required")?;

    let settings = Settings::load().context("failed to load configuration")?;

    let pg = PgStore::new(&settings.database.url, settings.database.max_connections)
        .await
        .context("failed to connect to database")?;
    pg.migrate().await.context("failed to run migrations")?;

    let operators: Arc<dyn OperatorRepository> = Arc::new(pg.operators());
    let operator_creds: Arc<dyn OperatorCredentialsRepository> =
        Arc::new(pg.operator_credentials());
    let operator_perms: Arc<dyn OperatorPermissionRepository> =
        Arc::new(pg.operator_permissions());
    let operator_roles: Arc<dyn OperatorRoleRepository> = Arc::new(pg.operator_roles());
    let tenants: Arc<dyn TenantRepository> = Arc::new(pg.tenants());

    let op = upsert_operator(operators.clone(), email).await?;
    println!("operator: {} ({})", op.email, op.id);

    set_operator_password(operator_creds.clone(), op.id, password).await?;
    println!("password: set");

    seed_operator_permission_catalog(operator_perms.clone()).await?;
    println!("operator permission catalog: seeded");

    let super_admin_role =
        upsert_super_admin_role(operator_roles.clone(), operator_perms.clone()).await?;
    println!("super_admin role: {}", super_admin_role.id);

    let viewer_role = upsert_viewer_role(operator_roles.clone(), operator_perms.clone()).await?;
    println!("viewer role: {}", viewer_role.id);

    operator_roles
        .assign_to_operator(op.id, super_admin_role.id)
        .await?;
    println!("operator assigned to super_admin role");

    let default_tenant = upsert_default_tenant(tenants.clone()).await?;
    println!(
        "default tenant: {} ({})",
        default_tenant.slug, default_tenant.id
    );

    println!(
        "\nAdmin init complete. Login at {}/admin (POST /operator/login)",
        settings.base_url
    );
    Ok(())
}

async fn upsert_operator(
    operators: Arc<dyn OperatorRepository>,
    email: &str,
) -> anyhow::Result<Operator> {
    if let Ok(o) = operators.get_by_email(email).await {
        return Ok(o);
    }
    let now = OffsetDateTime::now_utc();
    let op = Operator {
        id: Uuid::new_v4(),
        email: email.to_string(),
        name: Some("Admin".into()),
        status: OperatorStatus::Active,
        created_at: now,
        updated_at: now,
        last_login_at: None,
        deleted_at: None,
    };
    operators.create(op).await.context("failed to create operator")
}

async fn set_operator_password(
    creds: Arc<dyn OperatorCredentialsRepository>,
    operator_id: Uuid,
    password: &str,
) -> anyhow::Result<()> {
    let hash = hash_password(password).map_err(|e| anyhow::anyhow!("hash failed: {e}"))?;
    let now = OffsetDateTime::now_utc();
    match creds.get_by_operator_id(operator_id).await {
        Ok(mut existing) => {
            existing.password_hash = hash;
            existing.updated_at = now;
            creds.update(existing).await.context("failed to update operator password")?;
        }
        Err(_) => {
            let c = OperatorCredentials {
                operator_id,
                password_hash: hash,
                created_at: now,
                updated_at: now,
            };
            creds.create(c).await.context("failed to create operator credentials")?;
        }
    }
    Ok(())
}

async fn upsert_default_tenant(tenants: Arc<dyn TenantRepository>) -> anyhow::Result<Tenant> {
    if let Ok(t) = tenants.get_by_slug("default").await {
        return Ok(t);
    }
    let now = OffsetDateTime::now_utc();
    let t = Tenant {
        id: Uuid::new_v4(),
        name: "Default".into(),
        slug: "default".into(),
        settings: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    tenants.create(t).await.context("failed to create default tenant")
}

async fn seed_operator_permission_catalog(
    permissions: Arc<dyn OperatorPermissionRepository>,
) -> anyhow::Result<()> {
    let now = OffsetDateTime::now_utc();
    let existing: std::collections::HashSet<(String, String)> = permissions
        .list()
        .await
        .context("failed to list existing operator permissions")?
        .into_iter()
        .map(|p| (p.resource, p.action))
        .collect();
    for resource in ALLOWED_OPERATOR_RESOURCES {
        for action in ALLOWED_OPERATOR_ACTIONS {
            if existing.contains(&((*resource).into(), (*action).into())) {
                continue;
            }
            let p = OperatorPermission {
                id: Uuid::new_v4(),
                resource: (*resource).into(),
                action: (*action).into(),
                description: None,
                created_at: now,
            };
            permissions
                .create(p)
                .await
                .with_context(|| format!("failed to create permission {resource}:{action}"))?;
        }
    }
    Ok(())
}

async fn upsert_super_admin_role(
    roles: Arc<dyn OperatorRoleRepository>,
    permissions: Arc<dyn OperatorPermissionRepository>,
) -> anyhow::Result<OperatorRole> {
    let now = OffsetDateTime::now_utc();
    let role = match roles.get_by_name("super_admin", None).await {
        Ok(existing) => existing,
        Err(_) => {
            let r = OperatorRole {
                id: Uuid::new_v4(),
                tenant_id: None,
                name: "super_admin".into(),
                description: Some("Full system access".into()),
                created_at: now,
                updated_at: now,
            };
            roles.create(r).await.context("failed to create super_admin role")?
        }
    };

    // Always reconcile assignments so new permissions added to the catalog
    // (e.g. after adding a resource) are picked up on the next `admin init` run.
    let all_perms = permissions
        .list()
        .await
        .context("failed to list operator permissions")?;
    for perm in all_perms {
        roles
            .assign_permission(role.id, perm.id)
            .await
            .context("failed to assign permission to super_admin role")?;
    }

    Ok(role)
}

async fn upsert_viewer_role(
    roles: Arc<dyn OperatorRoleRepository>,
    permissions: Arc<dyn OperatorPermissionRepository>,
) -> anyhow::Result<OperatorRole> {
    let now = OffsetDateTime::now_utc();
    let role = match roles.get_by_name("viewer", None).await {
        Ok(existing) => existing,
        Err(_) => {
            let r = OperatorRole {
                id: Uuid::new_v4(),
                tenant_id: None,
                name: "viewer".into(),
                description: Some("Read-only access to all resources".into()),
                created_at: now,
                updated_at: now,
            };
            roles.create(r).await.context("failed to create viewer role")?
        }
    };

    let all_perms = permissions
        .list()
        .await
        .context("failed to list operator permissions")?;
    for perm in all_perms {
        if perm.action == "read" || perm.action == "list" {
            roles
                .assign_permission(role.id, perm.id)
                .await
                .context("failed to assign permission to viewer role")?;
        }
    }

    Ok(role)
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].as_str())
}
