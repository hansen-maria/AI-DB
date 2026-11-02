//! Errors returned by the AI-DB client.
//!
//! The variants follow what a caller has to decide on (retry, fix the
//! credentials, fix the input, report a bug), so the CLI can map them to
//! stable exit codes.

/// Result type of all client calls.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The configured server URL cannot be used.
    #[error("invalid server URL '{url}': {reason}")]
    InvalidUrl { url: String, reason: String },

    /// No connection could be established (DNS, refused, TLS, …).
    #[error("cannot reach the AI-DB server at {url}")]
    Unreachable {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    /// The server did not answer within the timeout.
    #[error("the AI-DB server at {url} did not respond in time")]
    Timeout {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    /// The server answered with an error status. `detail` is the text of the
    /// server's `ErrorResponse`, or the raw body if it was not JSON.
    #[error("{detail} (HTTP {status})")]
    Api { status: u16, detail: String },

    /// The server answered 2xx, but the body was not what this client expects
    /// (usually an incompatible server version).
    #[error("unexpected response from the AI-DB server")]
    Decode(#[source] reqwest::Error),

    /// Any other transport error.
    #[error("request to the AI-DB server failed")]
    Http(#[source] reqwest::Error),
}

impl Error {
    /// Classifies a transport error of a request to `url`.
    pub(crate) fn from_reqwest(url: &str, source: reqwest::Error) -> Self {
        if source.is_timeout() {
            Error::Timeout {
                url: url.to_string(),
                source,
            }
        } else if source.is_connect() {
            Error::Unreachable {
                url: url.to_string(),
                source,
            }
        } else if source.is_decode() {
            Error::Decode(source)
        } else {
            Error::Http(source)
        }
    }

    /// HTTP status of an [`Error::Api`], `None` for all other variants.
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Api { status, .. } => Some(*status),
            _ => None,
        }
    }
}
