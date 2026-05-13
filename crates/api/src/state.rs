use std::sync::Arc;

use irongate_auth::{PasswordService, SessionService};
use irongate_authz::AuthzService;
use irongate_core::repositories::{
    ApplicationRepository, AuditRepository, AuthCodeStore, GroupRepository, IdpConfigRepository,
    IdentityRepository, PasskeyRepository, PermissionRepository, RefreshTokenRepository,
    RoleRepository, TenantRepository, UserRepository,
};
use irongate_crypto::keys::SigningKeyRecord;

use crate::config::Settings;

pub struct AppState {
    pub config: Arc<Settings>,
    pub users: Arc<dyn UserRepository>,
    pub tenants: Arc<dyn TenantRepository>,
    pub applications: Arc<dyn ApplicationRepository>,
    pub refresh_tokens: Arc<dyn RefreshTokenRepository>,
    pub roles: Arc<dyn RoleRepository>,
    pub permissions: Arc<dyn PermissionRepository>,
    pub groups: Arc<dyn GroupRepository>,
    pub passkeys: Arc<dyn PasskeyRepository>,
    pub identities: Arc<dyn IdentityRepository>,
    pub idp_configs: Arc<dyn IdpConfigRepository>,
    pub audit: Arc<dyn AuditRepository>,
    pub auth_codes: Arc<dyn AuthCodeStore>,
    pub password_svc: Arc<PasswordService>,
    pub session_svc: Arc<SessionService>,
    pub authz_svc: Arc<AuthzService>,
    pub signing_key: Arc<SigningKeyRecord>,
}
