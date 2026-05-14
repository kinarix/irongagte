pub mod application;
pub mod audit;
pub mod claim_def;
pub mod group;
pub mod group_claim;
pub mod identity;
pub mod idp_config;
pub mod magic_link;
pub mod operator;
pub mod operator_permission;
pub mod operator_role;
pub mod passkey;
pub mod refresh_token;
pub mod tenant;
pub mod user;
pub mod user_claim;
pub mod user_credentials;

pub use application::PgApplicationRepo;
pub use audit::PgAuditRepo;
pub use claim_def::PgClaimDefinitionRepo;
pub use group::PgGroupRepo;
pub use group_claim::PgGroupClaimRepo;
pub use identity::PgIdentityRepo;
pub use idp_config::PgIdpConfigRepo;
pub use magic_link::PgMagicLinkRepo;
pub use operator::{PgOperatorCredentialsRepo, PgOperatorRepo};
pub use operator_permission::PgOperatorPermissionRepo;
pub use operator_role::PgOperatorRoleRepo;
pub use passkey::PgPasskeyRepo;
pub use refresh_token::PgRefreshTokenRepo;
pub use tenant::PgTenantRepo;
pub use user::PgUserRepo;
pub use user_claim::PgUserClaimRepo;
pub use user_credentials::PgUserCredentialsRepo;

use irongate_core::errors::StoreError;

pub struct PgStore {
    pool: sqlx::PgPool,
}

impl PgStore {
    pub async fn new(url: &str, max_connections: u32) -> Result<Self, StoreError> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(url)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), StoreError> {
        crate::PG_MIGRATOR
            .run(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))
    }

    pub fn tenants(&self) -> PgTenantRepo {
        PgTenantRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn users(&self) -> PgUserRepo {
        PgUserRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn applications(&self) -> PgApplicationRepo {
        PgApplicationRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn identities(&self) -> PgIdentityRepo {
        PgIdentityRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn refresh_tokens(&self) -> PgRefreshTokenRepo {
        PgRefreshTokenRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn operator_permissions(&self) -> PgOperatorPermissionRepo {
        PgOperatorPermissionRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn operator_roles(&self) -> PgOperatorRoleRepo {
        PgOperatorRoleRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn idp_configs(&self) -> PgIdpConfigRepo {
        PgIdpConfigRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn audit(&self) -> PgAuditRepo {
        PgAuditRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn user_credentials(&self) -> PgUserCredentialsRepo {
        PgUserCredentialsRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn magic_links(&self) -> PgMagicLinkRepo {
        PgMagicLinkRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn passkeys(&self) -> PgPasskeyRepo {
        PgPasskeyRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn groups(&self) -> PgGroupRepo {
        PgGroupRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn operators(&self) -> PgOperatorRepo {
        PgOperatorRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn operator_credentials(&self) -> PgOperatorCredentialsRepo {
        PgOperatorCredentialsRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn claim_definitions(&self) -> PgClaimDefinitionRepo {
        PgClaimDefinitionRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn group_claims(&self) -> PgGroupClaimRepo {
        PgGroupClaimRepo {
            pool: self.pool.clone(),
        }
    }
    pub fn user_claims(&self) -> PgUserClaimRepo {
        PgUserClaimRepo {
            pool: self.pool.clone(),
        }
    }
}
