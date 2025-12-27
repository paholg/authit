use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A provision token for self-service user registration.
/// This is the signed token format sent to clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvisionToken {
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

/// A persisted provision link record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvisionRecord {
    /// Unique identifier (UUIDv7)
    pub id: Uuid,
    /// Unix timestamp (seconds) when this link expires
    pub expires_at: u64,
    /// Maximum number of times this link can be used (None = unlimited)
    pub max_uses: Option<u32>,
    /// Current number of times this link has been used
    pub use_count: u32,
    /// Unix timestamp when this link was created
    pub created_at: u64,
}

impl ProvisionRecord {
    pub fn new(id: Uuid, duration_seconds: u64, max_uses: Option<u32>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            id,
            expires_at: now + duration_seconds,
            max_uses,
            use_count: 0,
            created_at: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.expires_at
    }

    pub fn is_exhausted(&self) -> bool {
        self.max_uses.map(|m| self.use_count >= m).unwrap_or(false)
    }

    pub fn uses_remaining(&self) -> Option<u32> {
        self.max_uses.map(|m| m.saturating_sub(self.use_count))
    }
}

/// Information about a provision link returned to the client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvisionLinkInfo {
    pub id: Uuid,
    pub expires_at: u64,
    pub max_uses: Option<u32>,
    pub uses_remaining: Option<u32>,
}

impl From<&ProvisionRecord> for ProvisionLinkInfo {
    fn from(record: &ProvisionRecord) -> Self {
        Self {
            id: record.id,
            expires_at: record.expires_at,
            max_uses: record.max_uses,
            uses_remaining: record.uses_remaining(),
        }
    }
}
