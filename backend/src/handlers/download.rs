//! ============================================================================
//! Download handlers for exporting job results
//! ============================================================================

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;

use crate::auth::OWNER_COOKIE_NAME;
use crate::export::{generate_content, DownloadFormat};
use crate::models::{AdvancedSequenceFilter, ErrorResponse, JobStatus, SequenceFilter};
use crate::state::AppState;

/// Optional sequence-filter query parameters accepted by the download endpoint.
/// All fields mirror those of `GetJobQuery` so the frontend can reuse its
/// existing filter state when triggering a filtered export.
#[derive(Debug, Default, Deserialize)]
pub struct DownloadFilterQuery {
    /// Match-status filter: all | hash_match | bakta_db | aidb_db | alignment | none
    pub filter: Option<String>,
    /// Case-insensitive text search in ID, gene, product
    pub search: Option<String>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    /// COG category code, e.g. "J"
    pub cog: Option<String>,
    /// EC class digit, e.g. "1"
    pub ec_class: Option<String>,
    pub has_gene: Option<bool>,
    pub has_product: Option<bool>,
}

/// Download job results in specified format (with optional sequence filtering)
#[utoipa::path(
    get,
    path = "/api/job/{job_id}/download/{format}",
    tag = "Jobs",
    params(
        ("job_id"      = String,         Path,  description = "Job ID (UUID)"),
        ("format"      = String,         Path,  description = "Download format: tsv, json, fasta, gff3"),
        ("filter"      = Option<String>, Query, description = "Match-status filter"),
        ("search"      = Option<String>, Query, description = "Text search in ID / gene / product"),
        ("min_length"  = Option<usize>,  Query, description = "Minimum sequence length"),
        ("max_length"  = Option<usize>,  Query, description = "Maximum sequence length"),
        ("cog"         = Option<String>, Query, description = "COG category code"),
        ("ec_class"    = Option<String>, Query, description = "EC class (1–7)"),
        ("has_gene"    = Option<bool>,   Query, description = "Only sequences with a gene name"),
        ("has_product" = Option<bool>,   Query, description = "Only sequences with a product")
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
    Query(filter_query): Query<DownloadFilterQuery>,
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

    // Build sequence filter from query params
    let advanced_filter = AdvancedSequenceFilter {
        basic: filter_query
            .filter
            .as_deref()
            .map(SequenceFilter::from_str)
            .unwrap_or(SequenceFilter::All),
        search: filter_query.search.filter(|s| !s.is_empty()),
        min_length: filter_query.min_length,
        max_length: filter_query.max_length,
        cog_category: filter_query.cog.filter(|s| !s.is_empty()),
        ec_class: filter_query.ec_class.filter(|s| !s.is_empty()),
        has_gene: filter_query.has_gene,
        has_product: filter_query.has_product,
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

            // Apply sequence filter when any filter is active
            let is_filtered = advanced_filter.has_advanced_filters()
                || advanced_filter.basic != SequenceFilter::All;

            let filtered_sequences = if is_filtered {
                job.sequences.as_ref().map(|seqs| {
                    seqs.iter()
                        .filter(|s| advanced_filter.matches(s))
                        .cloned()
                        .collect::<Vec<_>>()
                })
            } else {
                job.sequences.clone()
            };

            // Build a temporary view with the (possibly filtered) sequences
            let mut export_job = job.clone();
            export_job.sequences = filtered_sequences;

            // Generate content based on format
            let content = generate_content(&export_job, format);

            // Generate filename (append "_filtered" suffix when a filter was applied)
            let base_name = job
                .filename
                .as_ref()
                .map(|f| {
                    f.trim_end_matches(".gz")
                        .trim_end_matches(".fasta")
                        .trim_end_matches(".fa")
                })
                .unwrap_or("results");
            let suffix = if is_filtered { "_filtered" } else { "" };
            let filename = format!(
                "{}{}_annotations.{}",
                base_name,
                suffix,
                format.file_extension()
            );

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
