use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    /// Full postgres connection URL (e.g. `postgres://user:pass@localhost/irongate`).
    pub postgres_url: String,

    /// Redis connection URL (e.g. `redis://localhost:6379`).
    pub redis_url: String,

    /// Max connections in the Postgres pool.
    #[serde(default = "defaults::pg_pool_max")]
    pub pg_pool_max: u32,
}

mod defaults {
    pub fn pg_pool_max() -> u32 {
        10
    }
}
