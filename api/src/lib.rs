//! This crate contains fullstack server functions.
//!
//! Types are re-exported from the `types` crate for convenience.

pub use types::{
    decode_session, encode_session, Entry, Error, Group, Person, UserSession, SESSION_COOKIE_NAME,
};

use dioxus::prelude::*;

/// Convert our Error to ServerFnError, preserving the full message with backtrace.
#[cfg(feature = "server")]
fn to_server_error(e: Error) -> ServerFnError {
    ServerFnError::new(e.message)
}

#[post("/api/current-user")]
pub async fn get_current_user() -> Result<Option<UserSession>, ServerFnError> {
    match server::get_session_from_cookie().await {
        Ok(session) => Ok(Some(session)),
        Err(_) => Ok(None),
    }
}

#[post("/api/users")]
pub async fn list_users() -> Result<Vec<Person>, ServerFnError> {
    server::require_admin_session().await.map_err(to_server_error)?;
    server::kanidm_client()
        .map_err(to_server_error)?
        .list_persons()
        .await
        .map_err(to_server_error)
}

#[post("/api/groups")]
pub async fn list_groups() -> Result<Vec<Group>, ServerFnError> {
    server::require_admin_session().await.map_err(to_server_error)?;
    server::kanidm_client()
        .map_err(to_server_error)?
        .list_groups()
        .await
        .map_err(to_server_error)
}

#[post("/api/users/groups")]
pub async fn update_user_group(
    user_id: String,
    group_id: String,
    add: bool,
) -> Result<(), ServerFnError> {
    server::require_admin_session().await.map_err(to_server_error)?;
    let client = server::kanidm_client().map_err(to_server_error)?;

    if add {
        client
            .add_user_to_group(&group_id, &user_id)
            .await
            .map_err(to_server_error)
    } else {
        client
            .remove_user_from_group(&group_id, &user_id)
            .await
            .map_err(to_server_error)
    }
}

#[post("/api/users/reset-link")]
pub async fn generate_reset_link(user_id: String) -> Result<String, ServerFnError> {
    server::require_admin_session().await.map_err(to_server_error)?;
    server::kanidm_client()
        .map_err(to_server_error)?
        .generate_credential_reset_link(&user_id)
        .await
        .map_err(to_server_error)
}
