use async_trait::async_trait;
use uuid::Uuid;

use crate::errors::StoreError;
use crate::types::{
    Application, AuditEvent, Identity, IdpConfig, MagicLink, Permission, RefreshToken, Role,
    Session, Tenant, User, UserCredentials,
};

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

// ── RoleRepository ────────────────────────────────────────────────────────────

#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn create(&self, role: Role) -> Result<Role, StoreError>;
    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Role, StoreError>;
    async fn get_by_name(&self, name: &str, tenant_id: Uuid) -> Result<Role, StoreError>;
    async fn update(&self, role: Role) -> Result<Role, StoreError>;
    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Role>, StoreError>;
    async fn get_roles_for_user(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<Role>, StoreError>;
    async fn assign_role_to_user(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), StoreError>;
    async fn remove_role_from_user(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), StoreError>;
}

// ── PermissionRepository ──────────────────────────────────────────────────────

#[async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn create(&self, permission: Permission) -> Result<Permission, StoreError>;
    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Permission, StoreError>;
    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Permission>, StoreError>;
    async fn get_permissions_for_role(
        &self,
        role_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<Permission>, StoreError>;
    async fn assign_permission_to_role(
        &self,
        role_id: Uuid,
        permission_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), StoreError>;
    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError>;
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
