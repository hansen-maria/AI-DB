//! Command-line interface definition (clap).

use std::path::PathBuf;
use std::time::Duration;

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

    /// Timeout for a single request, in seconds (uploads may take longer)
    #[arg(long, global = true, default_value_t = 30, value_name = "SECS")]
    pub timeout: u64,

    /// Only essential output (e.g. job IDs), no progress or hints
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,

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

    /// Submit protein FASTA files as new annotation jobs
    ///
    /// Each file becomes one job. Plain and gzipped FASTA (.faa, .fasta,
    /// .faa.gz, …) up to 100 MB are accepted; nucleotide FASTA is rejected,
    /// translate the genes first. Prints the job IDs.
    Submit {
        /// Protein FASTA files (plain or gzipped)
        #[arg(required = true, value_name = "FASTA")]
        files: Vec<PathBuf>,

        /// Job name shown in the job list (default: the file name); only
        /// with a single file
        #[arg(long, short = 'n')]
        name: Option<String>,
    },

    /// Inspect and wait for jobs
    #[command(subcommand)]
    Job(JobCommand),
}

#[derive(Debug, Subcommand)]
pub enum JobCommand {
    /// Status and match counts of a job
    Show {
        /// Job ID or the job's web link
        #[arg(value_name = "JOB", value_parser = parse_job_id)]
        job_id: String,
    },

    /// Wait until jobs are finished, showing progress
    ///
    /// Exits with 0 when all jobs completed, 5 when all failed, 9 when some
    /// failed and 6 when --max-wait is exceeded. Interrupting only stops
    /// waiting; the jobs keep running on the server.
    Wait {
        /// Job IDs or web links
        #[arg(required = true, value_name = "JOB", value_parser = parse_job_id)]
        job_ids: Vec<String>,

        /// Give up after this long, e.g. 90s, 30m or 2h (default: no limit)
        #[arg(long, value_name = "DURATION", value_parser = parse_duration)]
        max_wait: Option<Duration>,
    },
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

/// Accepts a job ID or a web link ending in it (`…/job/<id>`).
pub fn parse_job_id(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    let candidate = raw
        .split(['?', '#'])
        .next()
        .unwrap_or(raw)
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(raw);
    uuid::Uuid::parse_str(candidate)
        .map(|id| id.to_string())
        .map_err(|_| format!("'{raw}' is not a job ID (expected a UUID or a job link)"))
}

/// Parses `90`, `90s`, `30m`, `2h`.
pub fn parse_duration(raw: &str) -> Result<Duration, String> {
    let raw = raw.trim();
    let (number, factor) = match raw.char_indices().last() {
        Some((i, 's')) => (&raw[..i], 1),
        Some((i, 'm')) => (&raw[..i], 60),
        Some((i, 'h')) => (&raw[..i], 3600),
        _ => (raw, 1),
    };
    number
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
        .map(|n| Duration::from_secs(n * factor))
        .ok_or_else(|| format!("'{raw}' is not a duration (e.g. 90s, 30m, 2h)"))
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
    fn job_ids_and_links_are_accepted() {
        let id = "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c";
        assert_eq!(parse_job_id(id).unwrap(), id);
        assert_eq!(parse_job_id(&id.to_uppercase()).unwrap(), id);
        for link in [
            format!("https://ai-db.computational.bio/job/{id}"),
            format!("https://ai-db.computational.bio/job/{id}/"),
            format!("https://ai-db.computational.bio/job/{id}?tab=analysis#top"),
        ] {
            assert_eq!(parse_job_id(&link).unwrap(), id, "{link}");
        }
        assert!(parse_job_id("abc").is_err());
        assert!(parse_job_id("https://ai-db.computational.bio/jobs").is_err());
    }

    #[test]
    fn durations_parse() {
        assert_eq!(parse_duration("90").unwrap(), Duration::from_secs(90));
        assert_eq!(parse_duration("90s").unwrap(), Duration::from_secs(90));
        assert_eq!(parse_duration("30m").unwrap(), Duration::from_secs(1800));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
        for bad in ["", "0", "-5m", "1.5h", "soon", "h"] {
            assert!(parse_duration(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn submit_requires_files_and_wait_parses_options() {
        assert!(Cli::try_parse_from(["aidb", "submit"]).is_err());
        let cli = Cli::try_parse_from([
            "aidb",
            "job",
            "wait",
            "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c",
            "--max-wait",
            "30m",
            "-q",
        ])
        .unwrap();
        assert!(cli.quiet);
        match cli.command {
            Command::Job(JobCommand::Wait { job_ids, max_wait }) => {
                assert_eq!(job_ids.len(), 1);
                assert_eq!(max_wait, Some(Duration::from_secs(1800)));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(Cli::try_parse_from(["aidb", "job", "show", "not-a-job"]).is_err());
    }

    #[test]
    fn db_info_is_a_nested_command() {
        let cli = Cli::try_parse_from(["aidb", "db", "info"]).unwrap();
        assert!(matches!(cli.command, Command::Db(DbCommand::Info)));
    }
}
