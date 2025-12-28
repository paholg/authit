use reqwest::Url;
use secrecy::SecretString;
use serde::Deserialize;
use std::env;
use std::path::PathBuf;
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| Config::new().unwrap());

#[derive(Debug, Deserialize)]
pub struct Config {
    pub kanidm_url: Url,
    pub kanidm_token: SecretString,
    pub oauth_client_id: String,
    pub oauth_client_secret: SecretString,
    pub oauth_redirect_uri: String,
    pub session_secret: SecretString,
    pub admin_group: String,
    pub data_dir: PathBuf,
    pub db_secret: SecretString,
}

impl Config {
    fn new() -> types::Result<Self> {
        let cfg = config::Config::builder().add_source(config::Environment::with_prefix("AUTHIT"));

        let cfg = if let Ok(path) = env::var("AUTHIT_CONFIG_PATH") {
            cfg.add_source(config::File::with_name(&path))
        } else {
            cfg
        };

        Ok(cfg.build()?.try_deserialize()?)
    }
}
