mod auth_routes;
mod config;
mod kanidm;
pub mod storage;
pub mod uuid_v7;

use axum::Router;
use axum::http::HeaderMap;
use dioxus::fullstack::FullstackContext;
use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;
use types::{Result, SESSION_COOKIE_NAME, UserData, err};

use crate::auth_routes::{AuthState, auth_router};
pub use crate::config::CONFIG;
pub use crate::kanidm::KANIDM_CLIENT;
pub use crate::storage::ProvisionLink;
use crate::storage::Session;
use tracing_subscriber::EnvFilter;

pub fn init_tracing() {
    let filter = EnvFilter::builder()
        .with_default_directive(CONFIG.log_level.into())
        .from_env_lossy();

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

trait ReqwestExt {
    async fn try_send<T: DeserializeOwned>(self) -> Result<T>;
}

impl ReqwestExt for RequestBuilder {
    async fn try_send<T: DeserializeOwned>(self) -> Result<T> {
        let response = self.send().await?.error_for_status()?;
        let body = response.bytes().await?;

        match serde_json::from_slice(&body) {
            Ok(r) => Ok(r),
            Err(error) => {
                // NOTE: We don't want to log these responses in production, but
                // they can be useful for debugging.
                // let body = String::from_utf8_lossy(&body);
                // tracing::debug!(?error, ?body, "failed to parse response");
                Err(error.into())
            }
        }
    }
}
pub async fn init() -> Result<Router> {
    storage::migrate().await?;

    let auth_state = AuthState::new()?;
    Ok(auth_router(auth_state))
}

pub async fn get_session_from_cookie() -> Result<UserData> {
    let headers: HeaderMap = FullstackContext::extract().await?;

    let cookie_header = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| err!("no cookies in request"))?;

    for cookie_str in cookie_header.split(';') {
        let cookie_str = cookie_str.trim();
        if let Some(token) = cookie_str.strip_prefix(&format!("{}=", SESSION_COOKIE_NAME)) {
            let session = Session::find_token(token).await?;
            return Ok(session.user_data().clone());
        }
    }

    Err(err!("session cookie not found"))
}

pub async fn require_admin_session() -> Result<UserData> {
    let user_data = get_session_from_cookie().await?;

    if !user_data.is_in_group(&CONFIG.admin_group) {
        return Err(err!(
            "access denied: user '{}' must be in '{}' group",
            user_data.username,
            CONFIG.admin_group
        ));
    }

    Ok(user_data)
}
