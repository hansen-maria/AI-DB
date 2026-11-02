//! `aidb download <JOB>` – save the result files of a completed job.
//!
//! The saving logic is shared with `aidb annotate`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use aidb_client::types::{
    result_base_name, result_file_name, JobStatus, JobViewResponse, SequenceFilterQuery,
};
use aidb_client::{Client, ResultFormat, WaitOptions};
use serde::Serialize;

use crate::cli::{FilterArgs, OutputArgs, OutputFormat};
use crate::commands::job::{self, Display};
use crate::error::{CliError, Result};
use crate::output::print_json;

/// One saved result file.
#[derive(Debug, Serialize)]
pub struct SavedFile {
    pub format: String,
    /// `None` when written to standard output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    pub bytes: u64,
}

pub struct DownloadArgs<'a> {
    pub job_id: &'a str,
    pub out: &'a OutputArgs,
    pub filter: &'a FilterArgs,
    pub wait: bool,
    pub output: OutputFormat,
    pub quiet: bool,
}

pub async fn run(client: &Client, args: DownloadArgs<'_>) -> Result<()> {
    let formats = args.out.formats();
    if args.out.stdout && formats.len() > 1 {
        return Err(CliError::Usage(
            "--stdout writes a single file; choose one format".to_string(),
        ));
    }

    let mut view = client.job_status(args.job_id).await?;
    if matches!(view.job.status, JobStatus::Pending | JobStatus::Processing) {
        if !args.wait {
            return Err(CliError::NotReady {
                job_id: args.job_id.to_string(),
                status: job::status_str(&view.job.status).to_string(),
            });
        }
        let display = Display::choose(args.output, args.quiet);
        let job_ids = [args.job_id.to_string()];
        let mut results = job::wait_all(client, &job_ids, &WaitOptions::default(), display).await;
        view = results.remove(0).1?;
    }
    ensure_downloadable(&view)?;

    let filter = args.filter.to_query();
    let base = base_name(view.job.filename.as_deref().unwrap_or(args.job_id));

    let saved = if args.out.stdout {
        let mut stdout = tokio::io::stdout();
        let bytes = client
            .download(args.job_id, formats[0], &filter, &mut stdout)
            .await?;
        vec![SavedFile {
            format: formats[0].to_string(),
            path: None,
            bytes,
        }]
    } else {
        let paths: Vec<PathBuf> = formats
            .iter()
            .map(|&format| result_path(&args.out.dir, &base, format, &filter))
            .collect();
        check_targets(&paths, args.out.force)?;
        save_all(client, args.job_id, &formats, &filter, &paths).await?
    };

    if args.out.stdout {
        return Ok(()); // stdout carries the data
    }
    report_saved(&saved, args.output, args.quiet);
    Ok(())
}

/// Rejects jobs whose results cannot be downloaded (failed, not owned).
pub fn ensure_downloadable(view: &JobViewResponse) -> Result<()> {
    let job = &view.job;
    if job.status == JobStatus::Failed {
        return Err(CliError::JobFailed {
            job_id: job.job_id.clone(),
            reason: job
                .error_message
                .clone()
                .unwrap_or_else(|| "no reason given".to_string()),
        });
    }
    if !view.is_owner {
        return Err(CliError::NotOwner(job.job_id.clone()));
    }
    Ok(())
}

/// Downloads every format to its path (`formats` and `paths` in the same order).
pub async fn save_all(
    client: &Client,
    job_id: &str,
    formats: &[ResultFormat],
    filter: &SequenceFilterQuery,
    paths: &[PathBuf],
) -> Result<Vec<SavedFile>> {
    let mut saved = Vec::with_capacity(formats.len());
    for (&format, path) in formats.iter().zip(paths) {
        let bytes = save_file(client, job_id, format, filter, path).await?;
        saved.push(SavedFile {
            format: format.to_string(),
            path: Some(path.clone()),
            bytes,
        });
    }
    Ok(saved)
}

/// Downloads into a hidden `.part` file next to `path` and renames it on
/// success, so an interrupted download never leaves a truncated result.
async fn save_file(
    client: &Client,
    job_id: &str,
    format: ResultFormat,
    filter: &SequenceFilterQuery,
    path: &Path,
) -> Result<u64> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)
            .map_err(|e| CliError::io(format!("cannot create {}", dir.display()), e))?;
    }
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "result".to_string());
    let part = path.with_file_name(format!(".{file_name}.part"));

    let mut file = tokio::fs::File::create(&part)
        .await
        .map_err(|e| CliError::io(format!("cannot write {}", part.display()), e))?;
    let result = client.download(job_id, format, filter, &mut file).await;
    drop(file);

    match result {
        Ok(bytes) => {
            tokio::fs::rename(&part, path)
                .await
                .map_err(|e| CliError::io(format!("cannot write {}", path.display()), e))?;
            Ok(bytes)
        }
        Err(error) => {
            let _ = tokio::fs::remove_file(&part).await;
            Err(error.into())
        }
    }
}

/// Fails if a target exists (without `--force`) or two targets collide.
pub fn check_targets(paths: &[PathBuf], force: bool) -> Result<()> {
    let mut seen = HashSet::new();
    for path in paths {
        if !seen.insert(path) {
            return Err(CliError::Usage(format!(
                "two results would be saved as {}",
                path.display()
            )));
        }
        if path.exists() && !force {
            return Err(CliError::Usage(format!(
                "{} already exists; add --force to overwrite it",
                path.display()
            )));
        }
    }
    Ok(())
}

/// `<dir>/<base>[_filtered]_annotations.<ext>`, like the web download.
pub fn result_path(
    dir: &Path,
    base: &str,
    format: ResultFormat,
    filter: &SequenceFilterQuery,
) -> PathBuf {
    dir.join(result_file_name(base, filter.is_active(), format.as_str()))
}

/// Safe file base name from a job or input file name (shared with the web
/// download, see `aidb_types::naming`).
pub fn base_name(name: &str) -> String {
    result_base_name(name)
}

/// Prints where the files went.
pub fn report_saved(saved: &[SavedFile], output: OutputFormat, quiet: bool) {
    match output {
        OutputFormat::Json => print_json(saved),
        OutputFormat::Human if quiet => {
            for file in saved {
                if let Some(path) = &file.path {
                    println!("{}", path.display());
                }
            }
        }
        OutputFormat::Human => {
            for file in saved {
                if let Some(path) = &file.path {
                    println!("Saved {} ({})", path.display(), human_bytes(file.bytes));
                }
            }
        }
    }
}

/// `1234` -> `1.2 kB`
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["kB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_paths_match_the_web_download() {
        let dir = Path::new("out");
        let none = SequenceFilterQuery::default();
        let unmatched = SequenceFilterQuery {
            filter: Some("none".into()),
            ..SequenceFilterQuery::default()
        };
        assert_eq!(
            result_path(dir, "proteins", ResultFormat::Tsv, &none),
            Path::new("out/proteins_annotations.tsv")
        );
        assert_eq!(
            result_path(dir, "proteins", ResultFormat::Fasta, &unmatched),
            Path::new("out/proteins_filtered_annotations.fasta")
        );
    }

    #[test]
    fn existing_and_colliding_targets_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("a_annotations.tsv");
        std::fs::write(&existing, "x").unwrap();
        let fresh = dir.path().join("b_annotations.tsv");

        assert!(check_targets(&[fresh.clone()], false).is_ok());
        assert!(check_targets(&[existing.clone()], false).is_err());
        assert!(check_targets(&[existing], true).is_ok());
        assert!(check_targets(&[fresh.clone(), fresh], true).is_err());
    }

    #[test]
    fn sizes_are_human_readable() {
        assert_eq!(human_bytes(999), "999 B");
        assert_eq!(human_bytes(1234), "1.2 kB");
        assert_eq!(human_bytes(5_600_000), "5.6 MB");
    }
}
