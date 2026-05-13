use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub base_url: String,
    pub log: LogConfig,
    pub tokens: TokenConfig,
    pub session: SessionConfig,
    pub smtp: SmtpConfig,
    /// Single SCIM tenant; if absent, SCIM routes are not mounted.
    pub scim_tenant_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenConfig {
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
    pub id_token_ttl_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionConfig {
    pub ttl_seconds: u64,
    pub cookie_name: String,
    pub cookie_secure: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub from: String,
    pub username: String,
    pub password: String,
}

impl Settings {
    pub fn load() -> Result<Self, config::ConfigError> {
        let mut builder = config::Config::builder()
            .add_source(config::File::with_name("config/default"))
            .add_source(config::Environment::default().separator("__"));

        // Support the standard PORT env var (overrides server.port).
        if let Ok(port) = std::env::var("PORT") {
            builder = builder.set_override("server.port", port)?;
        }

        builder.build()?.try_deserialize()
    }
}
