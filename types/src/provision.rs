use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct ProvisionToken {
    token: String,
}

impl ProvisionToken {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    pub fn as_str(&self) -> &str {
        &self.token
    }
}
