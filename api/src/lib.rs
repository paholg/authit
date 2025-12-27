use dioxus::prelude::*;
use types::{
    ProvisionLinkInfo, ResetLink, UserSession,
    kanidm::{Group, Person},
};
use uuid::Uuid;

#[post("/api/current-user")]
pub async fn get_current_user() -> ServerFnResult<Option<UserSession>> {
    match server::get_session_from_cookie().await {
        Ok(session) => Ok(Some(session)),
        Err(_) => Ok(None),
    }
}

#[post("/api/users")]
pub async fn list_users() -> ServerFnResult<Vec<Person>> {
    server::require_admin_session().await?;
    Ok(server::KANIDM_CLIENT.list_persons().await?)
}

#[post("/api/groups")]
pub async fn list_groups() -> ServerFnResult<Vec<Group>> {
    server::require_admin_session().await?;
    Ok(server::KANIDM_CLIENT.list_groups().await?)
}

#[post("/api/users/groups")]
pub async fn update_user_group(user_id: Uuid, group_id: Uuid, add: bool) -> ServerFnResult<()> {
    server::require_admin_session().await?;
    if add {
        server::KANIDM_CLIENT
            .add_user_to_group(&group_id, &user_id)
            .await?;
    } else {
        server::KANIDM_CLIENT
            .remove_user_from_group(&group_id, &user_id)
            .await?;
    }

    Ok(())
}

#[post("/api/users/reset-link")]
pub async fn generate_reset_link(user_id: Uuid) -> ServerFnResult<ResetLink> {
    server::require_admin_session().await?;
    Ok(server::KANIDM_CLIENT
        .generate_credential_reset_link(&user_id)
        .await?)
}

#[post("/api/users/delete")]
pub async fn delete_user(user_id: Uuid) -> ServerFnResult<()> {
    server::require_admin_session().await?;
    server::KANIDM_CLIENT.delete_person(&user_id).await?;
    Ok(())
}

#[post("/api/users/create")]
pub async fn create_user(
    name: String,
    display_name: String,
    email_address: String,
) -> ServerFnResult<()> {
    server::require_admin_session().await?;
    server::KANIDM_CLIENT
        .create_person(&name, &display_name, &email_address)
        .await?;
    Ok(())
}

#[post("/api/provision/generate")]
pub async fn generate_provision_url(
    duration_hours: u32,
    max_uses: Option<u32>,
) -> ServerFnResult<String> {
    server::require_admin_session().await?;
    let token = server::create_provision_link(duration_hours, max_uses)?;
    let base_url = server::get_request_base_url().await?;
    Ok(format!("{}/provision/{}", base_url, token))
}

#[post("/api/provision/verify")]
pub async fn verify_provision(token: String) -> ServerFnResult<ProvisionLinkInfo> {
    Ok(server::verify_provision_link(&token)?)
}

#[post("/api/provision/complete")]
pub async fn complete_provision(
    token: String,
    name: String,
    display_name: String,
    email_address: String,
) -> ServerFnResult<ResetLink> {
    let record = server::consume_provision_link(&token)?;

    match server::KANIDM_CLIENT
        .create_person(&name, &display_name, &email_address)
        .await
    {
        Ok(()) => {
            let person = server::KANIDM_CLIENT.get_person(&name).await?;
            Ok(server::KANIDM_CLIENT
                .generate_credential_reset_link(&person.uuid)
                .await?)
        }
        Err(e) => {
            // Restore the link so user can retry
            let _ = server::unconsume_provision_link(record);
            Err(e.into())
        }
    }
}
