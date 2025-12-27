mod config;
mod kanidm;
pub mod storage;

pub use config::Config;
pub use kanidm::KanidmClient;

use base64::prelude::*;
use eyre::{WrapErr, eyre};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use sha2::Sha256;
use types::{
    Error, ProvisionLinkInfo, ProvisionRecord, SESSION_COOKIE_NAME, UserSession, decode_session,
};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Initialize the server, including storage.
/// Must be called before using provision link functions.
pub fn init() -> Result<(), Error> {
    let config = Config::from_env()?;
    let db_path = config.data_dir.join("provision.redb");
    storage::init_storage(&db_path).map_err(Error::from)
}

/// Get the base URL from the current request (e.g., "https://example.com")
pub async fn get_request_base_url() -> Result<String, Error> {
    use axum::http::HeaderMap;
    use dioxus::fullstack::FullstackContext;

    let headers: HeaderMap = FullstackContext::extract()
        .await
        .wrap_err("failed to extract headers")?;

    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| eyre!("no host header in request"))?;

    // Use X-Forwarded-Proto if set (by reverse proxy), otherwise assume http
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");

    Ok(format!("{}://{}", proto, host))
}

/// Extract the user session from the request cookie.
pub async fn get_session_from_cookie() -> Result<UserSession, Error> {
    use axum::http::HeaderMap;
    use dioxus::fullstack::FullstackContext;

    let headers: HeaderMap = FullstackContext::extract()
        .await
        .wrap_err("failed to extract headers")?;

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

/// Create a provision link and persist it.
/// Returns a signed token string.
pub fn create_provision_link(duration_hours: u32, max_uses: Option<u32>) -> Result<String, Error> {
    let config = Config::from_env()?;

    // Create and persist the link
    let record = storage::storage()?
        .create_link(duration_hours as u64 * 3600, max_uses)
        .map_err(Error::from)?;

    // Sign the UUID for URL tamper-resistance
    sign_uuid(record.id, &config)
}

/// Verify a provision link without consuming it.
/// Returns link info if valid.
pub fn verify_provision_link(signed_token: &str) -> Result<ProvisionLinkInfo, Error> {
    let config = Config::from_env()?;
    let uuid = extract_uuid(signed_token, &config)?;

    storage::storage()?.verify_link(uuid).map_err(Error::from)
}

/// Consume a provision link (increment use count).
/// Returns the record for potential rollback, error if expired/exhausted.
pub fn consume_provision_link(signed_token: &str) -> Result<ProvisionRecord, Error> {
    let config = Config::from_env()?;
    let uuid = extract_uuid(signed_token, &config)?;

    storage::storage()?.consume_link(uuid).map_err(Error::from)
}

/// Restore a consumed provision link (e.g., if user creation failed).
pub fn unconsume_provision_link(record: ProvisionRecord) -> Result<(), Error> {
    storage::storage()?
        .unconsume_link(record)
        .map_err(Error::from)
}

/// Sign a UUID to create a tamper-resistant token.
fn sign_uuid(id: Uuid, config: &Config) -> Result<String, Error> {
    let uuid_b64 = BASE64_URL_SAFE_NO_PAD.encode(id.as_bytes());

    let mut mac = HmacSha256::new_from_slice(config.session_secret.expose_secret().as_bytes())
        .wrap_err("invalid secret key")?;
    mac.update(uuid_b64.as_bytes());
    let signature = BASE64_URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    Ok(format!("{}.{}", uuid_b64, signature))
}

/// Extract and verify a UUID from a signed token.
fn extract_uuid(signed_token: &str, config: &Config) -> Result<Uuid, Error> {
    let parts: Vec<&str> = signed_token.split('.').collect();
    if parts.len() != 2 {
        return Err(eyre!("invalid token format").into());
    }

    let uuid_b64 = parts[0];
    let signature_b64 = parts[1];

    // Verify signature
    let mut mac = HmacSha256::new_from_slice(config.session_secret.expose_secret().as_bytes())
        .wrap_err("invalid secret key")?;
    mac.update(uuid_b64.as_bytes());

    let signature = BASE64_URL_SAFE_NO_PAD
        .decode(signature_b64)
        .wrap_err("invalid signature encoding")?;

    mac.verify_slice(&signature)
        .map_err(|_| eyre!("invalid token signature"))?;

    // Decode UUID
    let uuid_bytes = BASE64_URL_SAFE_NO_PAD
        .decode(uuid_b64)
        .wrap_err("invalid token encoding")?;

    let uuid_bytes: [u8; 16] = uuid_bytes
        .try_into()
        .map_err(|_| eyre!("invalid token length"))?;

    Ok(Uuid::from_bytes(uuid_bytes))
}
