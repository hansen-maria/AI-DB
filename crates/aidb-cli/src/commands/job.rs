//! `aidb job show|wait` – job status, match counts and waiting for jobs.

use std::io::{self, IsTerminal};
use std::time::Duration;

use aidb_client::types::{JobStatus, JobViewResponse};
use aidb_client::{Client, WaitOptions};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::Serialize;

use crate::cli::OutputFormat;
use crate::error::{CliError, Result};
use crate::output::{print_json, print_rows, thousands};

/// Match counts of a completed job, by source.
#[derive(Debug, Serialize)]
struct MatchCounts {
    bakta_db: usize,
    aidb_db: usize,
    unmatched: usize,
}

/// JSON shape of `job show` and of each entry of `job wait`.
#[derive(Debug, Serialize)]
struct JobReport {
    job_id: String,
    /// "pending" | "processing" | "completed" | "failed" | "error" (could not
    /// be queried, see `error`)
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    processed_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hash_matches: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    matches: Option<MatchCounts>,
    /// Reason a job failed on the server
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
    /// Reason the job could not be queried
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_owner: Option<bool>,
    url: String,
}

impl JobReport {
    fn from_view(client: &Client, view: &JobViewResponse, matches: Option<MatchCounts>) -> Self {
        let job = &view.job;
        Self {
            job_id: job.job_id.clone(),
            status: status_str(&job.status).to_string(),
            name: job.filename.clone(),
            sequence_count: Some(job.sequence_count),
            processed_count: Some(job.processed_count),
            hash_matches: Some(job.hash_matches),
            matches,
            error_message: job.error_message.clone(),
            error: None,
            created_at: Some(job.created_at.to_rfc3339()),
            updated_at: Some(job.updated_at.to_rfc3339()),
            is_owner: Some(view.is_owner),
            url: client.job_web_url(&job.job_id).to_string(),
        }
    }

    fn from_error(client: &Client, job_id: &str, error: &aidb_client::Error) -> Self {
        Self {
            job_id: job_id.to_string(),
            status: "error".to_string(),
            name: None,
            sequence_count: None,
            processed_count: None,
            hash_matches: None,
            matches: None,
            error_message: None,
            error: Some(error.to_string()),
            created_at: None,
            updated_at: None,
            is_owner: None,
            url: client.job_web_url(job_id).to_string(),
        }
    }
}

/// API name of a job status (`pending`, `processing`, …).
pub fn status_str(status: &JobStatus) -> &'static str {
    match status {
        JobStatus::Pending => "pending",
        JobStatus::Processing => "processing",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
    }
}

fn count(n: usize) -> String {
    thousands(n as i64)
}

fn percent(part: usize, total: usize) -> String {
    if total == 0 {
        return "–".to_string();
    }
    format!("{} %", part * 100 / total)
}

// ── show ─────────────────────────────────────────────────────────────────────

pub async fn show(client: &Client, job_id: &str, output: OutputFormat) -> Result<()> {
    let view = client.job_status(job_id).await?;

    let matches = if view.job.status == JobStatus::Completed {
        let (bakta_db, aidb_db, unmatched) = tokio::try_join!(
            client.count_sequences(job_id, "bakta_db"),
            client.count_sequences(job_id, "aidb_db"),
            client.count_sequences(job_id, "none"),
        )?;
        Some(MatchCounts {
            bakta_db,
            aidb_db,
            unmatched,
        })
    } else {
        None
    };

    if output == OutputFormat::Json {
        print_json(&JobReport::from_view(client, &view, matches));
        return Ok(());
    }

    let job = &view.job;
    let mut rows: Vec<(&str, String)> = vec![
        ("Job", job.job_id.clone()),
        (
            "Name",
            job.filename.clone().unwrap_or_else(|| "–".to_string()),
        ),
        ("Status", status_str(&job.status).to_string()),
        ("Sequences", count(job.sequence_count)),
    ];
    if matches!(job.status, JobStatus::Pending | JobStatus::Processing) && job.sequence_count > 0 {
        rows.push((
            "Progress",
            format!(
                "{} / {} ({})",
                count(job.processed_count),
                count(job.sequence_count),
                percent(job.processed_count, job.sequence_count)
            ),
        ));
    }
    if let Some(m) = &matches {
        rows.push((
            "Matched",
            format!(
                "{} ({}): Bakta DB {} · AI-DB annotations DB {}",
                count(m.bakta_db + m.aidb_db),
                percent(m.bakta_db + m.aidb_db, job.sequence_count),
                count(m.bakta_db),
                count(m.aidb_db)
            ),
        ));
        rows.push(("Unmatched", count(m.unmatched)));
    }
    if let Some(error) = &job.error_message {
        rows.push(("Error", error.clone()));
    }
    rows.push((
        "Created",
        job.created_at.format("%Y-%m-%d %H:%M UTC").to_string(),
    ));
    rows.push((
        "Updated",
        job.updated_at.format("%Y-%m-%d %H:%M UTC").to_string(),
    ));
    rows.push((
        "Access",
        if view.is_owner {
            "owner".to_string()
        } else {
            "read-only (not your job, or no access token)".to_string()
        },
    ));
    rows.push(("Web", client.job_web_url(&job.job_id).to_string()));
    print_rows(&rows);
    Ok(())
}

// ── wait ─────────────────────────────────────────────────────────────────────

pub struct WaitArgs<'a> {
    pub job_ids: &'a [String],
    pub max_wait: Option<Duration>,
    pub output: OutputFormat,
    pub quiet: bool,
}

pub async fn wait(client: &Client, args: WaitArgs<'_>) -> Result<()> {
    let options = WaitOptions {
        max_wait: args.max_wait,
        ..WaitOptions::default()
    };
    let human = args.output == OutputFormat::Human;
    let display = Display::choose(args.output, args.quiet);
    let bars = display == Display::Bars;
    let results = wait_all(client, args.job_ids, &options, display).await;

    // Summary
    if human && !args.quiet && !bars {
        for (job_id, result) in &results {
            match result {
                Ok(view) => println!("{}", status_line(view)),
                Err(error) => println!("{}  error  {error}", short_id(job_id)),
            }
        }
    }
    if !human {
        let reports: Vec<JobReport> = results
            .iter()
            .map(|(job_id, result)| match result {
                Ok(view) => JobReport::from_view(client, view, None),
                Err(error) => JobReport::from_error(client, job_id, error),
            })
            .collect();
        print_json(&reports);
    }

    // Exit status: query errors first (they say more), then failed jobs
    let total = results.len();
    let mut failed = 0;
    let mut first_error = None;
    for (_, result) in results {
        match result {
            Ok(view) if view.job.status == JobStatus::Failed => failed += 1,
            Ok(_) => {}
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }
    if let Some(error) = first_error {
        return Err(error.into());
    }
    if failed > 0 {
        return Err(CliError::JobsFailed { failed, total });
    }
    Ok(())
}

/// How waiting is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    /// Live progress bars on stderr (interactive terminal)
    Bars,
    /// One stderr line per status change (CI logs, redirected output)
    Lines,
    /// Nothing
    Silent,
}

impl Display {
    pub fn choose(output: OutputFormat, quiet: bool) -> Self {
        if quiet {
            Display::Silent
        } else if output == OutputFormat::Human && io::stderr().is_terminal() {
            Display::Bars
        } else {
            Display::Lines
        }
    }
}

/// Waits for all jobs concurrently and returns each job's final state (or
/// the error that ended waiting for it), in the order given.
pub async fn wait_all(
    client: &Client,
    job_ids: &[String],
    options: &WaitOptions,
    display: Display,
) -> Vec<(String, aidb_client::Result<JobViewResponse>)> {
    let multi = MultiProgress::new();
    let waits = job_ids.iter().map(|job_id| {
        let bar = if display == Display::Bars {
            multi.add(job_bar(job_id))
        } else {
            ProgressBar::hidden()
        };
        async move {
            let mut last_logged: Option<(JobStatus, usize)> = None;
            let result = client
                .wait_for_job(job_id, options, |view| {
                    update_bar(&bar, view);
                    let state = (view.job.status.clone(), view.job.processed_count);
                    if display == Display::Lines && last_logged.as_ref() != Some(&state) {
                        eprintln!("{}", status_line(view));
                        last_logged = Some(state);
                    }
                })
                .await;
            finish_bar(&bar, &result);
            (job_id.clone(), result)
        }
    });
    futures_util::future::join_all(waits).await
}

pub fn short_id(job_id: &str) -> &str {
    job_id.get(..8).unwrap_or(job_id)
}

/// One line per job, e.g. `3f2b8c1e  completed  4,500 of 5,000 matched`.
pub fn status_line(view: &JobViewResponse) -> String {
    let job = &view.job;
    let id = short_id(&job.job_id);
    match job.status {
        JobStatus::Completed => format!(
            "{id}  completed  {} of {} matched",
            count(job.hash_matches),
            count(job.sequence_count)
        ),
        JobStatus::Failed => format!(
            "{id}  failed  {}",
            job.error_message.as_deref().unwrap_or("no reason given")
        ),
        JobStatus::Processing if job.sequence_count > 0 => format!(
            "{id}  processing  {} / {}",
            count(job.processed_count),
            count(job.sequence_count)
        ),
        ref status => format!("{id}  {}", status_str(status)),
    }
}

// ── Progress bars ────────────────────────────────────────────────────────────

fn job_bar(job_id: &str) -> ProgressBar {
    let bar = ProgressBar::new_spinner();
    if let Ok(style) = ProgressStyle::with_template("{spinner} {prefix}  {msg}") {
        bar.set_style(style);
    }
    bar.set_prefix(short_id(job_id).to_string());
    bar.set_message("waiting for the server…");
    bar.enable_steady_tick(Duration::from_millis(120));
    bar
}

fn update_bar(bar: &ProgressBar, view: &JobViewResponse) {
    let job = &view.job;
    if job.sequence_count > 0 && bar.length().is_none() {
        // Total known: switch from spinner to a bar
        if let Ok(style) = ProgressStyle::with_template(
            "{spinner} {prefix}  [{bar:30}] {human_pos}/{human_len} {msg}",
        ) {
            bar.set_style(style.progress_chars("=> "));
        }
        bar.set_length(job.sequence_count as u64);
    }
    bar.set_position(job.processed_count as u64);
    bar.set_message(status_str(&job.status));
}

fn finish_bar(bar: &ProgressBar, result: &aidb_client::Result<JobViewResponse>) {
    match result {
        Ok(view) if view.job.status == JobStatus::Completed => {
            let job = &view.job;
            bar.finish_with_message(format!(
                "completed – {} of {} matched",
                count(job.hash_matches),
                count(job.sequence_count)
            ));
        }
        Ok(view) => bar.abandon_with_message(format!(
            "failed: {}",
            view.job
                .error_message
                .as_deref()
                .unwrap_or("no reason given")
        )),
        Err(error) => bar.abandon_with_message(format!("error: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aidb_client::types::{PaginatedJobResponse, PaginationInfo};

    fn view(status: JobStatus, processed: usize, matches: usize) -> JobViewResponse {
        let now = "2026-11-02T10:00:00Z".parse().unwrap();
        JobViewResponse {
            job: PaginatedJobResponse {
                job_id: "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c".to_string(),
                status,
                created_at: now,
                updated_at: now,
                filename: Some("proteins.faa".to_string()),
                sequence_count: 5000,
                processed_count: processed,
                hash_matches: matches,
                alignment_matches: 0,
                error_message: None,
                sequences: Vec::new(),
                pagination: PaginationInfo::new(1, 1, 0),
                filter: "all".to_string(),
                filtered_count: 0,
            },
            is_owner: true,
        }
    }

    #[test]
    fn status_lines_are_short_and_informative() {
        assert_eq!(
            status_line(&view(JobStatus::Completed, 5000, 4500)),
            "3f2b8c1e  completed  4,500 of 5,000 matched"
        );
        assert_eq!(
            status_line(&view(JobStatus::Processing, 1200, 0)),
            "3f2b8c1e  processing  1,200 / 5,000"
        );
        assert_eq!(
            status_line(&view(JobStatus::Pending, 0, 0)),
            "3f2b8c1e  pending"
        );
        assert_eq!(
            status_line(&view(JobStatus::Failed, 0, 0)),
            "3f2b8c1e  failed  no reason given"
        );
    }

    #[test]
    fn percent_handles_empty_jobs() {
        assert_eq!(percent(1, 4), "25 %");
        assert_eq!(percent(0, 0), "–");
    }
}
