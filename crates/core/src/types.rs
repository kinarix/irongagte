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
    pub attributes: serde_json::Value,
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
    /// Namespace for this app's custom JWT claims. Final claim keys are
    /// `<claim_prefix>:<claim_definition.key>`. Standard OIDC claims (`sub`,
    /// `email`, `name`, …) are emitted unprefixed.
    pub claim_prefix: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

// ── Claim definitions ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    /// Single string value. Conflicts: user-direct overrides; among groups, the
    /// highest `priority` wins; ties broken by `created_at` ascending.
    Scalar,
    /// Multi-valued claim. Emitted as a JSON array of strings; all
    /// group/user values are merged and deduped.
    Multi,
}

impl ClaimType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Multi => "multi",
        }
    }
}

impl std::str::FromStr for ClaimType {
    type Err = crate::errors::CoreError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "scalar" => Ok(Self::Scalar),
            "multi" => Ok(Self::Multi),
            other => Err(crate::errors::CoreError::Validation(format!(
                "unknown claim type: {other}"
            ))),
        }
    }
}

/// Declaration of a single custom JWT claim emitted by an application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaimDefinition {
    pub id: Uuid,
    pub application_id: Uuid,
    pub key: String,
    pub claim_type: ClaimType,
    pub description: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// A single `(group, claim_definition, value)` assignment. Multi claims allow
/// multiple rows per `(group_id, claim_def_id)`; the table-level primary key
/// is `(group_id, claim_def_id, value)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupClaim {
    pub group_id: Uuid,
    pub claim_def_id: Uuid,
    pub value: String,
    pub created_at: OffsetDateTime,
}

/// A direct user-to-claim assignment. For scalar claims this overrides any
/// group-derived value; for multi claims it merges with group values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserClaim {
    pub user_id: Uuid,
    pub claim_def_id: Uuid,
    pub value: String,
    pub created_at: OffsetDateTime,
}

/// Resources allowed for Operator RBAC (system-level authorization).
/// The constants below mirror this list and are the recommended way to refer
/// to a resource in code — never use a string literal at a permission check site.
pub const ALLOWED_OPERATOR_RESOURCES: &[&str] = &[
    "tenants",
    "users",
    "applications",
    "operators",
    "operator_roles",
    "operator_permissions",
    "groups",
    "idp_configs",
    "audit_events",
    "claims",
    "sessions",
];

pub mod op_resource {
    pub const TENANTS: &str = "tenants";
    pub const USERS: &str = "users";
    pub const APPLICATIONS: &str = "applications";
    pub const OPERATORS: &str = "operators";
    pub const OPERATOR_ROLES: &str = "operator_roles";
    pub const OPERATOR_PERMISSIONS: &str = "operator_permissions";
    pub const GROUPS: &str = "groups";
    pub const IDP_CONFIGS: &str = "idp_configs";
    pub const AUDIT_EVENTS: &str = "audit_events";
    pub const CLAIMS: &str = "claims";
    pub const SESSIONS: &str = "sessions";
}

/// Actions allowed for Operator RBAC.
pub const ALLOWED_OPERATOR_ACTIONS: &[&str] = &[
    "create", "read", "update", "delete", "list", "assign", "revoke",
];

pub mod op_action {
    pub const CREATE: &str = "create";
    pub const READ: &str = "read";
    pub const UPDATE: &str = "update";
    pub const DELETE: &str = "delete";
    pub const LIST: &str = "list";
    pub const ASSIGN: &str = "assign";
    pub const REVOKE: &str = "revoke";
}

/// Top-level JWT claim names that the issuer reserves; admins may not map onto them.
pub const RESERVED_CLAIM_TARGETS: &[&str] = &[
    "sub",
    "iss",
    "aud",
    "exp",
    "iat",
    "jti",
    "nbf",
    "nonce",
    "auth_time",
    "acr",
    "amr",
    "azp",
    "tenant_id",
    "scope",
];

/// Validate an application's claim prefix. Returns `Err` with a human-readable
/// reason if the prefix is invalid. A valid prefix is non-empty, does not collide
/// with a reserved JWT claim name, and contains only `[A-Za-z0-9_-]`.
pub fn validate_claim_prefix(prefix: &str) -> Result<(), String> {
    if prefix.is_empty() {
        return Err("claim_prefix must not be empty".into());
    }
    if RESERVED_CLAIM_TARGETS.contains(&prefix) {
        return Err(format!(
            "claim_prefix '{prefix}' collides with a reserved JWT claim name"
        ));
    }
    if !prefix
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "claim_prefix '{prefix}' must contain only letters, digits, underscores or hyphens"
        ));
    }
    Ok(())
}

/// Validate a claim definition key. Same rules as `validate_claim_prefix`, plus
/// the result `<prefix>:<key>` must not match a reserved claim name.
pub fn validate_claim_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("claim key must not be empty".into());
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "claim key '{key}' must contain only letters, digits, underscores or hyphens"
        ));
    }
    Ok(())
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

// ── Operator RBAC ─────────────────────────────────────────────────────────────

/// System-level permission (resource, action) pair for operator authorization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorPermission {
    pub id: Uuid,
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
    pub created_at: OffsetDateTime,
}

/// Operator role. `tenant_id` is `None` for global (cross-tenant) roles and
/// `Some(id)` for roles whose permissions are restricted to a single tenant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorRole {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Assignment of an operator to an operator role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorRoleAssignment {
    pub operator_id: Uuid,
    pub operator_role_id: Uuid,
    pub assigned_at: OffsetDateTime,
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

// ── UserCredentials ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserCredentials {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub password_hash: Option<String>,
    pub totp_secret: Option<String>,
    pub totp_enabled: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// ── Operator (Irongate dashboard user, NOT an end-user) ──────────────────────

/// An Operator manages the irongate IAM instance itself: tenants, applications,
/// roles, permissions, groups, etc. This is the Auth0-style "Dashboard user" —
/// completely separate from end users who authenticate *through* irongate to
/// access their own applications.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Operator {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub status: OperatorStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub last_login_at: Option<OffsetDateTime>,
    pub deleted_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperatorStatus {
    Active,
    Suspended,
}

impl std::str::FromStr for OperatorStatus {
    type Err = crate::errors::CoreError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(OperatorStatus::Active),
            "suspended" => Ok(OperatorStatus::Suspended),
            other => Err(crate::errors::CoreError::Validation(format!(
                "invalid operator status '{other}'"
            ))),
        }
    }
}

impl OperatorStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperatorStatus::Active => "active",
            OperatorStatus::Suspended => "suspended",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorCredentials {
    pub operator_id: Uuid,
    pub password_hash: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// ── MagicLink ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MagicLink {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: OffsetDateTime,
    pub used_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

// ── PasskeyCredential ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PasskeyCredential {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    /// base64url-unpadded encoding of the WebAuthn credential ID bytes
    pub credential_id: String,
    pub friendly_name: Option<String>,
    /// Serialized `webauthn_rs::prelude::Passkey` — opaque JSON blob
    pub passkey_json: serde_json::Value,
    pub created_at: OffsetDateTime,
    pub last_used_at: Option<OffsetDateTime>,
}

// ── Group ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Group {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub display_name: String,
    pub external_id: Option<String>,
    /// Tiebreaker for scalar claim conflicts when a user is in multiple groups
    /// assigning the same scalar claim. Higher wins; ties → `created_at` asc.
    pub priority: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// ── SigningKey ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum KeyAlgorithm {
    Rs256,
    Es256,
}

impl std::fmt::Display for KeyAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rs256 => write!(f, "RS256"),
            Self::Es256 => write!(f, "ES256"),
        }
    }
}

impl std::str::FromStr for KeyAlgorithm {
    type Err = crate::errors::CoreError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "RS256" => Ok(Self::Rs256),
            "ES256" => Ok(Self::Es256),
            other => Err(crate::errors::CoreError::Validation(format!(
                "unknown key algorithm: {other}"
            ))),
        }
    }
}

/// An asymmetric signing key with lifecycle metadata. `tenant_id == None` is a
/// global key; per-tenant keys carry the owning tenant id.
#[derive(Debug, Clone)]
pub struct SigningKeyRecord {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub algorithm: KeyAlgorithm,
    /// PEM-encoded private key (PKCS#8).
    pub private_key_pem: String,
    /// PEM-encoded public key (SubjectPublicKeyInfo).
    pub public_key_pem: String,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    /// When this key was retired from signing. A retired key may still verify
    /// in-flight tokens until `expires_at` passes.
    pub retired_at: Option<OffsetDateTime>,
}

impl SigningKeyRecord {
    /// True if this key can still be used for signing new tokens.
    pub fn is_active(&self) -> bool {
        let now = OffsetDateTime::now_utc();
        self.retired_at.is_none() && now < self.expires_at
    }
}

// ── AuditEvent ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEvent {
    pub id: Uuid,
    /// `None` for system-level events (operator / operator-role / operator-permission
    /// CRUD) that have no tenant context. All tenant-scoped events use `Some(...)`.
    pub tenant_id: Option<Uuid>,
    pub event_type: String,
    pub actor_id: Option<Uuid>,
    pub target_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: OffsetDateTime,
}
