use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

// ── Tenant ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub settings: serde_json::Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

// ── User ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Suspended,
    Pending,
}

impl std::fmt::Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Suspended => write!(f, "suspended"),
            Self::Pending => write!(f, "pending"),
        }
    }
}

impl std::str::FromStr for UserStatus {
    type Err = crate::errors::CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "pending" => Ok(Self::Pending),
            other => Err(crate::errors::CoreError::Validation(format!(
                "unknown user status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture_url: Option<String>,
    pub status: UserStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub last_login_at: Option<OffsetDateTime>,
    pub deleted_at: Option<OffsetDateTime>,
}

// ── Identity (federated) ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Identity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub email: String,
    pub raw_claims: serde_json::Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Returned by every `IdentityProvider::exchange_callback` implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedIdentity {
    pub provider_user_id: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub raw_claims: serde_json::Value,
}

// ── Application ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AppType {
    Web,
    Spa,
    Native,
    Machine,
}

impl std::fmt::Display for AppType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Web => write!(f, "web"),
            Self::Spa => write!(f, "spa"),
            Self::Native => write!(f, "native"),
            Self::Machine => write!(f, "machine"),
        }
    }
}

impl std::str::FromStr for AppType {
    type Err = crate::errors::CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "web" => Ok(Self::Web),
            "spa" => Ok(Self::Spa),
            "native" => Ok(Self::Native),
            "machine" => Ok(Self::Machine),
            other => Err(crate::errors::CoreError::Validation(format!(
                "unknown app type: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Application {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub client_id: String,
    pub client_secret_hash: Option<String>,
    pub app_type: AppType,
    pub redirect_uris: Vec<String>,
    pub allowed_scopes: Vec<String>,
    pub grant_types: Vec<String>,
    pub access_token_ttl: i64,
    pub refresh_token_ttl: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

// ── Session ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub idp_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

impl Session {
    pub fn is_valid(&self) -> bool {
        self.revoked_at.is_none() && self.expires_at > OffsetDateTime::now_utc()
    }
}

// ── RefreshToken ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefreshToken {
    pub id: Uuid,
    pub session_id: Uuid,
    pub application_id: Uuid,
    pub token_hash: String,
    pub scope: String,
    pub previous_id: Option<Uuid>,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

// ── Role + Permission ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Role {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parent_role_id: Option<Uuid>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Permission {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
    pub created_at: OffsetDateTime,
}

// ── IdP config ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IdpType {
    Local,
    Oidc,
    Oauth2,
    Ldap,
}

impl std::fmt::Display for IdpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Oidc => write!(f, "oidc"),
            Self::Oauth2 => write!(f, "oauth2"),
            Self::Ldap => write!(f, "ldap"),
        }
    }
}

impl std::str::FromStr for IdpType {
    type Err = crate::errors::CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" => Ok(Self::Local),
            "oidc" => Ok(Self::Oidc),
            "oauth2" => Ok(Self::Oauth2),
            "ldap" => Ok(Self::Ldap),
            other => Err(crate::errors::CoreError::Validation(format!(
                "unknown idp type: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdpConfig {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub provider_type: IdpType,
    pub name: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// ── AuditEvent ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub event_type: String,
    pub actor_id: Option<Uuid>,
    pub target_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: OffsetDateTime,
}
