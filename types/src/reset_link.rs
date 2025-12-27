use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetLink {
    pub url: Url,
    pub expires_at: Timestamp,
}
