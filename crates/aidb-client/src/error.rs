//! Errors returned by the AI-DB client.
//!
//! The variants follow what a caller has to decide on (retry, fix the
//! credentials, fix the input, report a bug), so the CLI can map them to
//! stable exit codes.

use std::path::PathBuf;
use std::time::Duration;

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

    /// A local file to upload could not be read.
    #[error("cannot read {}", path.display())]
    File {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// [`crate::Client::wait_for_job`] gave up.
    #[error("job {job_id} did not finish within {}s", waited.as_secs())]
    WaitTimeout { job_id: String, waited: Duration },
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

    /// `true` for failures that may go away on their own: the server is
    /// unreachable, too slow, rate limiting (429) or failing (5xx).
    pub fn is_temporary(&self) -> bool {
        matches!(self, Error::Unreachable { .. } | Error::Timeout { .. })
            || matches!(self.status(), Some(429 | 500..=599))
    }

    /// HTTP status of an [`Error::Api`], `None` for all other variants.
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Api { status, .. } => Some(*status),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporary_errors_are_rate_limits_and_server_failures() {
        let api = |status| Error::Api {
            status,
            detail: String::new(),
        };
        assert!(api(429).is_temporary());
        assert!(api(503).is_temporary());
        assert!(!api(404).is_temporary());
        assert!(!api(403).is_temporary());
    }
}
