//! This crate contains fullstack server functions.
//!
//! Types are re-exported from the `types` crate for convenience.

pub use types::{
    Entry, Error, Group, Person, ProvisionLinkInfo, ResetLink, SESSION_COOKIE_NAME, UserSession,
    decode_session, encode_session,
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
    server::require_admin_session()
        .await
        .map_err(to_server_error)?;
    server::kanidm_client()
        .map_err(to_server_error)?
        .list_persons()
        .await
        .map_err(to_server_error)
}

#[post("/api/groups")]
pub async fn list_groups() -> Result<Vec<Group>, ServerFnError> {
    server::require_admin_session()
        .await
        .map_err(to_server_error)?;
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
    server::require_admin_session()
        .await
        .map_err(to_server_error)?;
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
pub async fn generate_reset_link(user_id: String) -> Result<ResetLink, ServerFnError> {
    server::require_admin_session()
        .await
        .map_err(to_server_error)?;
    server::kanidm_client()
        .map_err(to_server_error)?
        .generate_credential_reset_link(&user_id)
        .await
        .map_err(to_server_error)
}

#[post("/api/users/delete")]
pub async fn delete_user(user_id: String) -> Result<(), ServerFnError> {
    server::require_admin_session()
        .await
        .map_err(to_server_error)?;
    server::kanidm_client()
        .map_err(to_server_error)?
        .delete_person(&user_id)
        .await
        .map_err(to_server_error)
}

#[post("/api/users/create")]
pub async fn create_user(
    name: String,
    display_name: String,
    mail: Option<String>,
) -> Result<(), ServerFnError> {
    server::require_admin_session()
        .await
        .map_err(to_server_error)?;
    server::kanidm_client()
        .map_err(to_server_error)?
        .create_person(&name, &display_name, mail.as_deref())
        .await
        .map_err(to_server_error)
}

#[post("/api/provision/generate")]
pub async fn generate_provision_url(
    duration_hours: u32,
    max_uses: Option<u32>,
) -> Result<String, ServerFnError> {
    server::require_admin_session()
        .await
        .map_err(to_server_error)?;
    let token = server::create_provision_link(duration_hours, max_uses).map_err(to_server_error)?;
    let base_url = server::get_request_base_url()
        .await
        .map_err(to_server_error)?;
    Ok(format!("{}/provision/{}", base_url, token))
}

#[post("/api/provision/verify")]
pub async fn verify_provision(token: String) -> Result<ProvisionLinkInfo, ServerFnError> {
    server::verify_provision_link(&token).map_err(to_server_error)
}

#[post("/api/provision/complete")]
pub async fn complete_provision(
    token: String,
    name: String,
    display_name: String,
    mail: Option<String>,
) -> Result<ResetLink, ServerFnError> {
    // Consume the provision link (increments use count, checks limits)
    let record = server::consume_provision_link(&token).map_err(to_server_error)?;

    // Create the user - if this fails, restore the link so user can try again
    let client = server::kanidm_client().map_err(to_server_error)?;
    if let Err(e) = client
        .create_person(&name, &display_name, mail.as_deref())
        .await
    {
        // Restore the link so user can retry
        let _ = server::unconsume_provision_link(record);
        return Err(to_server_error(e));
    }

    // Generate credential reset link
    client
        .generate_credential_reset_link(&name)
        .await
        .map_err(to_server_error)
}
