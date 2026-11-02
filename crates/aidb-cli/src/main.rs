//! ============================================================================
//! aidb – command-line client for AI-DB
//!
//! Commands so far: `status`, `db info`, `auth init|import|show`, `submit`,
//! `job show|wait`. Input is restricted to protein FASTA, matching the
//! published scope.
//! ============================================================================

mod cli;
mod commands;
mod config;
mod credentials;
mod error;
mod exit;
mod fasta;
mod output;
mod token;

use std::env;
use std::error::Error as _;
use std::process::ExitCode;
use std::time::Duration;

use aidb_client::Client;
use clap::Parser;

use crate::cli::{AuthCommand, Cli, Command, DbCommand, JobCommand};
use crate::commands::auth::AuthContext;
use crate::commands::job::WaitArgs;
use crate::commands::submit::SubmitOptions;
use crate::error::{CliError, Result};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse(); // exits with code 2 on invalid arguments

    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report(&error);
            ExitCode::from(error.exit_code())
        }
    }
}

async fn run(cli: Cli) -> Result<()> {
    let Cli {
        server,
        output,
        timeout,
        quiet,
        command,
    } = cli;
    // Canonical form: the key for saved tokens and the base for all requests
    let server = aidb_client::normalize_server_url(&server)?.to_string();

    let client = |token: Option<String>| -> Result<Client> {
        Ok(Client::builder()
            .server(server.clone())
            .token(token)
            .timeout(Duration::from_secs(timeout.max(1)))
            .user_agent(concat!("aidb-cli/", env!("CARGO_PKG_VERSION")))
            .build()?)
    };

    match command {
        Command::Status => commands::status::run(&client(None)?, output).await?,
        Command::Db(DbCommand::Info) => commands::db::info(&client(None)?, output).await?,
        Command::Auth(auth) => {
            let ctx = AuthContext {
                server: server.clone(),
                credentials_path: config::credentials_path()?,
                env_token: env::var("AIDB_TOKEN").ok(),
                output,
            };
            match auth {
                AuthCommand::Init { force } => commands::auth::init(&ctx, force)?,
                AuthCommand::Import { file, force } => {
                    commands::auth::import(&ctx, file.as_deref(), force)?
                }
                AuthCommand::Show { reveal } => commands::auth::show(&ctx, reveal)?,
            }
        }
        Command::Submit { files, name } => {
            // Without a token the server would create an anonymous owner whose
            // jobs nobody could manage afterwards
            let token = saved_token(&server)?.ok_or_else(|| CliError::NoToken(server.clone()))?;
            let options = SubmitOptions {
                files: &files,
                name: name.as_deref(),
                output,
                quiet,
            };
            commands::submit::run(&client(Some(token))?, options).await?
        }
        Command::Job(JobCommand::Show { job_id }) => {
            // Viewing works without a token; with it, ownership is shown
            let client = client(saved_token(&server)?)?;
            commands::job::show(&client, &job_id, output).await?
        }
        Command::Job(JobCommand::Wait { job_ids, max_wait }) => {
            let client = client(saved_token(&server)?)?;
            let args = WaitArgs {
                job_ids: &job_ids,
                max_wait,
                output,
                quiet,
            };
            commands::job::wait(&client, args).await?
        }
    }
    Ok(())
}

/// Token for `server`: `AIDB_TOKEN`, else the saved one.
fn saved_token(server: &str) -> Result<Option<String>> {
    let resolved = token::resolve(
        server,
        env::var("AIDB_TOKEN").ok(),
        &config::credentials_path()?,
    )?;
    Ok(resolved.map(|r| r.token))
}

/// Error message plus its root cause on stderr.
///
/// Only the innermost cause is shown: for a refused connection that is
/// "Connection refused (os error 61)" instead of four layers of wrappers.
fn report(error: &CliError) {
    eprintln!("error: {error}");
    let mut root = error.source();
    while let Some(next) = root.and_then(|e| e.source()) {
        root = Some(next);
    }
    if let Some(cause) = root {
        let cause = cause.to_string();
        if !error.to_string().contains(&cause) {
            eprintln!("  cause: {cause}");
        }
    }
}
