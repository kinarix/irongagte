use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT claims for an Operator (irongate dashboard user). Strictly distinct from
/// the user/end-user `AccessTokenClaims` — different `actor_type`, no `tenant_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorClaims {
    pub sub: String, // operator id
    pub iss: String,
    pub aud: String, // always "irongate-operator"
    pub exp: u64,
    pub iat: u64,
    pub jti: String,
    pub email: String,
    pub actor_type: String, // always "operator"
}

pub const OPERATOR_AUDIENCE: &str = "irongate-operator";
pub const OPERATOR_ACTOR_TYPE: &str = "operator";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub jti: String,
    pub scope: String,
    pub tenant_id: String,
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extras: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdTokenClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub email: Option<String>,
    pub name: Option<String>,
    pub tenant_id: String,
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before epoch")
        .as_secs()
}

pub fn make_jti() -> String {
    Uuid::new_v4().to_string()
}
