//! `aidb jobs` – list your jobs, newest first.

use aidb_client::types::{JobStatus, JobSummary, ListJobsQuery};
use aidb_client::Client;

use crate::cli::{OutputFormat, StatusFilter};
use crate::commands::job::status_str;
use crate::error::Result;
use crate::output::{print_json, thousands};

/// Page size when fetching more than one page.
const PAGE_SIZE: usize = 1000;

pub struct JobsArgs {
    pub status: Option<StatusFilter>,
    pub search: Option<String>,
    /// `None`: all jobs
    pub limit: Option<usize>,
    pub output: OutputFormat,
    pub quiet: bool,
}

pub async fn run(client: &Client, args: JobsArgs) -> Result<()> {
    let per_page = args.limit.unwrap_or(PAGE_SIZE).clamp(1, PAGE_SIZE);
    let mut jobs: Vec<JobSummary> = Vec::new();
    let mut total;
    let mut page = 1;
    loop {
        let query = ListJobsQuery {
            page: Some(page),
            per_page: Some(per_page),
            status: args.status.map(|s| s.as_api().to_string()),
            search: args.search.clone().filter(|s| !s.trim().is_empty()),
        };
        let response = client.list_jobs(&query).await?;
        total = response.pagination.total_items;
        jobs.extend(response.jobs);
        let enough = args.limit.is_some_and(|limit| jobs.len() >= limit);
        if enough || !response.pagination.has_next {
            break;
        }
        page += 1;
    }
    if let Some(limit) = args.limit {
        jobs.truncate(limit);
    }

    match args.output {
        OutputFormat::Json => print_json(&jobs),
        OutputFormat::Human if args.quiet => {
            for job in &jobs {
                println!("{}", job.job_id);
            }
        }
        OutputFormat::Human => {
            if jobs.is_empty() {
                eprintln!("No jobs found.");
            } else {
                print_table(&jobs);
                if total > jobs.len() {
                    eprintln!(
                        "\nShowing {} of {} jobs – use --limit or --all to see more.",
                        jobs.len(),
                        total
                    );
                }
            }
        }
    }
    Ok(())
}

fn row(job: &JobSummary) -> [String; 6] {
    let matched = if job.status == JobStatus::Completed && job.sequence_count > 0 {
        format!(
            "{} ({} %)",
            thousands(job.hash_matches as i64),
            job.hash_matches * 100 / job.sequence_count
        )
    } else {
        "–".to_string()
    };
    [
        job.job_id.clone(),
        status_str(&job.status).to_string(),
        thousands(job.sequence_count as i64),
        matched,
        job.updated_at.format("%Y-%m-%d %H:%M").to_string(),
        job.filename.clone().unwrap_or_else(|| "–".to_string()),
    ]
}

/// Aligned columns; the last one (name) is not padded.
fn format_table(jobs: &[JobSummary]) -> Vec<String> {
    const HEADER: [&str; 6] = [
        "JOB ID",
        "STATUS",
        "SEQUENCES",
        "MATCHED",
        "UPDATED (UTC)",
        "NAME",
    ];
    let rows: Vec<[String; 6]> = jobs.iter().map(row).collect();
    let mut widths = HEADER.map(|h| h.chars().count());
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let line = |cells: [&str; 6]| -> String {
        let mut out = String::new();
        for (i, cell) in cells.iter().enumerate() {
            if i == cells.len() - 1 {
                out.push_str(cell);
            } else {
                let pad = widths[i] - cell.chars().count();
                out.push_str(cell);
                out.push_str(&" ".repeat(pad + 2));
            }
        }
        out
    };
    let mut lines = vec![line(HEADER)];
    for row in &rows {
        lines.push(line([&row[0], &row[1], &row[2], &row[3], &row[4], &row[5]]));
    }
    lines
}

fn print_table(jobs: &[JobSummary]) {
    for line in format_table(jobs) {
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(id: &str, status: JobStatus, name: &str) -> JobSummary {
        let now = "2026-11-02T10:00:00Z".parse().unwrap();
        JobSummary {
            job_id: id.to_string(),
            status,
            created_at: now,
            updated_at: now,
            filename: Some(name.to_string()),
            sequence_count: 5000,
            processed_count: 5000,
            hash_matches: 4500,
            error_message: None,
        }
    }

    #[test]
    fn table_columns_are_aligned() {
        let lines = format_table(&[
            job(
                "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c",
                JobStatus::Completed,
                "proteins.faa",
            ),
            job(
                "c0ffee00-0000-4000-8000-000000000000",
                JobStatus::Processing,
                "x.faa",
            ),
        ]);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("JOB ID"));
        // Every column starts at the same offset in every line
        let offset = |line: &str, text: &str| line.find(text).unwrap();
        assert_eq!(offset(&lines[0], "STATUS"), offset(&lines[1], "completed"));
        assert_eq!(
            offset(&lines[1], "completed"),
            offset(&lines[2], "processing")
        );
        assert!(lines[1].contains("4,500 (90 %)"));
        assert!(lines[2].contains("–"));
        assert!(lines[1].ends_with("proteins.faa"));
    }
}
