//! `aidb auth init|import|show` – the access token that identifies your jobs.
//!
//! The token is a random UUIDv4 that the server knows only as a hash. It is
//! the only key to the jobs created with it, so these commands never print it
//! in full unless asked to (`show --reveal`) and never take it as a command
//! line argument (because it would end up in the shell history).

use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};

use serde::Serialize;
use uuid::Uuid;

use crate::cli::OutputFormat;
use crate::credentials::Credentials;
use crate::error::{CliError, Result};
use crate::output::{print_json, print_rows, warn};
use crate::token::{extract_token, mask, parse_token, resolve, TokenSource};

pub struct AuthContext {
    /// Normalised server URL (key in the credentials file)
    pub server: String,
    pub credentials_path: PathBuf,
    /// Value of `AIDB_TOKEN`, if set
    pub env_token: Option<String>,
    pub output: OutputFormat,
}

impl AuthContext {
    fn env_token_set(&self) -> bool {
        self.env_token
            .as_deref()
            .is_some_and(|t| !t.trim().is_empty())
    }
}

/// JSON shape of `init` and `import`.
#[derive(Serialize)]
struct Saved<'a> {
    server: &'a str,
    saved_to: &'a Path,
    /// "created" | "imported" | "unchanged"
    result: &'a str,
}

// ── init ─────────────────────────────────────────────────────────────────────

pub fn init(ctx: &AuthContext, force: bool) -> Result<()> {
    let mut credentials = Credentials::load(&ctx.credentials_path)?;
    if credentials.get(&ctx.server).is_some() && !force {
        return Err(already_saved(ctx));
    }

    let token = Uuid::new_v4().to_string();
    credentials.set(&ctx.server, token.clone());
    credentials.save(&ctx.credentials_path)?;

    match ctx.output {
        OutputFormat::Json => print_json(&Saved {
            server: &ctx.server,
            saved_to: &ctx.credentials_path,
            result: "created",
        }),
        OutputFormat::Human => {
            println!("Created a new access token for {}", ctx.server);
            print_rows(&[
                ("  Token", mask(&token)),
                ("  Saved to", ctx.credentials_path.display().to_string()),
            ]);
            println!();
            println!("Keep a copy: the token is the only key to the jobs you create with it.");
            println!("  aidb auth show --reveal");
            println!();
            println!("Already using AI-DB in your browser? Import that token instead, so web");
            println!("and CLI jobs belong together (web interface: Jobs → Access token):");
            println!("  aidb auth import --force");
        }
    }
    warn_if_env_overrides(ctx);
    Ok(())
}

// ── import ───────────────────────────────────────────────────────────────────

pub fn import(ctx: &AuthContext, file: Option<&Path>, force: bool) -> Result<()> {
    let input = match file {
        Some(path) => fs::read_to_string(path)
            .map_err(|e| CliError::io(format!("cannot read {}", path.display()), e))?,
        None => read_token_input()?,
    };
    let token = extract_token(&input)?;

    let mut credentials = Credentials::load(&ctx.credentials_path)?;
    let existing = credentials.get(&ctx.server).map(str::to_owned);
    let result = match existing.as_deref() {
        Some(existing) if parse_token(existing).as_deref() == Some(token.as_str()) => "unchanged",
        Some(_) if !force => return Err(already_saved(ctx)),
        _ => {
            credentials.set(&ctx.server, token.clone());
            credentials.save(&ctx.credentials_path)?;
            "imported"
        }
    };

    match ctx.output {
        OutputFormat::Json => print_json(&Saved {
            server: &ctx.server,
            saved_to: &ctx.credentials_path,
            result,
        }),
        OutputFormat::Human => {
            if result == "unchanged" {
                println!("This token is already saved for {}", ctx.server);
            } else {
                println!("Saved the access token for {}", ctx.server);
            }
            print_rows(&[
                ("  Token", mask(&token)),
                ("  Saved to", ctx.credentials_path.display().to_string()),
            ]);
        }
    }
    warn_if_env_overrides(ctx);
    Ok(())
}

/// Reads the token without echoing it on a terminal, or from a pipe.
fn read_token_input() -> Result<String> {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        eprintln!("Paste your access token (web interface: Jobs → Access token → Copy).");
        rpassword::prompt_password("Token (input hidden): ")
            .map_err(|e| CliError::io("cannot read the token from the terminal", e))
    } else {
        let mut input = String::new();
        stdin
            .lock()
            .read_to_string(&mut input)
            .map_err(|e| CliError::io("cannot read the token from standard input", e))?;
        Ok(input)
    }
}

// ── show ─────────────────────────────────────────────────────────────────────

/// JSON shape of `show`.
#[derive(Serialize)]
struct Shown<'a> {
    server: &'a str,
    token: &'a str,
    masked: bool,
    /// "env" | "file"
    source: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<&'a Path>,
}

pub fn show(ctx: &AuthContext, reveal: bool) -> Result<()> {
    let resolved = resolve(&ctx.server, ctx.env_token.clone(), &ctx.credentials_path)?
        .ok_or_else(|| CliError::NoToken(ctx.server.clone()))?;
    let token = if reveal {
        resolved.token.clone()
    } else {
        mask(&resolved.token)
    };
    let (source, path) = match &resolved.source {
        TokenSource::Env => ("env", None),
        TokenSource::File(path) => ("file", Some(path.as_path())),
    };

    match ctx.output {
        OutputFormat::Json => print_json(&Shown {
            server: &ctx.server,
            token: &token,
            masked: !reveal,
            source,
            path,
        }),
        OutputFormat::Human => {
            let source_text = match path {
                Some(path) => path.display().to_string(),
                None => "environment variable AIDB_TOKEN".to_string(),
            };
            print_rows(&[
                ("Server", ctx.server.clone()),
                ("Token", token),
                ("Source", source_text),
            ]);
            if !reveal {
                eprintln!("Show the full token with --reveal.");
            }
        }
    }
    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn already_saved(ctx: &AuthContext) -> CliError {
    CliError::Usage(format!(
        "a token for {} is already saved in {}\n  \
         Replacing it makes the jobs created with it unreachable unless you keep a copy\n  \
         (aidb auth show --reveal). Add --force to replace it anyway.",
        ctx.server,
        ctx.credentials_path.display()
    ))
}

fn warn_if_env_overrides(ctx: &AuthContext) {
    if ctx.env_token_set() {
        warn("AIDB_TOKEN is set and takes precedence over the saved token");
    }
}
