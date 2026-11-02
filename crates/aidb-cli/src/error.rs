//! Errors of the CLI and their exit codes.

use crate::exit;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// Error talking to the server.
    #[error(transparent)]
    Client(#[from] aidb_client::Error),

    /// The command cannot run as invoked; the message says what to change.
    #[error("{0}")]
    Usage(String),

    /// No access token is available for the server.
    #[error(
        "no access token for {0}\n  \
         import the one from the web interface (Jobs → Access token):  aidb auth import\n  \
         or create a new one:                                         aidb auth init"
    )]
    NoToken(String),

    /// Reading or writing a local file failed.
    #[error("{message}")]
    Io {
        message: String,
        #[source]
        source: std::io::Error,
    },

    /// A local configuration file is invalid.
    #[error("{0}")]
    Config(String),

    /// Input rejected before it was sent (not protein FASTA, too large, …).
    #[error("{0}")]
    Input(String),

    /// Some or all jobs of a batch failed (details were printed already).
    #[error("{failed} of {total} jobs failed")]
    JobsFailed { failed: usize, total: usize },

    /// Some files of a batch could not be submitted (the others were).
    #[error("{failed} of {total} files could not be submitted")]
    SubmissionsFailed { failed: usize, total: usize },
}

impl CliError {
    pub fn io(message: impl Into<String>, source: std::io::Error) -> Self {
        CliError::Io {
            message: message.into(),
            source,
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            CliError::Client(error) => exit::code(error),
            CliError::Usage(_) => exit::USAGE,
            CliError::NoToken(_) => exit::UNAUTHORIZED,
            CliError::Io { .. } | CliError::Config(_) => exit::GENERAL,
            CliError::Input(_) => exit::INPUT_REJECTED,
            CliError::JobsFailed { failed, total } if failed == total => exit::JOB_FAILED,
            CliError::JobsFailed { .. } | CliError::SubmissionsFailed { .. } => exit::PARTIAL,
        }
    }
}

pub type Result<T> = std::result::Result<T, CliError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_follow_the_error_kind() {
        assert_eq!(CliError::Usage(String::new()).exit_code(), exit::USAGE);
        assert_eq!(
            CliError::NoToken("x".into()).exit_code(),
            exit::UNAUTHORIZED
        );
        assert_eq!(CliError::Config(String::new()).exit_code(), exit::GENERAL);
        assert_eq!(
            CliError::Input(String::new()).exit_code(),
            exit::INPUT_REJECTED
        );
        let all = CliError::JobsFailed {
            failed: 2,
            total: 2,
        };
        assert_eq!(all.exit_code(), exit::JOB_FAILED);
        let some = CliError::JobsFailed {
            failed: 1,
            total: 3,
        };
        assert_eq!(some.exit_code(), exit::PARTIAL);
        let api = aidb_client::Error::Api {
            status: 404,
            detail: String::new(),
        };
        assert_eq!(CliError::from(api).exit_code(), exit::NOT_FOUND);
    }
}
