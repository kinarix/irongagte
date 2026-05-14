use std::sync::Arc;

use axum::{routing::get, Router};

use crate::{
    groups::{
        create_group, delete_group, get_group, list_groups, patch_group, replace_group, GroupState,
    },
    users::{create_user, delete_user, get_user, list_users, patch_user, replace_user, UserState},
};

/// Mount point: `/scim/v2`
pub fn scim_router(user_state: Arc<UserState>, group_state: Arc<GroupState>) -> Router {
    let users = Router::new()
        .route("/", get(list_users).post(create_user))
        .route(
            "/{id}",
            get(get_user)
                .put(replace_user)
                .patch(patch_user)
                .delete(delete_user),
        )
        .with_state(user_state);

    let groups = Router::new()
        .route("/", get(list_groups).post(create_group))
        .route(
            "/{id}",
            get(get_group)
                .put(replace_group)
                .patch(patch_group)
                .delete(delete_group),
        )
        .with_state(group_state);

    Router::new().nest("/Users", users).nest("/Groups", groups)
}
