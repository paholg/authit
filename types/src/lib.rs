pub mod kanidm;
mod provision;
mod reset_link;
mod session;

use std::fmt;

pub use provision::{ProvisionLinkInfo, ProvisionRecord};
pub use reset_link::ResetLink;
pub use session::{SESSION_COOKIE_NAME, UserSession, decode_session, encode_session};

pub use anyhow::anyhow as internal_anyhow_dont_use;

#[macro_export]
macro_rules! err {
    ($($a:tt)*) => {
        $crate::Error::new($crate::internal_anyhow_dont_use!($($a)*))
    };
}

pub type Result<T> = std::result::Result<T, Error>;

/// A simple wrapper around anyhow to provide richer errors to the client.
///
/// It's probably not worth doing this way.
pub struct Error {
    inner: anyhow::Error,
}

impl Error {
    pub fn new(err: impl Into<anyhow::Error>) -> Self {
        Self { inner: err.into() }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<E: core::error::Error + Send + Sync + 'static> From<E> for Error {
    #[track_caller]
    fn from(value: E) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

#[cfg(feature = "server")]
impl From<Error> for dioxus::server::ServerFnError {
    fn from(value: Error) -> Self {
        // Build the error chain
        let mut chain: Vec<String> = Vec::new();
        chain.push(value.inner.to_string());
        let mut source = std::error::Error::source(&*value.inner);
        while let Some(err) = source {
            chain.push(err.to_string());
            source = err.source();
        }

        // Capture backtrace - will be empty if RUST_BACKTRACE is not set
        let backtrace = value.inner.backtrace().to_string();
        let backtrace = if backtrace.is_empty() || backtrace == "disabled backtrace" {
            None
        } else {
            Some(backtrace)
        };

        dioxus::server::ServerFnError::ServerError {
            message: chain.first().cloned().unwrap_or_default(),
            code: 500,
            details: Some(serde_json::json!({
                "chain": chain,
                "backtrace": backtrace,
            })),
        }
    }
}
