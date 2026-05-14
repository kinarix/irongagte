pub mod config;
pub mod pg;
pub mod session;
pub(crate) mod util;

pub use config::StoreConfig;
pub use irongate_core::repositories::AuthCodeStore;
pub use pg::PgStore;
pub use session::RedisSessionStore;

pub(crate) static PG_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../../migrations/postgres");
