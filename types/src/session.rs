use eyre::{Result, WrapErr};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

pub const SESSION_COOKIE_NAME: &str = "authit_session";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub groups: Vec<String>,
    #[serde(with = "secret_string")]
    pub access_token: SecretString,
}

impl UserSession {
    pub fn is_in_group(&self, group: &str) -> bool {
        self.groups.iter().any(|g| g == group)
    }
}

mod secret_string {
    use secrecy::SecretString;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(secret: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use secrecy::ExposeSecret;
        serializer.serialize_str(secret.expose_secret())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s.into())
    }
}

pub fn encode_session(session: &UserSession) -> Result<String> {
    let json = serde_json::to_string(session).wrap_err("failed to serialize session")?;
    use base64::Engine;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json.as_bytes()))
}

pub fn decode_session(encoded: &str) -> Result<UserSession> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .wrap_err("failed to decode base64")?;
    let json = String::from_utf8(bytes).wrap_err("invalid UTF-8 in session")?;
    serde_json::from_str(&json).wrap_err("failed to parse session JSON")
}
