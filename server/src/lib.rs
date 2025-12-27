mod auth_routes;
mod config;
mod kanidm;
pub mod storage;

use axum::Router;
use axum::http::HeaderMap;
use base64::prelude::*;
use dioxus::fullstack::FullstackContext;
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use sha2::Sha256;
use types::{
    ProvisionLinkInfo, ProvisionRecord, Result, SESSION_COOKIE_NAME, UserSession, decode_session,
    err,
};
use uuid::Uuid;

use crate::auth_routes::{AuthState, auth_router};
pub use crate::config::CONFIG;
pub use crate::kanidm::KANIDM_CLIENT;
use crate::storage::STORAGE;

type HmacSha256 = Hmac<Sha256>;

pub fn init() -> Router {
    let auth_state = AuthState::new().unwrap();
    auth_router(auth_state)
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

    Err(err!("session cookie not found").into())
}

/// Require an authenticated admin session, returning the session if valid.
pub async fn require_admin_session() -> Result<UserSession> {
    let session = get_session_from_cookie().await?;

    if !session.is_in_group(&CONFIG.admin_group) {
        return Err(err!(
            "access denied: user '{}' must be in '{}' group",
            session.username,
            CONFIG.admin_group
        )
        .into());
    }

    Ok(session)
}

pub fn create_provision_link(duration_hours: u32, max_uses: Option<u32>) -> Result<String> {
    let record = STORAGE.create_link(duration_hours as u64 * 3600, max_uses)?;

    // Sign the UUID for URL tamper-resistance
    sign_uuid(record.id)
}

/// Verify a provision link without consuming it.
/// Returns link info if valid.
pub fn verify_provision_link(signed_token: &str) -> Result<ProvisionLinkInfo> {
    let uuid = extract_uuid(signed_token)?;

    STORAGE.verify_link(uuid)
}

/// Consume a provision link (increment use count).
/// Returns the record for potential rollback, error if expired/exhausted.
pub fn consume_provision_link(signed_token: &str) -> Result<ProvisionRecord> {
    let uuid = extract_uuid(signed_token)?;

    STORAGE.consume_link(uuid)
}

/// Restore a consumed provision link (e.g., if user creation failed).
pub fn unconsume_provision_link(record: ProvisionRecord) -> Result<()> {
    STORAGE.unconsume_link(record)
}

fn sign_uuid(id: Uuid) -> Result<String> {
    let uuid_b64 = BASE64_URL_SAFE_NO_PAD.encode(id.as_bytes());

    let mut mac = HmacSha256::new_from_slice(CONFIG.session_secret.expose_secret().as_bytes())?;
    mac.update(uuid_b64.as_bytes());
    let signature = BASE64_URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    Ok(format!("{}.{}", uuid_b64, signature))
}

fn extract_uuid(signed_token: &str) -> Result<Uuid> {
    let parts: Vec<&str> = signed_token.split('.').collect();
    if parts.len() != 2 {
        return Err(err!("invalid token format").into());
    }

    let uuid_b64 = parts[0];
    let signature_b64 = parts[1];

    let mut mac = HmacSha256::new_from_slice(CONFIG.session_secret.expose_secret().as_bytes())?;
    mac.update(uuid_b64.as_bytes());

    let signature = BASE64_URL_SAFE_NO_PAD.decode(signature_b64)?;

    mac.verify_slice(&signature)?;

    // Decode UUID
    let uuid_bytes = BASE64_URL_SAFE_NO_PAD.decode(uuid_b64)?;

    let uuid_bytes: [u8; 16] = uuid_bytes
        .try_into()
        .map_err(|_| err!("invalid token length"))?;

    Ok(Uuid::from_bytes(uuid_bytes))
}
