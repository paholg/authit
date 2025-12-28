use std::fmt;

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

impl From<Error> for anyhow::Error {
    fn from(value: Error) -> Self {
        value.inner
    }
}

#[cfg(feature = "server")]
impl Error {
    /// Convert to a rich ServerFnError with full error chain and backtrace.
    /// Only use this for authenticated requests where exposing details is safe.
    pub fn into_rich_server_error(self) -> dioxus::server::ServerFnError {
        let mut chain: Vec<String> = Vec::new();
        chain.push(self.inner.to_string());
        let mut source = std::error::Error::source(&*self.inner);
        while let Some(err) = source {
            chain.push(err.to_string());
            source = err.source();
        }

        let backtrace = self.inner.backtrace().to_string();
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

#[cfg(feature = "server")]
impl From<Error> for dioxus::server::ServerFnError {
    fn from(value: Error) -> Self {
        // Default: return minimal error info for unauthenticated requests
        dioxus::server::ServerFnError::ServerError {
            message: value.inner.to_string(),
            code: 500,
            details: None,
        }
    }
}

