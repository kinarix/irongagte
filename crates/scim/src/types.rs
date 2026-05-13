use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_USER: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
pub const SCHEMA_GROUP: &str = "urn:ietf:params:scim:schemas:core:2.0:Group";
pub const SCHEMA_LIST: &str = "urn:ietf:params:scim:api:messages:2.0:ListResponse";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScimMeta {
    pub resource_type: String,
    pub created: String,
    pub last_modified: String,
    pub location: String,
}

// ── SCIM User ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScimName {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScimEmail {
    pub value: String,
    #[serde(rename = "type")]
    pub email_type: String,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScimPhoto {
    pub value: String,
    #[serde(rename = "type")]
    pub photo_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScimUser {
    pub schemas: Vec<String>,
    pub id: String,
    pub user_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<ScimName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub emails: Vec<ScimEmail>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub photos: Vec<ScimPhoto>,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub meta: ScimMeta,
}

/// Incoming create/replace payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScimUserInput {
    #[serde(default)]
    pub schemas: Vec<String>,
    pub user_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<ScimName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub emails: Vec<ScimEmail>,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
}

fn default_true() -> bool {
    true
}

// ── SCIM Group ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScimGroupMember {
    pub value: String,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub ref_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScimGroup {
    pub schemas: Vec<String>,
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub members: Vec<ScimGroupMember>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub meta: ScimMeta,
}

/// Incoming create/replace payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScimGroupInput {
    #[serde(default)]
    pub schemas: Vec<String>,
    pub display_name: String,
    #[serde(default)]
    pub members: Vec<ScimGroupMemberInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScimGroupMemberInput {
    pub value: String,
}

// ── SCIM Patch ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchOp {
    pub schemas: Vec<String>,
    // SCIM spec uses capital-O "Operations"; accept both for compatibility
    #[serde(alias = "Operations", alias = "operations")]
    pub operations: Vec<PatchOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchOperation {
    pub op: PatchOpType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatchOpType {
    Add,
    Remove,
    Replace,
}

// ── ListResponse ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse<T: Serialize> {
    pub schemas: Vec<String>,
    pub total_results: usize,
    pub start_index: usize,
    pub items_per_page: usize,
    #[serde(rename = "Resources")]
    pub resources: Vec<T>,
}

impl<T: Serialize> ListResponse<T> {
    pub fn new(resources: Vec<T>, total: usize, start: usize) -> Self {
        let count = resources.len();
        Self {
            schemas: vec![SCHEMA_LIST.into()],
            total_results: total,
            start_index: start,
            items_per_page: count,
            resources,
        }
    }
}

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListParams {
    pub filter: Option<String>,
    pub start_index: Option<usize>,
    pub count: Option<usize>,
}

// ── Conversion helpers ────────────────────────────────────────────────────────

impl ScimUser {
    pub fn from_user(user: &irongate_core::User, base_url: &str) -> Self {
        let formatted = if let (Some(g), Some(f)) = (&user.given_name, &user.family_name) {
            Some(format!("{g} {f}"))
        } else {
            user.name.clone()
        };
        let name = ScimName {
            given_name: user.given_name.clone(),
            family_name: user.family_name.clone(),
            formatted,
        };

        let mut photos = vec![];
        if let Some(url) = &user.picture_url {
            photos.push(ScimPhoto { value: url.clone(), photo_type: "photo".into() });
        }

        ScimUser {
            schemas: vec![SCHEMA_USER.into()],
            id: user.id.to_string(),
            user_name: user.email.clone(),
            display_name: user.name.clone(),
            name: Some(name),
            emails: vec![ScimEmail {
                value: user.email.clone(),
                email_type: "work".into(),
                primary: true,
            }],
            photos,
            active: matches!(user.status, irongate_core::types::UserStatus::Active),
            external_id: None,
            meta: ScimMeta {
                resource_type: "User".into(),
                created: user.created_at.to_string(),
                last_modified: user.updated_at.to_string(),
                location: format!("{base_url}/scim/v2/Users/{}", user.id),
            },
        }
    }
}

impl ScimGroup {
    pub fn from_group(
        group: &irongate_core::Group,
        members: &[irongate_core::User],
        base_url: &str,
    ) -> Self {
        let member_refs: Vec<ScimGroupMember> = members
            .iter()
            .map(|u| ScimGroupMember {
                value: u.id.to_string(),
                ref_: Some(format!("{base_url}/scim/v2/Users/{}", u.id)),
                display: u.name.clone(),
            })
            .collect();

        ScimGroup {
            schemas: vec![SCHEMA_GROUP.into()],
            id: group.id.to_string(),
            display_name: group.display_name.clone(),
            members: member_refs,
            external_id: group.external_id.clone(),
            meta: ScimMeta {
                resource_type: "Group".into(),
                created: group.created_at.to_string(),
                last_modified: group.updated_at.to_string(),
                location: format!("{base_url}/scim/v2/Groups/{}", group.id),
            },
        }
    }
}

// ── Id parsing ────────────────────────────────────────────────────────────────

pub fn parse_id(s: &str) -> Result<Uuid, crate::error::ScimError> {
    Uuid::parse_str(s).map_err(|_| crate::error::ScimError::BadRequest(format!("invalid id: {s}")))
}
