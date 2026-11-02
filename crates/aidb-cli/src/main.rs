//! ============================================================================
//! aidb – command-line client for AI-DB
//!
//! Commands so far: `status`, `db info`. Job submission, waiting and download
//! follow in the next steps of the CLI concept (v0.1). Input will be
//! restricted to protein FASTA, matching the published scope.
//! ============================================================================

mod cli;
mod commands;
mod exit;
mod output;

use std::error::Error as _;
use std::process::ExitCode;
use std::time::Duration;

use aidb_client::Client;
use clap::Parser;

use crate::cli::{Cli, Command, DbCommand};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse(); // exits with code 2 on invalid arguments

    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report(&error);
            ExitCode::from(exit::code(&error))
        }
    }
}

async fn run(cli: Cli) -> aidb_client::Result<()> {
    let client = Client::builder()
        .server(cli.server)
        .timeout(Duration::from_secs(cli.timeout.max(1)))
        .user_agent(concat!("aidb-cli/", env!("CARGO_PKG_VERSION")))
        .build()?;

    match cli.command {
        Command::Status => commands::status::run(&client, cli.output).await,
        Command::Db(DbCommand::Info) => commands::db::info(&client, cli.output).await,
    }
}

/// Error message plus its first cause on stderr.
fn report(error: &aidb_client::Error) {
    eprintln!("error: {error}");
    if let Some(cause) = error.source() {
        eprintln!("  cause: {cause}");
    }
}
