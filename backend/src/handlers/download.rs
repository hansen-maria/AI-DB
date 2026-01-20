//! ============================================================================
//! Download handlers for exporting job results
//! ============================================================================

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::CookieJar;

use crate::auth::OWNER_COOKIE_NAME;
use crate::export::{generate_content, DownloadFormat};
use crate::models::{ErrorResponse, JobStatus};
use crate::state::AppState;

/// Download job results in specified format
#[utoipa::path(
    get,
    path = "/api/job/{job_id}/download/{format}",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Job ID (UUID)"),
        ("format" = String, Path, description = "Download format: tsv, json, fasta, gff3")
    ),
    responses(
        (status = 200, description = "File download", content_type = "application/octet-stream"),
        (status = 400, description = "Invalid format or job not completed"),
        (status = 403, description = "Not authorized"),
        (status = 404, description = "Job not found")
    )
)]
pub async fn download_job(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((job_id, format_str)): Path<(String, String)>,
) -> impl IntoResponse {
    // Parse format
    let format = match DownloadFormat::from_str(&format_str) {
        Some(f) => f,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(format!(
                    "Invalid format '{}'. Supported formats: tsv, json, fasta, gff3",
                    format_str
                ))),
            )
                .into_response();
        }
    };

    let owner_id = jar.get(OWNER_COOKIE_NAME).map(|c| c.value().to_string());
    let jobs = state.jobs();

    match jobs.get(&job_id) {
        Some(job) => {
            // Check ownership
            let is_owner = match (&job.owner_id, &owner_id) {
                (Some(job_owner), Some(cookie_owner)) => job_owner == cookie_owner,
                _ => false,
            };

            if !is_owner {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("Not authorized to download this job")),
                )
                    .into_response();
            }

            // Check job is completed
            if job.status != JobStatus::Completed {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new("Job is not yet completed")),
                )
                    .into_response();
            }

            // Generate content based on format
            let content = generate_content(job, format);

            // Generate filename
            let base_name = job
                .filename
                .as_ref()
                .map(|f| {
                    f.trim_end_matches(".gz")
                        .trim_end_matches(".fasta")
                        .trim_end_matches(".fa")
                })
                .unwrap_or("results");
            let filename = format!("{}_annotations.{}", base_name, format.file_extension());

            // Return file response
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, format.content_type()),
                    (
                        header::CONTENT_DISPOSITION,
                        &format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                content,
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!(
                "Job with ID '{}' not found",
                job_id
            ))),
        )
            .into_response(),
    }
}
