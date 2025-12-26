use eyre::{eyre, Result};
use secrecy::SecretString;
use std::env;

#[derive(Clone)]
pub struct Config {
    pub kanidm_url: String,
    pub kanidm_token: SecretString,
    pub oauth_client_id: String,
    pub oauth_client_secret: SecretString,
    pub oauth_redirect_uri: String,
    pub session_secret: SecretString,
    pub admin_group: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            kanidm_url: env_var("AUTHIT_KANIDM_URL")?,
            kanidm_token: env_var("AUTHIT_KANIDM_TOKEN")?.into(),
            oauth_client_id: env_var("AUTHIT_OAUTH_CLIENT_ID")?,
            oauth_client_secret: env_var("AUTHIT_OAUTH_CLIENT_SECRET")?.into(),
            oauth_redirect_uri: env_var("AUTHIT_OAUTH_REDIRECT_URI")?,
            session_secret: env_var("AUTHIT_SESSION_SECRET")?.into(),
            admin_group: env::var("AUTHIT_ADMIN_GROUP").unwrap_or_else(|_| "authit_admin".into()),
        })
    }
}

fn env_var(name: &str) -> Result<String> {
    env::var(name).map_err(|_| eyre!("missing environment variable: {}", name))
}
