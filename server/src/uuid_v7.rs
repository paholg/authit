use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use jiff::Timestamp;
use secrecy::ExposeSecret;
use sha2::Sha256;
use types::{Result, err};
use uuid::Uuid;

use crate::CONFIG;

type HmacSha256 = Hmac<Sha256>;

pub trait UuidV7Ext: Sized {
    fn from_token(token: &str) -> Result<Self>;
    fn as_token(&self) -> Result<String>;

    fn jiff_timestamp(&self) -> Timestamp;
}

impl UuidV7Ext for Uuid {
    fn from_token(token: &str) -> Result<Self> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 2 {
            return Err(err!("invalid token format"));
        }

        let uuid_simple = parts[0];
        let signature_b64 = parts[1];

        // Verify HMAC signature
        let mut mac = HmacSha256::new_from_slice(CONFIG.signing_secret.expose_secret().as_bytes())?;
        mac.update(uuid_simple.as_bytes());
        let signature = BASE64_URL_SAFE_NO_PAD.decode(signature_b64)?;
        mac.verify_slice(&signature)?;

        let uuid = Uuid::parse_str(uuid_simple)?;
        Ok(uuid)
    }

    fn as_token(&self) -> Result<String> {
        let id_str = self.simple().to_string();
        let mut mac = HmacSha256::new_from_slice(CONFIG.signing_secret.expose_secret().as_bytes())?;
        mac.update(id_str.as_bytes());
        let signature = BASE64_URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        Ok(format!("{}.{}", id_str, signature))
    }

    fn jiff_timestamp(&self) -> Timestamp {
        let ts = self.get_timestamp().unwrap();

        let (seconds, nanos) = ts.to_unix();
        Timestamp::new(seconds as i64, nanos as i32).unwrap()
    }
}
