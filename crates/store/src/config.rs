use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    /// Full postgres connection URL (e.g. `postgres://user:pass@localhost/irongate`).
    /// Set to `None` when running in SQLite-only standalone mode.
    pub postgres_url: Option<String>,

    /// Path to the SQLite database file (e.g. `./irongate.db`).
    /// Set to `None` when running in Postgres mode.
    pub sqlite_path: Option<String>,

    /// Redis connection URL (e.g. `redis://localhost:6379`).
    pub redis_url: String,

    /// Max connections in the Postgres pool.
    #[serde(default = "defaults::pg_pool_max")]
    pub pg_pool_max: u32,

    /// Max connections in the SQLite pool.
    #[serde(default = "defaults::sqlite_pool_max")]
    pub sqlite_pool_max: u32,
}

mod defaults {
    pub fn pg_pool_max() -> u32 {
        10
    }
    pub fn sqlite_pool_max() -> u32 {
        5
    }
}
