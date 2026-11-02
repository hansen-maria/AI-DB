//! Exit codes – stable, so pipelines can react to the kind of failure.
//!
//! | Code | Meaning                                         |
//! |------|-------------------------------------------------|
//! | 0    | success                                         |
//! | 1    | general error                                   |
//! | 2    | wrong usage (invalid arguments, server URL)     |
//! | 3    | not authorized (missing / foreign token)        |
//! | 4    | not found (e.g. job expired after 30 days)      |
//! | 5    | job failed                                      |
//! | 6    | timeout while waiting for a job                 |
//! | 7    | server not reachable or temporarily failing     |
//! | 8    | input rejected (too large, not protein FASTA)   |
//! | 9    | partially successful batch                      |


use aidb_client::Error;

pub const GENERAL: u8 = 1;
pub const USAGE: u8 = 2;
pub const UNAUTHORIZED: u8 = 3;
pub const NOT_FOUND: u8 = 4;
pub const JOB_FAILED: u8 = 5;
pub const WAIT_TIMEOUT: u8 = 6;
pub const UNAVAILABLE: u8 = 7;
pub const INPUT_REJECTED: u8 = 8;
pub const PARTIAL: u8 = 9;

/// Exit code for a client error.
pub fn code(error: &Error) -> u8 {
    match error {
        Error::InvalidUrl { .. } => USAGE,
        Error::Unreachable { .. } | Error::Timeout { .. } => UNAVAILABLE,
        Error::Api { status, .. } => match status {
            401 | 403 => UNAUTHORIZED,
            404 | 410 => NOT_FOUND,
            400 | 413 | 415 | 422 => INPUT_REJECTED,
            429 | 500..=599 => UNAVAILABLE,
            _ => GENERAL,
        },
        Error::File { .. } => INPUT_REJECTED,
        Error::WaitTimeout { .. } => WAIT_TIMEOUT,
        Error::Decode(_) | Error::Http(_) | Error::Write(_) => GENERAL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api(status: u16) -> Error {
        Error::Api {
            status,
            detail: String::new(),
        }
    }

    #[test]
    fn http_status_maps_to_documented_codes() {
        assert_eq!(code(&api(401)), UNAUTHORIZED);
        assert_eq!(code(&api(403)), UNAUTHORIZED);
        assert_eq!(code(&api(404)), NOT_FOUND);
        assert_eq!(code(&api(413)), INPUT_REJECTED);
        assert_eq!(code(&api(429)), UNAVAILABLE);
        assert_eq!(code(&api(503)), UNAVAILABLE);
        assert_eq!(code(&api(418)), GENERAL);
    }

    #[test]
    fn invalid_server_url_is_a_usage_error() {
        let err = Error::InvalidUrl {
            url: "ftp://x".into(),
            reason: String::new(),
        };
        assert_eq!(code(&err), USAGE);
    }
}
