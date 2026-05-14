mod cmd;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use arc_swap::ArcSwap;
use irongate_api::{
    config::Settings,
    router::build_router,
    signing_key::{self, RotationPolicy},
    state::AppState,
};
use irongate_auth::{PasswordService, SessionService};
use irongate_authz::AuthzService;
use irongate_core::repositories::AuthCodeStore;
use irongate_core::repositories::{
    ApplicationRepository, AuditRepository, ClaimDefinitionRepository, GroupClaimRepository,
    GroupRepository, IdentityRepository, IdpConfigRepository, OperatorCredentialsRepository,
    OperatorPermissionRepository, OperatorRepository, OperatorRoleRepository, PasskeyRepository,
    RefreshTokenRepository, SigningKeyRepository, TenantRepository, UserClaimRepository,
    UserRepository,
};
use irongate_core::KeyAlgorithm;
use irongate_scim::{groups::GroupState, router::scim_router, users::UserState};
use irongate_store::{PgStore, RedisSessionStore};
use metrics_exporter_prometheus::PrometheusBuilder;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 && args[1] == "admin" && args[2] == "init" {
        return cmd::admin_init::run(&args[3..]).await;
    }
    if args.len() >= 3 && args[1] == "key" && args[2] == "rotate" {
        return cmd::key_rotate::run(&args[3..]).await;
    }

    let settings = Settings::load().context("failed to load configuration")?;

    tracing_subscriber::fmt()
        .with_env_filter(&settings.log.level)
        .init();

    let metrics_handle = PrometheusBuilder::new()
        .install_recorder()
        .context("failed to install prometheus recorder")?;

    let pg = PgStore::new(&settings.database.url, settings.database.max_connections)
        .await
        .context("failed to connect to database")?;
    pg.migrate().await.context("failed to run migrations")?;

    let session_store = RedisSessionStore::new(&settings.redis.url)
        .await
        .context("failed to connect to Redis")?;
    let auth_code_store: Arc<dyn AuthCodeStore> = Arc::new(
        RedisSessionStore::new(&settings.redis.url)
            .await
            .context("failed to connect to Redis (auth codes)")?,
    );

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
    let signing_keys_repo: Arc<dyn SigningKeyRepository> = Arc::new(pg.signing_keys());
    let credentials = Arc::new(pg.user_credentials());
    let sessions = Arc::new(session_store);

    let password_svc = Arc::new(PasswordService::new(users.clone(), credentials));
    let session_svc = Arc::new(SessionService::new(sessions, refresh_tokens.clone()));
    let authz_svc = Arc::new(AuthzService::new(
        claim_definitions.clone(),
        group_claims.clone(),
        user_claims.clone(),
    ));

    let initial_key =
        signing_key::load_or_create(&signing_keys_repo, settings.signing_keys.ttl_days)
            .await
            .context("failed to load or create signing key")?;
    let signing_key = Arc::new(ArcSwap::from_pointee(initial_key));

    // Background rotation: every replica wakes hourly; advisory lock ensures
    // only one performs the actual rotation at a time.
    tokio::spawn(signing_key::rotation_loop(
        signing_keys_repo.clone(),
        RotationPolicy {
            max_age: time::Duration::days(settings.signing_keys.rotation_interval_days),
            expiry_grace: time::Duration::days(settings.signing_keys.rotation_grace_days),
            new_key_ttl: time::Duration::days(settings.signing_keys.ttl_days),
        },
        KeyAlgorithm::Rs256,
        Duration::from_secs(60 * 60),
    ));
    // Hot refresh: pick up rotations performed elsewhere (CLI, peer replica).
    tokio::spawn(signing_key::refresh_loop(
        signing_keys_repo.clone(),
        signing_key.clone(),
        Duration::from_secs(settings.signing_keys.refresh_interval_seconds),
    ));

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
        signing_keys: signing_keys_repo,
        metrics: metrics_handle,
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
    // ConnectInfo<SocketAddr> is needed by tower_governor's
    // PeerIpKeyExtractor — it reads the peer IP from the request extensions.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("server error")?;

    Ok(())
}
