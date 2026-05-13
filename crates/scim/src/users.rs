use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use irongate_core::{
    repositories::{GroupRepository, UserRepository},
    types::{User, UserStatus},
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    error::ScimError,
    filter::{matches_filter, parse},
    types::{ListParams, ListResponse, ScimUser, ScimUserInput},
};

pub struct UserState {
    pub users: Arc<dyn UserRepository>,
    pub groups: Arc<dyn GroupRepository>,
    pub base_url: String,
    pub tenant_id: Uuid,
}

pub async fn list_users(
    State(state): State<Arc<UserState>>,
    Query(params): Query<ListParams>,
) -> Result<impl IntoResponse, ScimError> {
    let start = params.start_index.unwrap_or(1).max(1);
    let count = params.count.unwrap_or(100).min(1000) as i64;
    let offset = (start - 1) as i64;

    let users = state
        .users
        .list(state.tenant_id, count, offset)
        .await
        .map_err(ScimError::from)?;

    let filter_expr = params
        .filter
        .as_deref()
        .map(parse)
        .transpose()?;

    let resources: Vec<ScimUser> = users
        .iter()
        .filter(|u| {
            if let Some(ref f) = filter_expr {
                let active_str = if matches!(u.status, UserStatus::Active) { "true" } else { "false" };
                let mut attrs = std::collections::HashMap::new();
                attrs.insert("userName", u.email.as_str());
                if let Some(n) = u.name.as_deref() {
                    attrs.insert("displayName", n);
                }
                attrs.insert("active", active_str);
                matches_filter(f, &attrs)
            } else {
                true
            }
        })
        .map(|u| ScimUser::from_user(u, &state.base_url))
        .collect();

    let total = resources.len();
    Ok((
        StatusCode::OK,
        Json(ListResponse::new(resources, total, start)),
    ))
}

pub async fn get_user(
    State(state): State<Arc<UserState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ScimError> {
    let user_id = crate::types::parse_id(&id)?;
    let user = state
        .users
        .get_by_id(user_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;

    Ok((StatusCode::OK, Json(ScimUser::from_user(&user, &state.base_url))))
}

pub async fn create_user(
    State(state): State<Arc<UserState>>,
    Json(input): Json<ScimUserInput>,
) -> Result<impl IntoResponse, ScimError> {
    let now = OffsetDateTime::now_utc();
    let user = User {
        id: Uuid::new_v4(),
        tenant_id: state.tenant_id,
        email: input.user_name.clone(),
        email_verified: false,
        name: input.display_name.or_else(|| {
            input.name.as_ref().and_then(|n| n.formatted.clone())
        }),
        given_name: input.name.as_ref().and_then(|n| n.given_name.clone()),
        family_name: input.name.as_ref().and_then(|n| n.family_name.clone()),
        picture_url: None,
        status: if input.active { UserStatus::Active } else { UserStatus::Suspended },
        attributes: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        last_login_at: None,
        deleted_at: None,
    };

    let created = state.users.create(user).await.map_err(ScimError::from)?;
    Ok((
        StatusCode::CREATED,
        Json(ScimUser::from_user(&created, &state.base_url)),
    ))
}

pub async fn replace_user(
    State(state): State<Arc<UserState>>,
    Path(id): Path<String>,
    Json(input): Json<ScimUserInput>,
) -> Result<impl IntoResponse, ScimError> {
    let user_id = crate::types::parse_id(&id)?;
    let existing = state
        .users
        .get_by_id(user_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;

    let updated = User {
        id: existing.id,
        tenant_id: existing.tenant_id,
        email: input.user_name.clone(),
        email_verified: existing.email_verified,
        name: input.display_name.or_else(|| {
            input.name.as_ref().and_then(|n| n.formatted.clone())
        }),
        given_name: input.name.as_ref().and_then(|n| n.given_name.clone()),
        family_name: input.name.as_ref().and_then(|n| n.family_name.clone()),
        picture_url: existing.picture_url,
        status: if input.active { UserStatus::Active } else { UserStatus::Suspended },
        attributes: existing.attributes,
        created_at: existing.created_at,
        updated_at: OffsetDateTime::now_utc(),
        last_login_at: existing.last_login_at,
        deleted_at: existing.deleted_at,
    };

    let saved = state.users.update(updated).await.map_err(ScimError::from)?;
    Ok((StatusCode::OK, Json(ScimUser::from_user(&saved, &state.base_url))))
}

pub async fn patch_user(
    State(state): State<Arc<UserState>>,
    Path(id): Path<String>,
    Json(patch): Json<crate::types::PatchOp>,
) -> Result<impl IntoResponse, ScimError> {
    let user_id = crate::types::parse_id(&id)?;
    let mut user = state
        .users
        .get_by_id(user_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;

    for op in patch.operations {
        let path = op.path.as_deref().unwrap_or("").to_lowercase();
        let value = op.value;

        match (op.op, path.as_str()) {
            (crate::types::PatchOpType::Replace, "active")
            | (crate::types::PatchOpType::Add, "active") => {
                let active = value
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| ScimError::BadRequest("active must be bool".into()))?;
                user.status = if active { UserStatus::Active } else { UserStatus::Suspended };
            }
            (crate::types::PatchOpType::Replace, "username")
            | (crate::types::PatchOpType::Add, "username") => {
                user.email = value
                    .and_then(|v| v.as_str().map(String::from))
                    .ok_or_else(|| ScimError::BadRequest("userName must be string".into()))?;
            }
            (crate::types::PatchOpType::Replace, "displayname")
            | (crate::types::PatchOpType::Add, "displayname") => {
                user.name = value.and_then(|v| v.as_str().map(String::from));
            }
            _ => {
                return Err(ScimError::UnsupportedOperation(format!(
                    "patch path '{path}' is not supported"
                )));
            }
        }
    }

    user.updated_at = OffsetDateTime::now_utc();
    let saved = state.users.update(user).await.map_err(ScimError::from)?;
    Ok((StatusCode::OK, Json(ScimUser::from_user(&saved, &state.base_url))))
}

pub async fn delete_user(
    State(state): State<Arc<UserState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ScimError> {
    let user_id = crate::types::parse_id(&id)?;
    state
        .users
        .soft_delete(user_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;
    Ok(StatusCode::NO_CONTENT)
}
