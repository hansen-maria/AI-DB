//! `aidb annotate <FASTA>…` – submit, wait and download in one step.
//!
//! Everything that can be checked locally is checked before the first upload:
//! the input files and whether the result files can be written. One job per
//! file; results are named after the input files.

use std::collections::HashMap;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::time::Duration;

use aidb_client::types::JobStatus;
use aidb_client::{Client, WaitOptions};
use serde::Serialize;

use crate::cli::{FilterArgs, OutputArgs, OutputFormat};
use crate::commands::download::{
    base_name, check_targets, ensure_downloadable, human_bytes, result_path, save_all, SavedFile,
};
use crate::commands::job::{self, Display};
use crate::commands::submit::upload_bar;
use crate::error::{CliError, Result};
use crate::fasta;
use crate::output::print_json;

pub struct AnnotateArgs<'a> {
    pub files: &'a [PathBuf],
    pub out: &'a OutputArgs,
    pub filter: &'a FilterArgs,
    pub max_wait: Option<Duration>,
    pub output: OutputFormat,
    pub quiet: bool,
}

/// What happened to one input file.
enum Outcome {
    Saved(Vec<SavedFile>),
    /// The job failed on the server
    JobFailed(String),
    /// Submitting, waiting or downloading failed
    Error(CliError),
}

/// JSON shape of one entry of `aidb annotate -o json`.
#[derive(Serialize)]
struct Report<'a> {
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_id: Option<&'a str>,
    /// "completed" | "failed" | "error"
    status: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    results: Option<&'a [SavedFile]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

struct Item {
    path: PathBuf,
    /// Result file base name, unique within this run
    base: String,
    job_id: Option<String>,
    outcome: Option<Outcome>,
}

pub async fn run(client: &Client, args: AnnotateArgs<'_>) -> Result<()> {
    if args.out.stdout {
        return Err(CliError::Usage(
            "annotate saves files; use `aidb download --stdout` for a single result".to_string(),
        ));
    }
    let formats = args.out.formats();
    let filter = args.filter.to_query();

    // 1. Local checks: inputs, unique names, writable targets
    let checked = args
        .files
        .iter()
        .map(|path| fasta::check(path))
        .collect::<Result<Vec<_>>>()?;
    let mut items = unique_items(checked.iter().map(|c| c.path.clone()).collect());
    let targets: Vec<PathBuf> = items
        .iter()
        .flat_map(|item| {
            formats
                .iter()
                .map(|&format| result_path(&args.out.dir, &item.base, format, &filter))
        })
        .collect();
    check_targets(&targets, args.out.force)?;

    let human = args.output == OutputFormat::Human;
    let show_progress = human && !args.quiet && io::stderr().is_terminal();

    // 2. Submit one job per file
    for index in 0..items.len() {
        let file = &checked[index];
        let bar = upload_bar(file, show_progress);
        let progress = bar.clone();
        let result = client
            .submit_fasta(&file.path, None, move |bytes| progress.inc(bytes))
            .await;
        bar.finish_and_clear();
        match result {
            Ok(created) => items[index].job_id = Some(created.job_id),
            Err(error) => {
                let affects_all = error.is_temporary() || matches!(error.status(), Some(401 | 403));
                items[index].outcome = Some(Outcome::Error(error.into()));
                if affects_all {
                    // Every further upload would fail the same way
                    for item in items.iter_mut().skip(index + 1) {
                        item.outcome = Some(Outcome::Error(CliError::Usage(
                            "not submitted after an earlier error".to_string(),
                        )));
                    }
                    break;
                }
            }
        }
    }

    // 3. Wait for all submitted jobs
    let submitted: Vec<String> = items.iter().filter_map(|i| i.job_id.clone()).collect();
    if !submitted.is_empty() {
        let options = WaitOptions {
            max_wait: args.max_wait,
            ..WaitOptions::default()
        };
        let display = Display::choose(args.output, args.quiet);
        let mut finished: HashMap<String, _> = job::wait_all(client, &submitted, &options, display)
            .await
            .into_iter()
            .collect();

        // 4. Download the results of completed jobs
        for item in items.iter_mut() {
            let Some(job_id) = item.job_id.as_deref() else {
                continue;
            };
            let outcome = match finished.remove(job_id) {
                Some(Ok(view)) if view.job.status == JobStatus::Failed => Outcome::JobFailed(
                    view.job
                        .error_message
                        .clone()
                        .unwrap_or_else(|| "no reason given".to_string()),
                ),
                Some(Ok(view)) => match ensure_downloadable(&view) {
                    Ok(()) => {
                        let paths: Vec<PathBuf> = formats
                            .iter()
                            .map(|&format| result_path(&args.out.dir, &item.base, format, &filter))
                            .collect();
                        match save_all(client, job_id, &formats, &filter, &paths).await {
                            Ok(saved) => Outcome::Saved(saved),
                            Err(error) => Outcome::Error(error),
                        }
                    }
                    Err(error) => Outcome::Error(error),
                },
                Some(Err(error)) => Outcome::Error(error.into()),
                None => Outcome::Error(CliError::Usage("job result missing".to_string())),
            };
            item.outcome = Some(outcome);
        }
    }

    report(&items, args.output, args.quiet);
    exit_status(items)
}

/// Pairs each input with a result base name, made unique with `_2`, `_3`, …
/// when several inputs share a name (e.g. `a/proteins.faa`, `b/proteins.faa`).
fn unique_items(paths: Vec<PathBuf>) -> Vec<Item> {
    let mut used: HashMap<String, usize> = HashMap::new();
    paths
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let stem = base_name(&name);
            let count = used.entry(stem.clone()).or_insert(0);
            *count += 1;
            let base = if *count == 1 {
                stem
            } else {
                format!("{stem}_{count}")
            };
            Item {
                path,
                base,
                job_id: None,
                outcome: None,
            }
        })
        .collect()
}

fn report(items: &[Item], output: OutputFormat, quiet: bool) {
    if output == OutputFormat::Json {
        let reports: Vec<Report> = items
            .iter()
            .map(|item| {
                let (status, results, error) = match &item.outcome {
                    Some(Outcome::Saved(saved)) => ("completed", Some(saved.as_slice()), None),
                    Some(Outcome::JobFailed(reason)) => ("failed", None, Some(reason.clone())),
                    Some(Outcome::Error(error)) => ("error", None, Some(error.to_string())),
                    None => ("error", None, Some("not processed".to_string())),
                };
                Report {
                    file: item.path.display().to_string(),
                    job_id: item.job_id.as_deref(),
                    status,
                    results,
                    error,
                }
            })
            .collect();
        print_json(&reports);
        return;
    }

    for item in items {
        let file = item.path.display();
        match &item.outcome {
            Some(Outcome::Saved(saved)) if quiet => {
                for path in saved.iter().filter_map(|s| s.path.as_ref()) {
                    println!("{}", path.display());
                }
            }
            Some(Outcome::Saved(saved)) => {
                println!("{file}");
                for saved in saved {
                    if let Some(path) = &saved.path {
                        println!("  → {} ({})", path.display(), human_bytes(saved.bytes));
                    }
                }
            }
            Some(Outcome::JobFailed(reason)) => eprintln!("{file}: job failed: {reason}"),
            Some(Outcome::Error(error)) => eprintln!("{file}: {error}"),
            None => eprintln!("{file}: not processed"),
        }
    }
}

/// 0 when everything was saved. When nothing worked, the first cause with
/// its own exit code (e.g. 7 for an unreachable server); otherwise "x of y
/// jobs failed" (exit 5 when all jobs failed, 9 when some did).
fn exit_status(items: Vec<Item>) -> Result<()> {
    let total = items.len();
    let saved = items
        .iter()
        .filter(|i| matches!(i.outcome, Some(Outcome::Saved(_))))
        .count();
    if saved == total {
        return Ok(());
    }
    if saved > 0 {
        return Err(CliError::JobsFailed {
            failed: total - saved,
            total,
        });
    }

    let mut first_failed = None;
    for item in items {
        match item.outcome {
            Some(Outcome::Error(error)) => return Err(error),
            Some(Outcome::JobFailed(reason)) if first_failed.is_none() => {
                first_failed = Some((item.job_id.unwrap_or_default(), reason));
            }
            _ => {}
        }
    }
    match first_failed {
        Some((job_id, reason)) if total == 1 => Err(CliError::JobFailed { job_id, reason }),
        _ => Err(CliError::JobsFailed {
            failed: total,
            total,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_input_names_get_unique_result_names() {
        let items = unique_items(vec![
            PathBuf::from("a/proteins.faa"),
            PathBuf::from("b/proteins.faa.gz"),
            PathBuf::from("c/other.faa"),
            PathBuf::from("d/proteins.fasta"),
        ]);
        let bases: Vec<&str> = items.iter().map(|i| i.base.as_str()).collect();
        assert_eq!(bases, ["proteins", "proteins_2", "other", "proteins_3"]);
    }

    #[test]
    fn exit_status_distinguishes_all_some_and_none_failed() {
        let item = |outcome| Item {
            path: PathBuf::from("x.faa"),
            base: "x".to_string(),
            job_id: Some("j".to_string()),
            outcome: Some(outcome),
        };
        assert!(exit_status(vec![item(Outcome::Saved(Vec::new()))]).is_ok());

        let some = exit_status(vec![
            item(Outcome::Saved(Vec::new())),
            item(Outcome::JobFailed("x".into())),
        ])
        .unwrap_err();
        assert_eq!(some.exit_code(), crate::exit::PARTIAL);

        let all = exit_status(vec![
            item(Outcome::JobFailed("x".into())),
            item(Outcome::JobFailed("y".into())),
        ])
        .unwrap_err();
        assert_eq!(all.exit_code(), crate::exit::JOB_FAILED);

        let single = exit_status(vec![item(Outcome::JobFailed("boom".into()))]).unwrap_err();
        assert_eq!(single.exit_code(), crate::exit::JOB_FAILED);

        // Nothing worked because of a server problem: its own exit code
        let unreachable = CliError::from(aidb_client::Error::Api {
            status: 503,
            detail: String::new(),
        });
        let down = exit_status(vec![
            item(Outcome::Error(unreachable)),
            item(Outcome::JobFailed("x".into())),
        ])
        .unwrap_err();
        assert_eq!(down.exit_code(), crate::exit::UNAVAILABLE);
    }
}
