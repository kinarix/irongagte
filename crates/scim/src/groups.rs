use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use irongate_core::{
    repositories::{GroupRepository, UserRepository},
    types::Group,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    error::ScimError,
    filter::{matches_filter, parse},
    types::{ListParams, ListResponse, ScimGroup, ScimGroupInput},
};

pub struct GroupState {
    pub groups: Arc<dyn GroupRepository>,
    pub users: Arc<dyn UserRepository>,
    pub base_url: String,
    pub tenant_id: Uuid,
}

pub async fn list_groups(
    State(state): State<Arc<GroupState>>,
    Query(params): Query<ListParams>,
) -> Result<impl IntoResponse, ScimError> {
    let start = params.start_index.unwrap_or(1).max(1);
    let count = params.count.unwrap_or(100).min(1000) as i64;
    let offset = (start - 1) as i64;

    let groups = state
        .groups
        .list(state.tenant_id, count, offset)
        .await
        .map_err(ScimError::from)?;

    let filter_expr = params.filter.as_deref().map(parse).transpose()?;

    let mut resources: Vec<ScimGroup> = Vec::new();
    for g in &groups {
        if let Some(ref f) = filter_expr {
            let mut attrs = std::collections::HashMap::new();
            attrs.insert("displayName", g.display_name.as_str());
            if let Some(ext) = g.external_id.as_deref() {
                attrs.insert("externalId", ext);
            }
            if !matches_filter(f, &attrs) {
                continue;
            }
        }
        let members = state
            .groups
            .list_members(g.id, state.tenant_id)
            .await
            .map_err(ScimError::from)?;
        resources.push(ScimGroup::from_group(g, &members, &state.base_url));
    }

    let total = resources.len();
    Ok((
        StatusCode::OK,
        Json(ListResponse::new(resources, total, start)),
    ))
}

pub async fn get_group(
    State(state): State<Arc<GroupState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ScimError> {
    let group_id = crate::types::parse_id(&id)?;
    let group = state
        .groups
        .get_by_id(group_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;

    let members = state
        .groups
        .list_members(group_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;

    Ok((
        StatusCode::OK,
        Json(ScimGroup::from_group(&group, &members, &state.base_url)),
    ))
}

pub async fn create_group(
    State(state): State<Arc<GroupState>>,
    Json(input): Json<ScimGroupInput>,
) -> Result<impl IntoResponse, ScimError> {
    let now = OffsetDateTime::now_utc();
    let group = Group {
        id: Uuid::new_v4(),
        tenant_id: state.tenant_id,
        display_name: input.display_name.clone(),
        external_id: input.external_id.clone(),
        priority: 0,
        created_at: now,
        updated_at: now,
    };

    let created = state.groups.create(group).await.map_err(ScimError::from)?;

    // Add initial members if provided
    for member in &input.members {
        if let Ok(user_id) = Uuid::parse_str(&member.value) {
            state
                .groups
                .add_member(created.id, user_id, state.tenant_id)
                .await
                .map_err(ScimError::from)?;
        }
    }

    let members = state
        .groups
        .list_members(created.id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;

    Ok((
        StatusCode::CREATED,
        Json(ScimGroup::from_group(&created, &members, &state.base_url)),
    ))
}

pub async fn replace_group(
    State(state): State<Arc<GroupState>>,
    Path(id): Path<String>,
    Json(input): Json<ScimGroupInput>,
) -> Result<impl IntoResponse, ScimError> {
    let group_id = crate::types::parse_id(&id)?;
    let existing = state
        .groups
        .get_by_id(group_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;

    let updated = Group {
        id: existing.id,
        tenant_id: existing.tenant_id,
        display_name: input.display_name.clone(),
        external_id: input.external_id.clone(),
        priority: existing.priority,
        created_at: existing.created_at,
        updated_at: OffsetDateTime::now_utc(),
    };

    let saved = state.groups.update(updated).await.map_err(ScimError::from)?;

    // Replace membership: remove all existing, add new set
    let current_members = state
        .groups
        .list_members(group_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;

    let new_ids: std::collections::HashSet<String> =
        input.members.iter().map(|m| m.value.clone()).collect();

    for user in &current_members {
        if !new_ids.contains(&user.id.to_string()) {
            state
                .groups
                .remove_member(group_id, user.id, state.tenant_id)
                .await
                .map_err(ScimError::from)?;
        }
    }

    let existing_ids: std::collections::HashSet<String> =
        current_members.iter().map(|u| u.id.to_string()).collect();

    for member in &input.members {
        if !existing_ids.contains(&member.value) {
            if let Ok(user_id) = Uuid::parse_str(&member.value) {
                state
                    .groups
                    .add_member(group_id, user_id, state.tenant_id)
                    .await
                    .map_err(ScimError::from)?;
            }
        }
    }

    let members = state
        .groups
        .list_members(group_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;

    Ok((
        StatusCode::OK,
        Json(ScimGroup::from_group(&saved, &members, &state.base_url)),
    ))
}

pub async fn patch_group(
    State(state): State<Arc<GroupState>>,
    Path(id): Path<String>,
    Json(patch): Json<crate::types::PatchOp>,
) -> Result<impl IntoResponse, ScimError> {
    let group_id = crate::types::parse_id(&id)?;
    let mut group = state
        .groups
        .get_by_id(group_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;

    for op in patch.operations {
        let path = op.path.as_deref().unwrap_or("").to_lowercase();
        let value = op.value;

        match (op.op, path.as_str()) {
            (crate::types::PatchOpType::Replace, "displayname")
            | (crate::types::PatchOpType::Add, "displayname") => {
                group.display_name = value
                    .and_then(|v| v.as_str().map(String::from))
                    .ok_or_else(|| ScimError::BadRequest("displayName must be string".into()))?;
            }
            (crate::types::PatchOpType::Add, "members") => {
                if let Some(arr) = value.and_then(|v| {
                    if v.is_array() { Some(v) } else { None }
                }) {
                    for item in arr.as_array().unwrap_or(&vec![]) {
                        if let Some(uid) = item.get("value").and_then(|v| v.as_str()) {
                            if let Ok(user_id) = Uuid::parse_str(uid) {
                                state
                                    .groups
                                    .add_member(group_id, user_id, state.tenant_id)
                                    .await
                                    .map_err(ScimError::from)?;
                            }
                        }
                    }
                }
            }
            (crate::types::PatchOpType::Remove, "members") => {
                if let Some(arr) = value.and_then(|v| {
                    if v.is_array() { Some(v) } else { None }
                }) {
                    for item in arr.as_array().unwrap_or(&vec![]) {
                        if let Some(uid) = item.get("value").and_then(|v| v.as_str()) {
                            if let Ok(user_id) = Uuid::parse_str(uid) {
                                state
                                    .groups
                                    .remove_member(group_id, user_id, state.tenant_id)
                                    .await
                                    .map_err(ScimError::from)?;
                            }
                        }
                    }
                }
            }
            _ => {
                return Err(ScimError::UnsupportedOperation(format!(
                    "patch path '{path}' is not supported"
                )));
            }
        }
    }

    group.updated_at = OffsetDateTime::now_utc();
    let saved = state.groups.update(group).await.map_err(ScimError::from)?;
    let members = state
        .groups
        .list_members(group_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;

    Ok((
        StatusCode::OK,
        Json(ScimGroup::from_group(&saved, &members, &state.base_url)),
    ))
}

pub async fn delete_group(
    State(state): State<Arc<GroupState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ScimError> {
    let group_id = crate::types::parse_id(&id)?;
    state
        .groups
        .delete(group_id, state.tenant_id)
        .await
        .map_err(ScimError::from)?;
    Ok(StatusCode::NO_CONTENT)
}
