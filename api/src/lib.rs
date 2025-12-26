//! This crate contains fullstack server functions.
//!
//! Types are re-exported from the `types` crate for convenience.

pub use types::{
    decode_session, encode_session, Entry, Group, Person, SessionError, UserSession,
    SESSION_COOKIE_NAME,
};

use dioxus::prelude::*;

#[post("/api/current-user")]
pub async fn get_current_user() -> Result<Option<UserSession>, ServerFnError> {
    match server::get_session_from_cookie().await {
        Ok(session) => Ok(Some(session)),
        Err(_) => Ok(None),
    }
}

#[post("/api/users")]
pub async fn list_users() -> Result<Vec<Person>, ServerFnError> {
    server::require_admin_session().await?;
    let client = server::kanidm_client()?;
    client
        .list_persons()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/groups")]
pub async fn list_groups() -> Result<Vec<Group>, ServerFnError> {
    server::require_admin_session().await?;
    let client = server::kanidm_client()?;
    client
        .list_groups()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/users/groups")]
pub async fn update_user_group(
    user_id: String,
    group_id: String,
    add: bool,
) -> Result<(), ServerFnError> {
    server::require_admin_session().await?;
    let client = server::kanidm_client()?;

    if add {
        client
            .add_user_to_group(&group_id, &user_id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))
    } else {
        client
            .remove_user_from_group(&group_id, &user_id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))
    }
}

#[post("/api/users/reset-link")]
pub async fn generate_reset_link(user_id: String) -> Result<String, ServerFnError> {
    server::require_admin_session().await?;
    let client = server::kanidm_client()?;
    client
        .generate_credential_reset_link(&user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
