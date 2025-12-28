mod auth_routes;
mod config;
mod kanidm;
pub mod storage;

use axum::Router;
use axum::http::HeaderMap;
use dioxus::fullstack::FullstackContext;
use types::{Result, SESSION_COOKIE_NAME, UserSession, decode_session, err};

use crate::auth_routes::{AuthState, auth_router};
pub use crate::config::CONFIG;
pub use crate::kanidm::KANIDM_CLIENT;
pub use crate::storage::ProvisionLink;

pub async fn init() -> Result<Router> {
    storage::migrate().await?;
    let auth_state = AuthState::new()?;
    Ok(auth_router(auth_state))
}

pub async fn get_request_base_url() -> Result<String> {
    use axum::http::HeaderMap;
    use dioxus::fullstack::FullstackContext;

    let headers: HeaderMap = FullstackContext::extract().await?;

    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| err!("no host header in request"))?;

    // Use X-Forwarded-Proto if set (by reverse proxy), otherwise assume http
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");

    Ok(format!("{}://{}", proto, host))
}

/// Extract the user session from the request cookie.
pub async fn get_session_from_cookie() -> Result<UserSession> {
    let headers: HeaderMap = FullstackContext::extract().await?;

    let cookie_header = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| err!("no cookies in request"))?;

    for cookie_str in cookie_header.split(';') {
        let cookie_str = cookie_str.trim();
        if let Some(value) = cookie_str.strip_prefix(&format!("{}=", SESSION_COOKIE_NAME)) {
            return decode_session(value);
        }
    }

    Err(err!("session cookie not found"))
}

/// Require an authenticated admin session, returning the session if valid.
pub async fn require_admin_session() -> Result<UserSession> {
    let session = get_session_from_cookie().await?;

    if !session.is_in_group(&CONFIG.admin_group) {
        return Err(err!(
            "access denied: user '{}' must be in '{}' group",
            session.username,
            CONFIG.admin_group
        ));
    }

    Ok(session)
}
