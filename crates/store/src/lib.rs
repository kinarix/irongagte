pub mod config;
pub(crate) mod util;
pub mod pg;
pub mod session;
pub mod sqlite;

pub use config::StoreConfig;
pub use pg::PgStore;
pub use session::RedisSessionStore;
pub use sqlite::SqliteStore;

pub(crate) static PG_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../../migrations/postgres");

pub(crate) static SQLITE_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../../migrations/sqlite");
