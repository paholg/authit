use serde::{Deserialize, Serialize};

/// A credential reset link with expiry information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetLink {
    /// The full reset URL
    pub url: String,
    /// Unix timestamp (seconds) when this link expires
    pub expires_at: u64,
}
