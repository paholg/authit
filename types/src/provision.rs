use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A provision token for self-service user registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionToken {
    /// Unique identifier for tracking token usage (UUIDv7)
    pub id: Uuid,
    /// Unix timestamp (seconds) when this token expires
    pub expires_at: u64,
}

impl ProvisionToken {
    pub fn new(id: Uuid, duration_seconds: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            id,
            expires_at: now + duration_seconds,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.expires_at
    }
}
