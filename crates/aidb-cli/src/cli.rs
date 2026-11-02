//! Command-line interface definition (clap).

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Command-line client for AI-DB – hash-based annotation of bacterial proteins
#[derive(Debug, Parser)]
#[command(name = "aidb", version, propagate_version = true)]
pub struct Cli {
    /// AI-DB server URL
    #[arg(
        long,
        global = true,
        env = "AIDB_SERVER",
        default_value = aidb_client::DEFAULT_SERVER,
        value_name = "URL"
    )]
    pub server: String,

    /// Output format: tables for people, JSON for scripts
    #[arg(
        long,
        short = 'o',
        global = true,
        env = "AIDB_OUTPUT",
        value_enum,
        default_value_t = OutputFormat::Human
    )]
    pub output: OutputFormat,

    /// Timeout for a single request, in seconds
    #[arg(long, global = true, default_value_t = 30, value_name = "SECS")]
    pub timeout: u64,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text (not meant for parsing)
    Human,
    /// JSON on stdout, stable for scripts
    Json,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check the server: health, version and compatibility with this CLI
    Status,

    /// Information about the annotation databases
    #[command(subcommand)]
    Db(DbCommand),

    /// Manage the access token that identifies your jobs
    #[command(subcommand)]
    Auth(AuthCommand),
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Create a new access token for this server and save it
    Init {
        /// Replace a token that is already saved for this server
        #[arg(long)]
        force: bool,
    },

    /// Save an existing token, e.g. the one from the web interface
    ///
    /// Copy it in the web interface (Jobs → Access token) and paste it when
    /// asked, pipe it in (`pbpaste | aidb auth import`), or pass the downloaded
    /// file with --file. The token is never taken as an argument, so it does
    /// not end up in your shell history.
    Import {
        /// Read the token from a file, e.g. aidb-token.env from the web interface
        #[arg(long, value_name = "PATH")]
        file: Option<PathBuf>,

        /// Replace a different token that is already saved for this server
        #[arg(long)]
        force: bool,
    },

    /// Show the token in use and where it comes from
    Show {
        /// Print the full token instead of a masked one (e.g. for a CI secret)
        #[arg(long)]
        reveal: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    /// Bakta DB release and size, AI-DB annotations DB status
    Info,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn global_flags_work_after_the_subcommand() {
        let cli = Cli::try_parse_from([
            "aidb",
            "status",
            "-o",
            "json",
            "--server",
            "http://localhost:8000",
        ])
        .unwrap();
        assert_eq!(cli.output, OutputFormat::Json);
        assert_eq!(cli.server, "http://localhost:8000");
        assert!(matches!(cli.command, Command::Status));
    }

    #[test]
    fn auth_commands_parse() {
        let cli = Cli::try_parse_from([
            "aidb",
            "auth",
            "import",
            "--file",
            "aidb-token.env",
            "--force",
        ])
        .unwrap();
        match cli.command {
            Command::Auth(AuthCommand::Import { file, force }) => {
                assert_eq!(file, Some(PathBuf::from("aidb-token.env")));
                assert!(force);
            }
            other => panic!("unexpected {other:?}"),
        }
        let cli = Cli::try_parse_from(["aidb", "auth", "show", "--reveal"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Auth(AuthCommand::Show { reveal: true })
        ));
    }

    #[test]
    fn import_never_takes_the_token_as_an_argument() {
        let token = "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c";
        assert!(Cli::try_parse_from(["aidb", "auth", "import", token]).is_err());
    }

    #[test]
    fn db_info_is_a_nested_command() {
        let cli = Cli::try_parse_from(["aidb", "db", "info"]).unwrap();
        assert!(matches!(cli.command, Command::Db(DbCommand::Info)));
    }
}
