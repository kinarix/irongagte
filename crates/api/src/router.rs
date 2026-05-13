use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    handlers::{
        admin_applications, admin_audit, admin_idp, admin_permissions, admin_roles,
        admin_sessions, admin_tenants, admin_users, authorize, health, oidc, tenants, token, users,
    },
    state::AppState,
};

fn admin_router() -> Router<Arc<AppState>> {
    Router::new()
        // Tenants
        .route("/tenants", get(admin_tenants::list_tenants).post(admin_tenants::create_tenant))
        .route(
            "/tenants/{id}",
            get(admin_tenants::get_tenant)
                .put(admin_tenants::update_tenant)
                .delete(admin_tenants::delete_tenant),
        )
        // Applications
        .route(
            "/applications",
            get(admin_applications::list_applications).post(admin_applications::create_application),
        )
        .route(
            "/tenants/{tenant_id}/applications/{id}",
            get(admin_applications::get_application)
                .put(admin_applications::update_application)
                .delete(admin_applications::delete_application),
        )
        // Users
        .route("/users", get(admin_users::list_users).post(admin_users::create_user))
        .route(
            "/tenants/{tenant_id}/users/{id}",
            get(admin_users::get_user)
                .put(admin_users::update_user)
                .delete(admin_users::delete_user),
        )
        .route("/tenants/{tenant_id}/users/{id}/suspend", post(admin_users::suspend_user))
        .route("/tenants/{tenant_id}/users/{id}/unsuspend", post(admin_users::unsuspend_user))
        .route(
            "/tenants/{tenant_id}/users/{id}/roles",
            get(admin_users::list_user_roles).post(admin_users::assign_role),
        )
        .route(
            "/tenants/{tenant_id}/users/{user_id}/roles/{role_id}",
            delete(admin_users::remove_role),
        )
        // Roles
        .route("/roles", get(admin_roles::list_roles).post(admin_roles::create_role))
        .route(
            "/tenants/{tenant_id}/roles/{id}",
            get(admin_roles::get_role)
                .put(admin_roles::update_role)
                .delete(admin_roles::delete_role),
        )
        .route(
            "/tenants/{tenant_id}/roles/{id}/permissions",
            get(admin_roles::list_role_permissions).post(admin_roles::assign_permission),
        )
        .route(
            "/tenants/{tenant_id}/roles/{role_id}/permissions/{perm_id}",
            delete(admin_roles::remove_permission),
        )
        // Permissions
        .route(
            "/permissions",
            get(admin_permissions::list_permissions).post(admin_permissions::create_permission),
        )
        .route(
            "/tenants/{tenant_id}/permissions/{id}",
            delete(admin_permissions::delete_permission),
        )
        // IdP configs
        .route(
            "/idp-configs",
            get(admin_idp::list_idp_configs).post(admin_idp::create_idp_config),
        )
        .route(
            "/tenants/{tenant_id}/idp-configs/{id}",
            get(admin_idp::get_idp_config)
                .put(admin_idp::update_idp_config)
                .delete(admin_idp::delete_idp_config),
        )
        // Sessions
        .route("/sessions", get(admin_sessions::list_sessions))
        .route("/sessions/{id}", delete(admin_sessions::delete_session))
        // Audit log
        .route("/audit", get(admin_audit::list_audit_events))
}

pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health::health))
        // OIDC discovery + JWKS
        .route("/.well-known/openid-configuration", get(oidc::discovery))
        .route("/.well-known/jwks.json", get(oidc::jwks))
        // OAuth2 / OIDC
        .route("/oauth2/authorize", get(authorize::get_authorize).post(authorize::post_authorize))
        .route("/oauth2/token", post(token::token))
        .route("/oauth2/userinfo", get(oidc::userinfo))
        // Management API — users
        .route("/api/v1/users", get(users::list_users).post(users::create_user))
        .route(
            "/api/v1/users/{id}",
            get(users::get_user).delete(users::delete_user),
        )
        // Management API — tenants
        .route("/api/v1/tenants", get(tenants::list_tenants).post(tenants::create_tenant))
        .route("/api/v1/tenants/{id}", get(tenants::get_tenant))
        // Admin API
        .nest("/admin/api/v1", admin_router())
        // Admin UI SPA (served from static/admin/)
        .nest_service(
            "/admin",
            ServeDir::new("static/admin")
                .fallback(ServeFile::new("static/admin/index.html")),
        )
        .with_state(state)
        .layer(cors)
}
