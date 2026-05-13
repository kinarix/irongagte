pub mod config;
pub(crate) mod util;
pub mod pg;
pub mod session;

pub use config::StoreConfig;
pub use pg::PgStore;
pub use session::RedisSessionStore;
pub use irongate_core::repositories::AuthCodeStore;

pub(crate) static PG_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../../migrations/postgres");
