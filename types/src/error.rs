use serde::{Deserialize, Serialize};

/// A serializable error for client rendering.
///
/// When `RUST_BACKTRACE=1` is set, the message will include the full backtrace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    /// The error message (includes chain and backtrace from eyre's Debug output)
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl From<eyre::Report> for Error {
    fn from(report: eyre::Report) -> Self {
        // The Debug representation includes the error chain and backtrace
        Self {
            message: format!("{:?}", report),
        }
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Self {
            message: s.to_string(),
        }
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self { message: s }
    }
}
