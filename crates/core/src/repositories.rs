use async_trait::async_trait;
use uuid::Uuid;

use crate::errors::StoreError;
use crate::types::{
    Application, AuditEvent, ClaimDefinition, ClaimType, Group, GroupClaim, Identity, IdpConfig,
    MagicLink, Operator, OperatorCredentials, OperatorPermission, OperatorRole, PasskeyCredential,
    RefreshToken, Session, Tenant, User, UserClaim, UserCredentials,
};
use time::OffsetDateTime;

// ── UserRepository ────────────────────────────────────────────────────────────

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: User) -> Result<User, StoreError>;
    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<User, StoreError>;
    async fn get_by_email(&self, email: &str, tenant_id: Uuid) -> Result<User, StoreError>;
    async fn update(&self, user: User) -> Result<User, StoreError>;
    async fn soft_delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    async fn list(&self, tenant_id: Uuid, limit: i64, offset: i64)
        -> Result<Vec<User>, StoreError>;
}

// ── TenantRepository ──────────────────────────────────────────────────────────

#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn create(&self, tenant: Tenant) -> Result<Tenant, StoreError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Tenant, StoreError>;
    async fn get_by_slug(&self, slug: &str) -> Result<Tenant, StoreError>;
    async fn update(&self, tenant: Tenant) -> Result<Tenant, StoreError>;
    async fn soft_delete(&self, id: Uuid) -> Result<(), StoreError>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Tenant>, StoreError>;
}

// ── ApplicationRepository ─────────────────────────────────────────────────────

#[async_trait]
pub trait ApplicationRepository: Send + Sync {
    async fn create(&self, app: Application) -> Result<Application, StoreError>;
    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Application, StoreError>;
    async fn get_by_client_id(
        &self,
        client_id: &str,
        tenant_id: Uuid,
    ) -> Result<Application, StoreError>;
    async fn update(&self, app: Application) -> Result<Application, StoreError>;
    async fn soft_delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    async fn list(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Application>, StoreError>;
}

// ── SessionRepository ─────────────────────────────────────────────────────────

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: Session) -> Result<Session, StoreError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Session, StoreError>;
    async fn revoke(&self, id: Uuid) -> Result<(), StoreError>;
    async fn revoke_all_for_user(&self, user_id: Uuid, tenant_id: Uuid) -> Result<u64, StoreError>;
    async fn list_for_user(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<Session>, StoreError>;
}

// ── IdentityRepository ────────────────────────────────────────────────────────

#[async_trait]
pub trait IdentityRepository: Send + Sync {
    async fn create(&self, identity: Identity) -> Result<Identity, StoreError>;
    async fn get_by_provider(
        &self,
        provider: &str,
        provider_user_id: &str,
        tenant_id: Uuid,
    ) -> Result<Identity, StoreError>;
    async fn list_for_user(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<Identity>, StoreError>;
    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
}

// ── RefreshTokenRepository ────────────────────────────────────────────────────

#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    async fn create(&self, token: RefreshToken) -> Result<RefreshToken, StoreError>;
    async fn get_by_hash(&self, token_hash: &str) -> Result<RefreshToken, StoreError>;
    async fn revoke(&self, id: Uuid) -> Result<(), StoreError>;
    async fn revoke_all_for_session(&self, session_id: Uuid) -> Result<u64, StoreError>;
}

// ── Claim repositories ────────────────────────────────────────────────────────

/// One row of group-derived claim data for a specific user, resolved across
/// `group_members → groups → group_claims → claim_definitions`. Returned by
/// `GroupClaimRepository::list_for_user_in_app`. The caller uses
/// `(group_priority, group_created_at)` to break ties for scalar claims.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedGroupClaim {
    pub claim_def_id: Uuid,
    pub claim_key: String,
    pub claim_type: ClaimType,
    pub group_id: Uuid,
    pub group_priority: i32,
    pub group_created_at: OffsetDateTime,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedUserClaim {
    pub claim_def_id: Uuid,
    pub claim_key: String,
    pub claim_type: ClaimType,
    pub value: String,
}

#[async_trait]
pub trait ClaimDefinitionRepository: Send + Sync {
    async fn create(&self, def: ClaimDefinition) -> Result<ClaimDefinition, StoreError>;
    async fn get_by_id(&self, id: Uuid) -> Result<ClaimDefinition, StoreError>;
    async fn get_by_app_and_key(
        &self,
        application_id: Uuid,
        key: &str,
    ) -> Result<ClaimDefinition, StoreError>;
    async fn list_for_app(&self, application_id: Uuid) -> Result<Vec<ClaimDefinition>, StoreError>;
    /// Every claim def for every application in the tenant. Used by the global
    /// Claims management page.
    async fn list_for_tenant(&self, tenant_id: Uuid) -> Result<Vec<ClaimDefinition>, StoreError>;
    async fn update(&self, def: ClaimDefinition) -> Result<ClaimDefinition, StoreError>;
    async fn delete(&self, id: Uuid) -> Result<(), StoreError>;
}

#[async_trait]
pub trait GroupClaimRepository: Send + Sync {
    async fn assign(
        &self,
        group_id: Uuid,
        claim_def_id: Uuid,
        value: &str,
    ) -> Result<GroupClaim, StoreError>;
    async fn revoke(
        &self,
        group_id: Uuid,
        claim_def_id: Uuid,
        value: &str,
    ) -> Result<(), StoreError>;
    async fn list_for_group(&self, group_id: Uuid) -> Result<Vec<GroupClaim>, StoreError>;
    async fn list_for_claim_def(&self, claim_def_id: Uuid) -> Result<Vec<GroupClaim>, StoreError>;
    /// Returns every group-claim row that flows into a token minted for `user`
    /// against `application`. Joins through `group_members → groups →
    /// group_claims → claim_definitions` and filters claim defs to the app.
    async fn list_for_user_in_app(
        &self,
        user_id: Uuid,
        application_id: Uuid,
    ) -> Result<Vec<ResolvedGroupClaim>, StoreError>;
}

#[async_trait]
pub trait UserClaimRepository: Send + Sync {
    async fn assign(
        &self,
        user_id: Uuid,
        claim_def_id: Uuid,
        value: &str,
    ) -> Result<UserClaim, StoreError>;
    async fn revoke(
        &self,
        user_id: Uuid,
        claim_def_id: Uuid,
        value: &str,
    ) -> Result<(), StoreError>;
    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<UserClaim>, StoreError>;
    async fn list_for_user_in_app(
        &self,
        user_id: Uuid,
        application_id: Uuid,
    ) -> Result<Vec<ResolvedUserClaim>, StoreError>;
}

// ── OperatorPermissionRepository ──────────────────────────────────────────────

#[async_trait]
pub trait OperatorPermissionRepository: Send + Sync {
    async fn create(&self, p: OperatorPermission) -> Result<OperatorPermission, StoreError>;
    async fn list(&self) -> Result<Vec<OperatorPermission>, StoreError>;
    async fn get_by_id(&self, id: Uuid) -> Result<OperatorPermission, StoreError>;
    async fn get_by_resource_action(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<OperatorPermission, StoreError>;
}

// ── OperatorRoleRepository ────────────────────────────────────────────────────

#[async_trait]
pub trait OperatorRoleRepository: Send + Sync {
    async fn create(&self, r: OperatorRole) -> Result<OperatorRole, StoreError>;
    async fn get_by_id(&self, id: Uuid) -> Result<OperatorRole, StoreError>;
    /// Look up a role by name within a scope. `tenant_id = None` returns the
    /// global role with that name; `Some(id)` returns the role scoped to that tenant.
    async fn get_by_name(
        &self,
        name: &str,
        tenant_id: Option<Uuid>,
    ) -> Result<OperatorRole, StoreError>;
    /// List roles. `None` returns all roles (global + every tenant); pass a
    /// scope filter to narrow.
    async fn list(&self, scope: OperatorRoleScope) -> Result<Vec<OperatorRole>, StoreError>;
    async fn update(&self, r: OperatorRole) -> Result<OperatorRole, StoreError>;
    async fn delete(&self, id: Uuid) -> Result<(), StoreError>;
    async fn assign_permission(&self, role_id: Uuid, permission_id: Uuid)
        -> Result<(), StoreError>;
    async fn revoke_permission(&self, role_id: Uuid, permission_id: Uuid)
        -> Result<(), StoreError>;
    async fn list_permissions(&self, role_id: Uuid) -> Result<Vec<OperatorPermission>, StoreError>;
    async fn assign_to_operator(&self, operator_id: Uuid, role_id: Uuid) -> Result<(), StoreError>;
    async fn revoke_from_operator(
        &self,
        operator_id: Uuid,
        role_id: Uuid,
    ) -> Result<(), StoreError>;
    async fn list_for_operator(&self, operator_id: Uuid) -> Result<Vec<OperatorRole>, StoreError>;
    /// All permissions an operator effectively holds across every role assignment.
    /// Returns the union of permissions from global roles and tenant-scoped roles
    /// (regardless of tenant). Callers that need to authorize a specific tenant
    /// should use `list_permissions_for_operator_in_tenant`.
    async fn list_permissions_for_operator(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<OperatorPermission>, StoreError>;
    /// Permissions an operator effectively holds for actions targeting `tenant_id`:
    /// the union of permissions granted by their global roles and by their roles
    /// scoped to that tenant.
    async fn list_permissions_for_operator_in_tenant(
        &self,
        operator_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<OperatorPermission>, StoreError>;
    /// Permissions an operator effectively holds for global (cross-tenant) actions,
    /// i.e. only those granted by roles with `tenant_id IS NULL`.
    async fn list_permissions_for_operator_global(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<OperatorPermission>, StoreError>;
}

/// Scope filter for listing operator roles.
#[derive(Debug, Clone, Copy)]
pub enum OperatorRoleScope {
    /// Every role (global + every tenant).
    All,
    /// Only global (cross-tenant) roles.
    Global,
    /// Only roles scoped to the given tenant.
    Tenant(Uuid),
}

// ── IdpConfigRepository ───────────────────────────────────────────────────────

#[async_trait]
pub trait IdpConfigRepository: Send + Sync {
    async fn create(&self, config: IdpConfig) -> Result<IdpConfig, StoreError>;
    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<IdpConfig, StoreError>;
    async fn list(&self, tenant_id: Uuid) -> Result<Vec<IdpConfig>, StoreError>;
    async fn update(&self, config: IdpConfig) -> Result<IdpConfig, StoreError>;
    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
}

// ── AuditRepository ───────────────────────────────────────────────────────────

#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn record(&self, event: AuditEvent) -> Result<(), StoreError>;
    async fn list(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditEvent>, StoreError>;
}

// ── UserCredentialsRepository ─────────────────────────────────────────────────

// ── OperatorRepository ────────────────────────────────────────────────────────

#[async_trait]
pub trait OperatorRepository: Send + Sync {
    async fn create(&self, operator: Operator) -> Result<Operator, StoreError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Operator, StoreError>;
    async fn get_by_email(&self, email: &str) -> Result<Operator, StoreError>;
    async fn update(&self, operator: Operator) -> Result<Operator, StoreError>;
    async fn soft_delete(&self, id: Uuid) -> Result<(), StoreError>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Operator>, StoreError>;
    async fn touch_last_login(&self, id: Uuid) -> Result<(), StoreError>;
}

// ── OperatorCredentialsRepository ─────────────────────────────────────────────

#[async_trait]
pub trait OperatorCredentialsRepository: Send + Sync {
    async fn create(&self, creds: OperatorCredentials) -> Result<OperatorCredentials, StoreError>;
    async fn get_by_operator_id(
        &self,
        operator_id: Uuid,
    ) -> Result<OperatorCredentials, StoreError>;
    async fn update(&self, creds: OperatorCredentials) -> Result<OperatorCredentials, StoreError>;
    async fn delete(&self, operator_id: Uuid) -> Result<(), StoreError>;
}

#[async_trait]
pub trait UserCredentialsRepository: Send + Sync {
    async fn create(&self, creds: UserCredentials) -> Result<UserCredentials, StoreError>;
    async fn get_by_user_id(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<UserCredentials, StoreError>;
    async fn update(&self, creds: UserCredentials) -> Result<UserCredentials, StoreError>;
    async fn delete(&self, user_id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
}

// ── MagicLinkRepository ───────────────────────────────────────────────────────

#[async_trait]
pub trait MagicLinkRepository: Send + Sync {
    async fn create(&self, link: MagicLink) -> Result<MagicLink, StoreError>;
    async fn get_by_token_hash(
        &self,
        token_hash: &str,
        tenant_id: Uuid,
    ) -> Result<MagicLink, StoreError>;
    async fn mark_used(&self, id: Uuid) -> Result<(), StoreError>;
    async fn delete_expired(&self, tenant_id: Uuid) -> Result<u64, StoreError>;
}

// ── GroupRepository ───────────────────────────────────────────────────────────

#[async_trait]
pub trait GroupRepository: Send + Sync {
    async fn create(&self, group: Group) -> Result<Group, StoreError>;
    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Group, StoreError>;
    async fn get_by_display_name(
        &self,
        display_name: &str,
        tenant_id: Uuid,
    ) -> Result<Group, StoreError>;
    async fn update(&self, group: Group) -> Result<Group, StoreError>;
    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    async fn list(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Group>, StoreError>;
    async fn add_member(
        &self,
        group_id: Uuid,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), StoreError>;
    async fn remove_member(
        &self,
        group_id: Uuid,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), StoreError>;
    async fn list_members(&self, group_id: Uuid, tenant_id: Uuid) -> Result<Vec<User>, StoreError>;
    async fn list_for_user(&self, user_id: Uuid, tenant_id: Uuid)
        -> Result<Vec<Group>, StoreError>;
}

// ── AuthCodeStore ─────────────────────────────────────────────────────────────

/// Data stored alongside a short-lived authorization code.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthCodeData {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub user_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

#[async_trait]
pub trait AuthCodeStore: Send + Sync {
    /// Store a code for `ttl_secs` seconds. Replaces any existing entry.
    async fn store_code(
        &self,
        code: &str,
        data: AuthCodeData,
        ttl_secs: i64,
    ) -> Result<(), StoreError>;

    /// Atomically retrieve-and-delete the code. Returns `None` if not found or expired.
    async fn take_code(&self, code: &str) -> Result<Option<AuthCodeData>, StoreError>;
}

// ── PasskeyRepository ─────────────────────────────────────────────────────────

#[async_trait]
pub trait PasskeyRepository: Send + Sync {
    async fn create(&self, cred: PasskeyCredential) -> Result<PasskeyCredential, StoreError>;
    async fn get_by_credential_id(
        &self,
        credential_id: &str,
        tenant_id: Uuid,
    ) -> Result<PasskeyCredential, StoreError>;
    async fn list_for_user(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<PasskeyCredential>, StoreError>;
    async fn update(&self, cred: PasskeyCredential) -> Result<PasskeyCredential, StoreError>;
    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
}
