use dioxus::{fullstack::reqwest::Url, prelude::*};
use types::{
    ResetLink,
    kanidm::{Group, Person},
};
use uuid::Uuid;

#[post("/api/current-user")]
pub async fn get_current_user() -> ServerFnResult<Option<Person>> {
    match server::get_current_user().await {
        Ok(person) => Ok(Some(person)),
        Err(_) => Ok(None),
    }
}

#[post("/api/users")]
pub async fn list_users() -> ServerFnResult<Vec<Person>> {
    server::with_admin_session(|_| async { Ok(server::KANIDM_CLIENT.list_persons().await?) }).await
}

#[post("/api/groups")]
pub async fn list_groups() -> ServerFnResult<Vec<Group>> {
    server::with_admin_session(|_| async { Ok(server::KANIDM_CLIENT.list_groups().await?) }).await
}

#[post("/api/users/groups")]
pub async fn update_user_group(user_id: Uuid, group_id: Uuid, add: bool) -> ServerFnResult<()> {
    server::with_admin_session(|_| async move {
        if add {
            server::KANIDM_CLIENT
                .add_user_to_group(&group_id.to_string(), &user_id)
                .await?;
        } else {
            server::KANIDM_CLIENT
                .remove_user_from_group(&group_id, &user_id)
                .await?;
        }
        Ok(())
    })
    .await
}

#[post("/api/users/reset-link")]
pub async fn generate_reset_link(user_id: Uuid) -> ServerFnResult<ResetLink> {
    server::with_admin_session(|_| async move {
        Ok(server::KANIDM_CLIENT
            .generate_credential_reset_link(&user_id)
            .await?)
    })
    .await
}

#[post("/api/users/delete")]
pub async fn delete_user(user_id: Uuid) -> ServerFnResult<()> {
    server::with_admin_session(|_| async move {
        server::KANIDM_CLIENT.delete_person(&user_id).await?;
        Ok(())
    })
    .await
}

#[post("/api/users/create")]
pub async fn create_user(
    name: String,
    display_name: String,
    email_address: String,
) -> ServerFnResult<()> {
    server::with_admin_session(|_| async {
        server::KANIDM_CLIENT
            .create_person(&name, &display_name, &email_address)
            .await?;
        Ok(())
    })
    .await
}

#[post("/api/provision/generate")]
pub async fn generate_provision_url(
    duration_hours: u32,
    max_uses: Option<u8>,
    group_names: Vec<String>,
) -> ServerFnResult<Url> {
    server::with_admin_session(|_| async move {
        let duration = std::time::Duration::from_secs(duration_hours as u64 * 3600);
        let link = server::ProvisionLink::create(duration, max_uses, group_names).await?;
        let token = link.as_token()?;
        Ok(server::CONFIG.provision_url(token)?)
    })
    .await
}

#[post("/api/provision/verify")]
pub async fn verify_provision(token: String) -> ServerFnResult<()> {
    server::ProvisionLink::find_token(token).await?.verify()?;
    Ok(())
}

#[post("/api/provision/complete")]
pub async fn complete_provision(
    token: String,
    name: String,
    display_name: String,
    email_address: String,
) -> ServerFnResult<ResetLink> {
    let link = server::ProvisionLink::consume(token).await?;

    let result = server::KANIDM_CLIENT
        .create_person_with_link(&name, &display_name, &email_address)
        .await;

    if result.is_err() {
        let _ = link.decrement().await;
        return Ok(result?);
    }

    let reset_link = result?;

    // Add the user to the groups specified in the provision link
    let person = server::KANIDM_CLIENT.get_person(&name).await?;
    for group_name in link.groups() {
        server::KANIDM_CLIENT
            .add_user_to_group(group_name, &person.uuid)
            .await?;
    }

    Ok(reset_link)
}
