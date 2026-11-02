//! `aidb submit <FASTA>…` – upload protein FASTA files as new jobs.

use std::io::{self, IsTerminal};
use std::path::PathBuf;

use aidb_client::Client;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

use crate::cli::OutputFormat;
use crate::error::{CliError, Result};
use crate::fasta::{self, CheckedFile};
use crate::output::print_json;

pub struct SubmitOptions<'a> {
    pub files: &'a [PathBuf],
    pub name: Option<&'a str>,
    pub output: OutputFormat,
    pub quiet: bool,
}

/// JSON shape of one entry of `aidb submit -o json`.
#[derive(Serialize)]
struct Submitted {
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn run(client: &Client, options: SubmitOptions<'_>) -> Result<()> {
    if options.name.is_some() && options.files.len() > 1 {
        return Err(CliError::Usage(
            "--name only works with a single file; with several files each job is named \
             after its file"
                .to_string(),
        ));
    }

    // Check every file before uploading any, so a typo in the last file does
    // not leave a half-submitted batch behind
    let checked = options
        .files
        .iter()
        .map(|path| fasta::check(path))
        .collect::<Result<Vec<CheckedFile>>>()?;

    let human = options.output == OutputFormat::Human;
    let show_progress = human && !options.quiet && io::stderr().is_terminal();
    let mut submitted = Vec::with_capacity(checked.len());
    let mut first_error: Option<aidb_client::Error> = None;

    for file in &checked {
        let display = file.path.display().to_string();
        let bar = upload_bar(file, show_progress);
        let progress = bar.clone();
        let outcome = client
            .submit_fasta(&file.path, options.name, move |bytes| progress.inc(bytes))
            .await;
        bar.finish_and_clear();

        match outcome {
            Ok(created) => {
                let url = client.job_web_url(&created.job_id).to_string();
                if human && options.quiet {
                    println!("{}", created.job_id);
                } else if human {
                    println!("{display}  →  job {}", created.job_id);
                    println!("    {url}");
                }
                submitted.push(Submitted {
                    file: display,
                    job_id: Some(created.job_id),
                    url: Some(url),
                    error: None,
                });
            }
            Err(error) => {
                // Unreachable server, invalid token, rate limit: every further
                // file would fail the same way
                let affects_all = error.is_temporary() || matches!(error.status(), Some(401 | 403));
                if checked.len() > 1 && human {
                    eprintln!("error: {display}: {error}");
                }
                submitted.push(Submitted {
                    file: display,
                    job_id: None,
                    url: None,
                    error: Some(error.to_string()),
                });
                if first_error.is_none() {
                    first_error = Some(error);
                }
                if affects_all {
                    break;
                }
            }
        }
    }

    if !human {
        print_json(&submitted);
    }

    let ok: Vec<&str> = submitted
        .iter()
        .filter_map(|s| s.job_id.as_deref())
        .collect();
    if human && !options.quiet && !ok.is_empty() {
        eprintln!();
        eprintln!("Follow the progress with:  aidb job wait {}", ok.join(" "));
    }

    match first_error {
        None => Ok(()),
        // Nothing submitted: report the cause with its own exit code
        Some(error) if ok.is_empty() => Err(error.into()),
        Some(_) => Err(CliError::SubmissionsFailed {
            failed: checked.len() - ok.len(),
            total: checked.len(),
        }),
    }
}

pub fn upload_bar(file: &CheckedFile, visible: bool) -> ProgressBar {
    if !visible {
        return ProgressBar::hidden();
    }
    let bar = ProgressBar::new(file.size);
    let style = ProgressStyle::with_template(
        "{msg} [{bar:30}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("=> ");
    bar.set_style(style);
    let name = file
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    bar.set_message(format!("Uploading {name}"));
    bar
}
