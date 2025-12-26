//! Server-only code for Authit.
//!
//! This crate contains code that only runs on the server:
//! - Configuration
//! - Kanidm API client
//! - Session management

mod config;
mod kanidm;

pub use config::Config;
pub use kanidm::KanidmClient;

use eyre::{eyre, WrapErr};
use types::{decode_session, Error, UserSession, SESSION_COOKIE_NAME};

/// Extract the user session from the request cookie.
pub async fn get_session_from_cookie() -> Result<UserSession, Error> {
    use axum::http::HeaderMap;
    use dioxus::fullstack::extract;

    let headers: HeaderMap = extract().await.wrap_err("failed to extract headers")?;

    let cookie_header = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| eyre!("no cookies in request"))?;

    for cookie_str in cookie_header.split(';') {
        let cookie_str = cookie_str.trim();
        if let Some(value) = cookie_str.strip_prefix(&format!("{}=", SESSION_COOKIE_NAME)) {
            return decode_session(value)
                .wrap_err("invalid session cookie")
                .map_err(Error::from);
        }
    }

    Err(eyre!("session cookie not found").into())
}

/// Require an authenticated admin session, returning the session if valid.
pub async fn require_admin_session() -> Result<UserSession, Error> {
    let session = get_session_from_cookie().await?;
    let config = Config::from_env()?;

    tracing::info!(
        "Admin check - user: {}, groups: {:?}, required: {}",
        session.username,
        session.groups,
        config.admin_group
    );

    if !session.is_in_group(&config.admin_group) {
        return Err(eyre!(
            "access denied: user '{}' must be in '{}' group",
            session.username,
            config.admin_group
        )
        .into());
    }

    Ok(session)
}

/// Get a configured Kanidm client.
pub fn kanidm_client() -> Result<KanidmClient, Error> {
    let config = Config::from_env()?;
    Ok(KanidmClient::new(config.kanidm_url, config.kanidm_token))
}
