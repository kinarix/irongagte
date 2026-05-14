use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    handlers::{
        admin_applications, admin_audit, admin_claims, admin_groups, admin_idp,
        admin_operator_permissions, admin_operator_roles, admin_operators, admin_sessions,
        admin_tenants, admin_users, authorize, health, metrics, oidc, operator, tenants, token,
        users,
    },
    state::AppState,
};

fn admin_router() -> Router<Arc<AppState>> {
    Router::new()
        // Tenants
        .route(
            "/tenants",
            get(admin_tenants::list_tenants).post(admin_tenants::create_tenant),
        )
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
        .route(
            "/users",
            get(admin_users::list_users).post(admin_users::create_user),
        )
        .route(
            "/tenants/{tenant_id}/users/{id}",
            get(admin_users::get_user)
                .put(admin_users::update_user)
                .delete(admin_users::delete_user),
        )
        .route(
            "/tenants/{tenant_id}/users/{id}/suspend",
            post(admin_users::suspend_user),
        )
        .route(
            "/tenants/{tenant_id}/users/{id}/unsuspend",
            post(admin_users::unsuspend_user),
        )
        .route(
            "/tenants/{tenant_id}/users/import",
            post(admin_users::bulk_create_users),
        )
        // Claims: definitions, group assignments, user assignments
        .route(
            "/claims/definitions",
            get(admin_claims::list_claim_definitions).post(admin_claims::create_claim_definition),
        )
        .route(
            "/tenants/{tenant_id}/claims/definitions/{id}",
            get(admin_claims::get_claim_definition)
                .put(admin_claims::update_claim_definition)
                .delete(admin_claims::delete_claim_definition),
        )
        .route(
            "/claims/group-assignments",
            get(admin_claims::list_group_claims)
                .post(admin_claims::assign_group_claim)
                .delete(admin_claims::revoke_group_claim),
        )
        .route(
            "/claims/user-assignments",
            get(admin_claims::list_user_claims)
                .post(admin_claims::assign_user_claim)
                .delete(admin_claims::revoke_user_claim),
        )
        .route(
            "/claims/effective",
            get(admin_claims::preview_effective_claims),
        )
        // Operators (irongate dashboard users)
        .route(
            "/operators",
            get(admin_operators::list_operators).post(admin_operators::create_operator),
        )
        .route(
            "/operators/{id}",
            get(admin_operators::get_operator)
                .put(admin_operators::update_operator)
                .delete(admin_operators::delete_operator),
        )
        .route(
            "/operators/{id}/password",
            post(admin_operators::change_password),
        )
        // Operator role assignments (per operator)
        .route(
            "/operators/{operator_id}/roles",
            get(admin_operator_roles::list_operator_role_assignments),
        )
        .route(
            "/operators/{operator_id}/roles/{role_id}",
            post(admin_operator_roles::assign_role_to_operator)
                .delete(admin_operator_roles::revoke_role_from_operator),
        )
        // Operator roles (system-level RBAC)
        .route(
            "/operator-roles",
            get(admin_operator_roles::list_operator_roles)
                .post(admin_operator_roles::create_operator_role),
        )
        .route(
            "/operator-roles/{id}",
            get(admin_operator_roles::get_operator_role)
                .put(admin_operator_roles::update_operator_role)
                .delete(admin_operator_roles::delete_operator_role),
        )
        .route(
            "/operator-roles/{role_id}/permissions",
            get(admin_operator_roles::list_role_permissions),
        )
        .route(
            "/operator-roles/{role_id}/permissions/{permission_id}",
            post(admin_operator_roles::assign_permission_to_role)
                .delete(admin_operator_roles::revoke_permission_from_role),
        )
        // Operator permission catalog (read-only)
        .route(
            "/operator-permissions",
            get(admin_operator_permissions::list_operator_permissions),
        )
        // Groups
        .route(
            "/groups",
            get(admin_groups::list_groups).post(admin_groups::create_group),
        )
        .route(
            "/tenants/{tenant_id}/groups/{id}",
            get(admin_groups::get_group)
                .put(admin_groups::update_group)
                .delete(admin_groups::delete_group),
        )
        .route(
            "/tenants/{tenant_id}/groups/{id}/members",
            get(admin_groups::list_group_members).post(admin_groups::add_group_member),
        )
        .route(
            "/tenants/{tenant_id}/groups/{group_id}/members/{user_id}",
            delete(admin_groups::remove_group_member),
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
        .route("/metrics", get(metrics::render))
        // OIDC discovery + JWKS
        .route("/.well-known/openid-configuration", get(oidc::discovery))
        .route("/.well-known/jwks.json", get(oidc::jwks))
        // OAuth2 / OIDC
        .route(
            "/oauth2/authorize",
            get(authorize::get_authorize).post(authorize::post_authorize),
        )
        .route("/oauth2/token", post(token::token))
        .route("/oauth2/userinfo", get(oidc::userinfo))
        // Operator (irongate dashboard) auth — distinct from end-user OAuth flow.
        .route("/operator/login", post(operator::login))
        .route("/operator/logout", post(operator::logout))
        .route("/operator/me", get(operator::me))
        // Management API — users
        .route(
            "/api/v1/users",
            get(users::list_users).post(users::create_user),
        )
        .route(
            "/api/v1/users/{id}",
            get(users::get_user).delete(users::delete_user),
        )
        // Management API — tenants
        .route(
            "/api/v1/tenants",
            get(tenants::list_tenants).post(tenants::create_tenant),
        )
        .route("/api/v1/tenants/{id}", get(tenants::get_tenant))
        // Admin API
        .nest("/admin/api/v1", admin_router())
        // Admin UI SPA (served from static/admin/)
        .nest_service(
            "/admin",
            ServeDir::new("static/admin").fallback(ServeFile::new("static/admin/index.html")),
        )
        .with_state(state)
        .layer(cors)
}
