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
        let api = aidb_client::Error::Api {
            status: 404,
            detail: String::new(),
        };
        assert_eq!(CliError::from(api).exit_code(), exit::NOT_FOUND);
    }
}
