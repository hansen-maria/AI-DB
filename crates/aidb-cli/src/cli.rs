//! Command-line interface definition (clap).

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
    fn db_info_is_a_nested_command() {
        let cli = Cli::try_parse_from(["aidb", "db", "info"]).unwrap();
        assert!(matches!(cli.command, Command::Db(DbCommand::Info)));
    }
}
