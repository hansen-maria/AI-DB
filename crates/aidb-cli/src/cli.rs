//! Command-line interface definition (clap).

use std::path::PathBuf;
use std::time::Duration;

use aidb_client::types::SequenceFilterQuery;
use aidb_client::ResultFormat;
use clap::{Args, Parser, Subcommand, ValueEnum};

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

    /// List your jobs (newest first)
    Jobs {
        /// Only jobs with this status
        #[arg(long, value_enum)]
        status: Option<StatusFilter>,

        /// Only jobs whose name contains this text (case-insensitive)
        #[arg(long)]
        search: Option<String>,

        /// Show at most this many jobs
        #[arg(long, default_value_t = 20, conflicts_with = "all")]
        limit: usize,

        /// Show all jobs
        #[arg(long)]
        all: bool,
    },

    /// Download the results of a completed job
    ///
    /// Saves one file per format, named after the job, e.g.
    /// proteins_annotations.tsv. The filters are the same as in the web
    /// interface's sequence table.
    Download {
        /// Job ID or web link
        #[arg(value_name = "JOB", value_parser = parse_job_id)]
        job_id: String,

        #[command(flatten)]
        out: OutputArgs,

        #[command(flatten)]
        filter: FilterArgs,

        /// Wait for the job to finish first, if it is still running
        #[arg(long)]
        wait: bool,
    },

    /// Submit, wait and download in one step
    ///
    /// One job per file; results are saved next to each other in --dir,
    /// named after the input files. Exits with 0 when all jobs completed,
    /// 5 when all failed and 9 when some failed.
    Annotate {
        /// Protein FASTA files (plain or gzipped)
        #[arg(required = true, value_name = "FASTA")]
        files: Vec<PathBuf>,

        #[command(flatten)]
        out: OutputArgs,

        #[command(flatten)]
        filter: FilterArgs,

        /// Give up waiting after this long, e.g. 30m or 2h (default: no limit)
        #[arg(long, value_name = "DURATION", value_parser = parse_duration)]
        max_wait: Option<Duration>,
    },
}

/// Where and in which formats results are saved.
#[derive(Debug, Clone, Args)]
pub struct OutputArgs {
    /// Result formats, comma-separated or repeated
    #[arg(
        long,
        short = 'f',
        value_enum,
        value_delimiter = ',',
        default_value = "tsv",
        value_name = "FORMAT"
    )]
    pub format: Vec<FormatArg>,

    /// Directory for the result files
    #[arg(long, short = 'd', default_value = ".", value_name = "DIR")]
    pub dir: PathBuf,

    /// Write the result to standard output instead of a file (one format only)
    #[arg(long, conflicts_with = "dir")]
    pub stdout: bool,

    /// Overwrite existing result files
    #[arg(long)]
    pub force: bool,
}

impl OutputArgs {
    /// Requested formats without duplicates, in the given order.
    pub fn formats(&self) -> Vec<ResultFormat> {
        let mut formats: Vec<ResultFormat> = Vec::new();
        for format in self.format.iter().map(|f| f.to_result_format()) {
            if !formats.contains(&format) {
                formats.push(format);
            }
        }
        formats
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FormatArg {
    /// Tab-separated table, one row per sequence
    Tsv,
    /// Full job result as JSON
    Json,
    /// Sequences with annotations in the headers
    Fasta,
    /// GFF3 features
    Gff3,
}

impl FormatArg {
    pub fn to_result_format(self) -> ResultFormat {
        match self {
            FormatArg::Tsv => ResultFormat::Tsv,
            FormatArg::Json => ResultFormat::Json,
            FormatArg::Fasta => ResultFormat::Fasta,
            FormatArg::Gff3 => ResultFormat::Gff3,
        }
    }
}

/// Sequence filters of the result download (same as in the web interface).
#[derive(Debug, Clone, Default, Args)]
pub struct FilterArgs {
    /// Only sequences with this match status
    #[arg(long, value_enum, value_name = "STATUS")]
    pub only: Option<MatchFilter>,

    /// Only sequences whose ID, gene or product contains this text
    #[arg(long)]
    pub search: Option<String>,

    /// Minimum sequence length (aa)
    #[arg(long, value_name = "AA")]
    pub min_length: Option<usize>,

    /// Maximum sequence length (aa)
    #[arg(long, value_name = "AA")]
    pub max_length: Option<usize>,

    /// Only this COG category, e.g. J
    #[arg(long, value_name = "CODE")]
    pub cog: Option<String>,

    /// Only this EC class (1–7)
    #[arg(long, value_name = "CLASS", value_parser = clap::value_parser!(u8).range(1..=7))]
    pub ec_class: Option<u8>,

    /// Only sequences with a gene name
    #[arg(long)]
    pub has_gene: bool,

    /// Only sequences with a product
    #[arg(long)]
    pub has_product: bool,
}

impl FilterArgs {
    pub fn to_query(&self) -> SequenceFilterQuery {
        SequenceFilterQuery {
            filter: self.only.map(|f| f.as_api().to_string()),
            search: self.search.clone().filter(|s| !s.trim().is_empty()),
            min_length: self.min_length,
            max_length: self.max_length,
            cog: self.cog.clone().filter(|s| !s.trim().is_empty()),
            ec_class: self.ec_class.map(|c| c.to_string()),
            has_gene: self.has_gene.then_some(true),
            has_product: self.has_product.then_some(true),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MatchFilter {
    /// Hash match in the Bakta DB
    #[value(alias = "bakta_db")]
    BaktaDb,
    /// Hash match in the BakRep DB (needs a server with BakRep support)
    #[value(alias = "bakrep_db")]
    BakrepDb,
    /// Hash match in the AI-DB annotations DB
    #[value(alias = "aidb_db")]
    AidbDb,
    /// Hash match in any database
    #[value(alias = "hash_match")]
    Matched,
    /// No match – candidates for Bakta or Psos
    #[value(alias = "none")]
    Unmatched,
}

impl MatchFilter {
    /// Value of the API's `filter` parameter.
    pub fn as_api(self) -> &'static str {
        match self {
            MatchFilter::BaktaDb => "bakta_db",
            MatchFilter::BakrepDb => "bakrep_db",
            MatchFilter::AidbDb => "aidb_db",
            MatchFilter::Matched => "hash_match",
            MatchFilter::Unmatched => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum StatusFilter {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl StatusFilter {
    pub fn as_api(self) -> &'static str {
        match self {
            StatusFilter::Pending => "pending",
            StatusFilter::Processing => "processing",
            StatusFilter::Completed => "completed",
            StatusFilter::Failed => "failed",
        }
    }
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
    fn download_options_parse_into_api_filters() {
        let cli = Cli::try_parse_from([
            "aidb",
            "download",
            "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c",
            "-f",
            "tsv,gff3",
            "-f",
            "tsv",
            "--only",
            "unmatched",
            "--ec-class",
            "3",
            "--has-gene",
        ])
        .unwrap();
        match cli.command {
            Command::Download { out, filter, .. } => {
                assert_eq!(out.formats(), vec![ResultFormat::Tsv, ResultFormat::Gff3]);
                let query = filter.to_query();
                assert_eq!(query.filter.as_deref(), Some("none"));
                assert_eq!(query.ec_class.as_deref(), Some("3"));
                assert_eq!(query.has_gene, Some(true));
                assert_eq!(query.has_product, None);
                assert!(query.is_active());
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn api_filter_names_work_as_aliases() {
        let cli = Cli::try_parse_from([
            "aidb",
            "download",
            "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c",
            "--only",
            "bakta_db",
        ])
        .unwrap();
        match cli.command {
            Command::Download { filter, out, .. } => {
                assert_eq!(filter.only, Some(MatchFilter::BaktaDb));
                assert_eq!(out.formats(), vec![ResultFormat::Tsv]); // default
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn bakrep_filter_maps_to_api_value() {
        for arg in ["bakrep-db", "bakrep_db"] {
            let cli = Cli::try_parse_from([
                "aidb",
                "download",
                "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c",
                "--only",
                arg,
            ])
            .unwrap();
            match cli.command {
                Command::Download { filter, .. } => {
                    assert_eq!(filter.only, Some(MatchFilter::BakrepDb));
                    assert_eq!(filter.to_query().filter.as_deref(), Some("bakrep_db"));
                }
                other => panic!("unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn invalid_output_combinations_are_rejected() {
        let job = "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c";
        assert!(Cli::try_parse_from(["aidb", "download", job, "--stdout", "-d", "out"]).is_err());
        assert!(Cli::try_parse_from(["aidb", "download", job, "--ec-class", "9"]).is_err());
        assert!(Cli::try_parse_from(["aidb", "jobs", "--all", "--limit", "5"]).is_err());
        assert!(Cli::try_parse_from(["aidb", "annotate"]).is_err());
    }

    #[test]
    fn db_info_is_a_nested_command() {
        let cli = Cli::try_parse_from(["aidb", "db", "info"]).unwrap();
        assert!(matches!(cli.command, Command::Db(DbCommand::Info)));
    }
}
