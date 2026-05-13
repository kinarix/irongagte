use std::{
    collections::HashSet,
    sync::Arc,
};

use irongate_core::{
    errors::{AuthzError, StoreError},
    repositories::{PermissionRepository, RoleRepository},
    types::{Permission, Role},
};
use uuid::Uuid;

pub struct AuthzService {
    roles: Arc<dyn RoleRepository>,
    permissions: Arc<dyn PermissionRepository>,
}

impl AuthzService {
    pub fn new(
        roles: Arc<dyn RoleRepository>,
        permissions: Arc<dyn PermissionRepository>,
    ) -> Self {
        Self { roles, permissions }
    }

    pub async fn check_permission(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        resource: &str,
        action: &str,
    ) -> Result<bool, AuthzError> {
        let perms = self.get_user_permissions(user_id, tenant_id).await?;
        Ok(perms.iter().any(|p| p.resource == resource && p.action == action))
    }

    pub async fn get_user_permissions(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<Permission>, AuthzError> {
        let direct_roles = self
            .roles
            .get_roles_for_user(user_id, tenant_id)
            .await
            .map_err(AuthzError::Store)?;

        let all_roles = self.expand_role_hierarchy(direct_roles, tenant_id).await?;

        let mut seen: HashSet<Uuid> = HashSet::new();
        let mut permissions: Vec<Permission> = Vec::new();

        for role in all_roles {
            let role_perms = self
                .permissions
                .get_permissions_for_role(role.id, tenant_id)
                .await
                .map_err(AuthzError::Store)?;

            for perm in role_perms {
                if seen.insert(perm.id) {
                    permissions.push(perm);
                }
            }
        }

        Ok(permissions)
    }

    pub async fn get_user_roles(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<Role>, AuthzError> {
        self.roles
            .get_roles_for_user(user_id, tenant_id)
            .await
            .map_err(AuthzError::Store)
    }

    pub async fn assign_role(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), AuthzError> {
        self.roles
            .assign_role_to_user(user_id, role_id, tenant_id)
            .await
            .map_err(AuthzError::Store)
    }

    pub async fn remove_role(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), AuthzError> {
        self.roles
            .remove_role_from_user(user_id, role_id, tenant_id)
            .await
            .map_err(AuthzError::Store)
    }

    /// Expands a set of roles to include all ancestors via parent_role_id.
    async fn expand_role_hierarchy(
        &self,
        initial: Vec<Role>,
        tenant_id: Uuid,
    ) -> Result<Vec<Role>, AuthzError> {
        let mut all: Vec<Role> = Vec::new();
        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut queue = initial;

        while let Some(role) = queue.pop() {
            if !visited.insert(role.id) {
                continue;
            }

            if let Some(parent_id) = role.parent_role_id {
                if !visited.contains(&parent_id) {
                    match self.roles.get_by_id(parent_id, tenant_id).await {
                        Ok(parent) => queue.push(parent),
                        Err(StoreError::NotFound(_)) => {}
                        Err(e) => return Err(AuthzError::Store(e)),
                    }
                }
            }

            all.push(role);
        }

        Ok(all)
    }
}
