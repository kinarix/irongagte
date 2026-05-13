use std::sync::Arc;

use irongate_auth::{PasswordService, SessionService};
use irongate_authz::AuthzService;
use irongate_core::repositories::{
    ApplicationRepository, AuditRepository, AuthCodeStore, ClaimDefinitionRepository,
    GroupClaimRepository, GroupRepository, IdentityRepository, IdpConfigRepository,
    OperatorCredentialsRepository, OperatorPermissionRepository, OperatorRepository,
    OperatorRoleRepository, PasskeyRepository, RefreshTokenRepository, TenantRepository,
    UserClaimRepository, UserRepository,
};
use irongate_crypto::keys::SigningKeyRecord;

use crate::config::Settings;

pub struct AppState {
    pub config: Arc<Settings>,
    pub users: Arc<dyn UserRepository>,
    pub tenants: Arc<dyn TenantRepository>,
    pub applications: Arc<dyn ApplicationRepository>,
    pub refresh_tokens: Arc<dyn RefreshTokenRepository>,
    pub groups: Arc<dyn GroupRepository>,
    pub passkeys: Arc<dyn PasskeyRepository>,
    pub identities: Arc<dyn IdentityRepository>,
    pub idp_configs: Arc<dyn IdpConfigRepository>,
    pub audit: Arc<dyn AuditRepository>,
    pub operators: Arc<dyn OperatorRepository>,
    pub operator_credentials: Arc<dyn OperatorCredentialsRepository>,
    pub operator_permissions: Arc<dyn OperatorPermissionRepository>,
    pub operator_roles_repo: Arc<dyn OperatorRoleRepository>,
    pub claim_definitions: Arc<dyn ClaimDefinitionRepository>,
    pub group_claims: Arc<dyn GroupClaimRepository>,
    pub user_claims: Arc<dyn UserClaimRepository>,
    pub auth_codes: Arc<dyn AuthCodeStore>,
    pub password_svc: Arc<PasswordService>,
    pub session_svc: Arc<SessionService>,
    pub authz_svc: Arc<AuthzService>,
    pub signing_key: Arc<SigningKeyRecord>,
}
