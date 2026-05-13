pub mod application;
pub mod audit;
pub mod group;
pub mod identity;
pub mod idp_config;
pub mod magic_link;
pub mod passkey;
pub mod permission;
pub mod refresh_token;
pub mod role;
pub mod tenant;
pub mod user;
pub mod user_credentials;

pub use application::SqliteApplicationRepo;
pub use audit::SqliteAuditRepo;
pub use group::SqliteGroupRepo;
pub use identity::SqliteIdentityRepo;
pub use idp_config::SqliteIdpConfigRepo;
pub use magic_link::SqliteMagicLinkRepo;
pub use passkey::SqlitePasskeyRepo;
pub use permission::SqlitePermissionRepo;
pub use refresh_token::SqliteRefreshTokenRepo;
pub use role::SqliteRoleRepo;
pub use tenant::SqliteTenantRepo;
pub use user::SqliteUserRepo;
pub use user_credentials::SqliteUserCredentialsRepo;

use irongate_core::errors::StoreError;

pub struct SqliteStore {
    pool: sqlx::SqlitePool,
}

impl SqliteStore {
    pub async fn new(path: &str, max_connections: u32) -> Result<Self, StoreError> {
        let url = format!("sqlite:{path}?mode=rwc");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect(&url)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(Self { pool })
    }

    /// Create an in-memory SQLite store with migrations already applied.
    /// Useful in tests; the database is destroyed when all pool connections close.
    pub async fn new_in_memory() -> Result<Self, StoreError> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .min_connections(1)
            .connect("sqlite::memory:")
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    pub async fn migrate(&self) -> Result<(), StoreError> {
        // Enable FK enforcement for every connection in the pool.
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        crate::SQLITE_MIGRATOR
            .run(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))
    }

    pub fn tenants(&self) -> SqliteTenantRepo {
        SqliteTenantRepo { pool: self.pool.clone() }
    }
    pub fn users(&self) -> SqliteUserRepo {
        SqliteUserRepo { pool: self.pool.clone() }
    }
    pub fn applications(&self) -> SqliteApplicationRepo {
        SqliteApplicationRepo { pool: self.pool.clone() }
    }
    pub fn identities(&self) -> SqliteIdentityRepo {
        SqliteIdentityRepo { pool: self.pool.clone() }
    }
    pub fn refresh_tokens(&self) -> SqliteRefreshTokenRepo {
        SqliteRefreshTokenRepo { pool: self.pool.clone() }
    }
    pub fn roles(&self) -> SqliteRoleRepo {
        SqliteRoleRepo { pool: self.pool.clone() }
    }
    pub fn permissions(&self) -> SqlitePermissionRepo {
        SqlitePermissionRepo { pool: self.pool.clone() }
    }
    pub fn idp_configs(&self) -> SqliteIdpConfigRepo {
        SqliteIdpConfigRepo { pool: self.pool.clone() }
    }
    pub fn audit(&self) -> SqliteAuditRepo {
        SqliteAuditRepo { pool: self.pool.clone() }
    }
    pub fn user_credentials(&self) -> SqliteUserCredentialsRepo {
        SqliteUserCredentialsRepo { pool: self.pool.clone() }
    }
    pub fn magic_links(&self) -> SqliteMagicLinkRepo {
        SqliteMagicLinkRepo { pool: self.pool.clone() }
    }
    pub fn passkeys(&self) -> SqlitePasskeyRepo {
        SqlitePasskeyRepo { pool: self.pool.clone() }
    }
    pub fn groups(&self) -> SqliteGroupRepo {
        SqliteGroupRepo { pool: self.pool.clone() }
    }
}
