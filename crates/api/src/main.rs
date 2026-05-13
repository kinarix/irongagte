mod cmd;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use irongate_api::{config::Settings, router::build_router, state::AppState};
use irongate_auth::{PasswordService, SessionService};
use irongate_authz::AuthzService;
use irongate_core::repositories::{
    ApplicationRepository, AuditRepository, ClaimDefinitionRepository, GroupClaimRepository,
    GroupRepository, IdentityRepository, IdpConfigRepository, OperatorCredentialsRepository,
    OperatorPermissionRepository, OperatorRepository, OperatorRoleRepository, PasskeyRepository,
    RefreshTokenRepository, TenantRepository, UserClaimRepository, UserRepository,
};
use irongate_crypto::keys::generate_rsa_key;
use irongate_scim::{groups::GroupState, router::scim_router, users::UserState};
use irongate_core::repositories::AuthCodeStore;
use irongate_store::{PgStore, RedisSessionStore};
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 && args[1] == "admin" && args[2] == "init" {
        return cmd::admin_init::run(&args[3..]).await;
    }

    let settings = Settings::load().context("failed to load configuration")?;

    tracing_subscriber::fmt()
        .with_env_filter(&settings.log.level)
        .init();

    let pg = PgStore::new(&settings.database.url, settings.database.max_connections)
        .await
        .context("failed to connect to database")?;
    pg.migrate().await.context("failed to run migrations")?;

    let session_store = RedisSessionStore::new(&settings.redis.url)
        .await
        .context("failed to connect to Redis")?;
    let auth_code_store: Arc<dyn AuthCodeStore> =
        Arc::new(RedisSessionStore::new(&settings.redis.url).await.context("failed to connect to Redis (auth codes)")?);

    let users: Arc<dyn UserRepository> = Arc::new(pg.users());
    let tenants: Arc<dyn TenantRepository> = Arc::new(pg.tenants());
    let applications: Arc<dyn ApplicationRepository> = Arc::new(pg.applications());
    let refresh_tokens: Arc<dyn RefreshTokenRepository> = Arc::new(pg.refresh_tokens());
    let groups: Arc<dyn GroupRepository> = Arc::new(pg.groups());
    let claim_definitions: Arc<dyn ClaimDefinitionRepository> = Arc::new(pg.claim_definitions());
    let group_claims: Arc<dyn GroupClaimRepository> = Arc::new(pg.group_claims());
    let user_claims: Arc<dyn UserClaimRepository> = Arc::new(pg.user_claims());
    let passkeys: Arc<dyn PasskeyRepository> = Arc::new(pg.passkeys());
    let identities: Arc<dyn IdentityRepository> = Arc::new(pg.identities());
    let idp_configs: Arc<dyn IdpConfigRepository> = Arc::new(pg.idp_configs());
    let audit: Arc<dyn AuditRepository> = Arc::new(pg.audit());
    let operators: Arc<dyn OperatorRepository> = Arc::new(pg.operators());
    let operator_credentials: Arc<dyn OperatorCredentialsRepository> =
        Arc::new(pg.operator_credentials());
    let operator_permissions: Arc<dyn OperatorPermissionRepository> =
        Arc::new(pg.operator_permissions());
    let operator_roles: Arc<dyn OperatorRoleRepository> = Arc::new(pg.operator_roles());
    let credentials = Arc::new(pg.user_credentials());
    let sessions = Arc::new(session_store);

    let password_svc = Arc::new(PasswordService::new(users.clone(), credentials));
    let session_svc = Arc::new(SessionService::new(sessions, refresh_tokens.clone()));
    let authz_svc = Arc::new(AuthzService::new(
        claim_definitions.clone(),
        group_claims.clone(),
        user_claims.clone(),
    ));

    let signing_key = Arc::new(
        generate_rsa_key(Uuid::nil(), 365).context("failed to generate signing key")?,
    );

    let config = Arc::new(settings.clone());

    let state = Arc::new(AppState {
        config,
        users: users.clone(),
        tenants: tenants.clone(),
        applications,
        refresh_tokens,
        groups: groups.clone(),
        passkeys,
        identities,
        idp_configs,
        audit,
        operators,
        operator_credentials,
        operator_permissions,
        operator_roles_repo: operator_roles,
        claim_definitions,
        group_claims,
        user_claims,
        auth_codes: auth_code_store,
        password_svc,
        session_svc,
        authz_svc,
        signing_key,
    });

    let mut app = build_router(state);

    if let Some(scim_tenant_id) = settings.scim_tenant_id {
        let user_state = Arc::new(UserState {
            users: users.clone(),
            groups: groups.clone(),
            base_url: settings.base_url.clone(),
            tenant_id: scim_tenant_id,
        });

        let group_state = Arc::new(GroupState {
            groups,
            users,
            base_url: settings.base_url.clone(),
            tenant_id: scim_tenant_id,
        });

        app = app.merge(axum::Router::new().nest("/scim/v2", scim_router(user_state, group_state)));
    }

    let addr: SocketAddr = format!("{}:{}", settings.server.host, settings.server.port)
        .parse()
        .context("invalid server address")?;

    tracing::info!("irongate listening on {addr}");

    let listener = TcpListener::bind(addr).await.context("failed to bind")?;
    axum::serve(listener, app).await.context("server error")?;

    Ok(())
}
