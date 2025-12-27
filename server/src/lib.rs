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

use base64::prelude::*;
use eyre::{WrapErr, eyre};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use sha2::Sha256;
use types::{Error, ProvisionToken, SESSION_COOKIE_NAME, UserSession, decode_session};

type HmacSha256 = Hmac<Sha256>;

/// Get the base URL from the current request (e.g., "https://example.com")
pub async fn get_request_base_url() -> Result<String, Error> {
    use axum::http::HeaderMap;
    use dioxus::fullstack::extract;

    let headers: HeaderMap = extract().await.wrap_err("failed to extract headers")?;

    // Try X-Forwarded-Proto and X-Forwarded-Host first (for reverse proxies)
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("https");

    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| eyre!("no host header in request"))?;

    Ok(format!("{}://{}", proto, host))
}

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

/// Create a signed provision token.
pub fn create_provision_token(duration_hours: u32) -> Result<String, Error> {
    let config = Config::from_env()?;
    let token = ProvisionToken::new(uuid::Uuid::now_v7(), duration_hours as u64 * 3600);

    // Serialize the token
    let token_json = serde_json::to_string(&token).wrap_err("failed to serialize token")?;
    let token_b64 = BASE64_URL_SAFE_NO_PAD.encode(token_json.as_bytes());

    // Create HMAC signature
    let mut mac = HmacSha256::new_from_slice(config.session_secret.expose_secret().as_bytes())
        .wrap_err("invalid secret key")?;
    mac.update(token_b64.as_bytes());
    let signature = BASE64_URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    // Return token.signature format
    Ok(format!("{}.{}", token_b64, signature))
}

/// Verify and decode a provision token.
pub fn verify_provision_token(signed_token: &str) -> Result<ProvisionToken, Error> {
    let config = Config::from_env()?;

    // Split into token and signature
    let parts: Vec<&str> = signed_token.split('.').collect();
    if parts.len() != 2 {
        return Err(eyre!("invalid token format").into());
    }

    let token_b64 = parts[0];
    let signature_b64 = parts[1];

    // Verify signature
    let mut mac = HmacSha256::new_from_slice(config.session_secret.expose_secret().as_bytes())
        .wrap_err("invalid secret key")?;
    mac.update(token_b64.as_bytes());

    let signature = BASE64_URL_SAFE_NO_PAD
        .decode(signature_b64)
        .wrap_err("invalid signature encoding")?;

    mac.verify_slice(&signature)
        .map_err(|_| eyre!("invalid signature"))?;

    // Decode token
    let token_json = BASE64_URL_SAFE_NO_PAD
        .decode(token_b64)
        .wrap_err("invalid token encoding")?;

    let token: ProvisionToken =
        serde_json::from_slice(&token_json).wrap_err("failed to parse token")?;

    // Check expiration
    if token.is_expired() {
        return Err(eyre!("provision link has expired").into());
    }

    Ok(token)
}
