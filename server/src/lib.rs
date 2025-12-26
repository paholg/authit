//! Server-only code for Authit.
//!
//! This crate contains code that only runs on the server:
//! - Configuration
//! - Kanidm API client
//! - Session management

mod config;
mod kanidm;

pub use config::{Config, ConfigError};
pub use kanidm::{KanidmClient, KanidmError};

use dioxus::prelude::*;
use types::{decode_session, UserSession, SESSION_COOKIE_NAME};

/// Extract the user session from the request cookie.
pub async fn get_session_from_cookie() -> Result<UserSession, ServerFnError> {
    use axum::http::HeaderMap;
    use dioxus::fullstack::extract;

    let headers: HeaderMap = extract().await?;

    let cookie_header = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ServerFnError::new("No cookies"))?;

    for cookie_str in cookie_header.split(';') {
        let cookie_str = cookie_str.trim();
        if let Some(value) = cookie_str.strip_prefix(&format!("{}=", SESSION_COOKIE_NAME)) {
            return decode_session(value)
                .map_err(|e| ServerFnError::new(format!("Invalid session: {}", e)));
        }
    }

    Err(ServerFnError::new("Session cookie not found"))
}

/// Require an authenticated admin session, returning the session if valid.
pub async fn require_admin_session() -> Result<UserSession, ServerFnError> {
    let session = get_session_from_cookie().await?;
    let config = Config::from_env().map_err(|e| ServerFnError::new(e.to_string()))?;

    tracing::info!(
        "Admin check - user: {}, groups: {:?}, required: {}",
        session.username,
        session.groups,
        config.admin_group
    );

    if !session.is_in_group(&config.admin_group) {
        return Err(ServerFnError::new(format!(
            "Access denied: user must be in '{}' group",
            config.admin_group
        )));
    }

    Ok(session)
}

/// Get a configured Kanidm client.
pub fn kanidm_client() -> Result<KanidmClient, ServerFnError> {
    let config = Config::from_env().map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(KanidmClient::new(config.kanidm_url, config.kanidm_token))
}
