//! ============================================================================
//! Job management handlers
//! ============================================================================

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use std::{env, io::Write, path::PathBuf};
use tempfile::Builder as TempFileBuilder;
use uuid::Uuid;

use crate::auth::{get_or_create_owner, OWNER_COOKIE_NAME};
use crate::models::{
    AdvancedSequenceFilter, ErrorResponse, GetJobQuery, JobCreateResponse, JobResponse, JobStatus,
    JobSummary, ListJobsQuery, PaginatedJobResponse, PaginatedJobsResponse, PaginationInfo,
    SequenceFilter, DEFAULT_PER_PAGE, MAX_PER_PAGE,
};
use crate::services::process_job_from_file;
use crate::state::AppState;

/// Maximum upload size (100 MB)
const MAX_UPLOAD_SIZE: usize = 100 * 1024 * 1024;

/// Get temp directory from environment or use default
fn get_temp_dir() -> PathBuf {
    env::var("AI_DB_TEMP_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Get job status and results (with paginated and filtered sequence list)
#[utoipa::path(
    get,
    path = "/api/job/{job_id}",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)"),
        ("page" = Option<usize>, Query, description = "Sequence page (1-indexed, default: 1)"),
        ("per_page" = Option<usize>, Query, description = "Sequences per page (default: 20, max: 10000)"),
        ("filter" = Option<String>, Query, description = "Filter: all, hash_match, alignment, none"),
        ("search" = Option<String>, Query, description = "Search in ID, gene, product"),
        ("min_length" = Option<usize>, Query, description = "Minimum sequence length"),
        ("max_length" = Option<usize>, Query, description = "Maximum sequence length"),
        ("cog" = Option<String>, Query, description = "Filter by COG category"),
        ("ec_class" = Option<String>, Query, description = "Filter by EC class (1-7)"),
        ("has_gene" = Option<bool>, Query, description = "Only sequences with gene name"),
        ("has_product" = Option<bool>, Query, description = "Only sequences with product")
    ),
    responses(
        (status = 200, description = "Job found", body = PaginatedJobResponse),
        (status = 404, description = "Job not found", body = ErrorResponse)
    )
)]
pub async fn get_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Query(query): Query<GetJobQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .min(MAX_PER_PAGE)
        .max(1);

    // Build advanced filter from query parameters
    let advanced_filter = AdvancedSequenceFilter {
        basic: query
            .filter
            .as_deref()
            .map(SequenceFilter::from_str)
            .unwrap_or(SequenceFilter::All),
        search: query.search.clone().filter(|s| !s.is_empty()),
        min_length: query.min_length,
        max_length: query.max_length,
        cog_category: query.cog.clone().filter(|s| !s.is_empty()),
        ec_class: query.ec_class.clone().filter(|s| !s.is_empty()),
        has_gene: query.has_gene,
        has_product: query.has_product,
    };

    let filter_str = advanced_filter.basic.as_str().to_string();

    let jobs = state.jobs();

    match jobs.get(&job_id) {
        Some(job) => {
            // Apply advanced filter to sequences
            let filtered_sequences: Vec<_> = job
                .sequences
                .as_ref()
                .map(|seqs| seqs.iter().filter(|s| advanced_filter.matches(s)).collect())
                .unwrap_or_default();

            let filtered_count = filtered_sequences.len();

            // Calculate pagination based on filtered results
            let pagination = PaginationInfo::new(page, per_page, filtered_count);

            // Get paginated sequences from filtered results
            let paginated_sequences = {
                let start = (page - 1) * per_page;
                let end = (start + per_page).min(filtered_count);
                if start < filtered_count {
                    filtered_sequences[start..end]
                        .iter()
                        .map(|s| (*s).clone())
                        .collect()
                } else {
                    Vec::new()
                }
            };

            let response = PaginatedJobResponse {
                job_id: job.job_id.clone(),
                status: job.status.clone(),
                created_at: job.created_at,
                updated_at: job.updated_at,
                filename: job.filename.clone(),
                sequence_count: job.sequence_count,
                processed_count: job.processed_count,
                hash_matches: job.hash_matches,
                alignment_matches: job.alignment_matches,
                error_message: job.error_message.clone(),
                sequences: paginated_sequences,
                pagination,
                filter: filter_str,
                filtered_count,
            };

            (StatusCode::OK, Json(response)).into_response()
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

/// Create a new annotation job
#[utoipa::path(
    post,
    path = "/api/job/",
    tag = "Jobs",
    request_body(
        content_type = "multipart/form-data",
        description = "FASTA file or content"
    ),
    responses(
        (status = 201, description = "Job created", body = JobCreateResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse)
    )
)]
pub async fn create_job(
    State(state): State<AppState>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Get or create owner ID from cookie
    let (owner_id, jar) = get_or_create_owner(jar);

    let mut temp_file = None;
    let mut filename: Option<String> = None;
    let mut job_name: Option<String> = None;
    let mut is_gzip_data = false;
    let mut total_bytes = 0usize;

    // Get temp directory
    let temp_dir = get_temp_dir();

    // Process multipart form data - stream directly to temp file
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();
        let file_name = field.file_name().map(|s| s.to_string());

        match field_name.as_str() {
            "file" | "fasta_content" => {
                if field_name == "file" {
                    filename = file_name;
                }

                // Create temp file in configured directory
                let tf = match TempFileBuilder::new()
                    .prefix("ai-db-upload-")
                    .suffix(".fasta")
                    .tempfile_in(&temp_dir)
                {
                    Ok(tf) => tf,
                    Err(e) => {
                        tracing::error!("Failed to create temp file in {:?}: {}", temp_dir, e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            jar,
                            Json(ErrorResponse::new(
                                "Failed to create temporary file for upload.",
                            )),
                        )
                            .into_response();
                    }
                };

                let mut file = tf;
                let mut first_chunk = true;

                // Stream chunks directly to temp file
                let mut stream = field;
                loop {
                    match stream.chunk().await {
                        Ok(Some(chunk)) => {
                            // Check for gzip magic bytes in first chunk
                            if first_chunk && chunk.len() >= 2 {
                                is_gzip_data = chunk[0] == 0x1f && chunk[1] == 0x8b;
                                first_chunk = false;
                            }

                            // Check size limit
                            if total_bytes + chunk.len() > MAX_UPLOAD_SIZE {
                                return (
                                    StatusCode::BAD_REQUEST,
                                    jar,
                                    Json(ErrorResponse::new(format!(
                                        "File too large. Maximum size is {} MB.",
                                        MAX_UPLOAD_SIZE / (1024 * 1024)
                                    ))),
                                )
                                    .into_response();
                            }

                            // Write chunk to temp file
                            if let Err(e) = file.write_all(&chunk) {
                                tracing::error!("Failed to write to temp file: {}", e);
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    jar,
                                    Json(ErrorResponse::new("Failed to save uploaded data.")),
                                )
                                    .into_response();
                            }

                            total_bytes += chunk.len();
                        }
                        Ok(None) => break,
                        Err(e) => {
                            return (
                                StatusCode::BAD_REQUEST,
                                jar,
                                Json(ErrorResponse::new(format!("Error reading upload: {}", e))),
                            )
                                .into_response();
                        }
                    }
                }

                // Flush and save the temp file
                if total_bytes > 0 {
                    if let Err(e) = file.flush() {
                        tracing::warn!("Failed to flush temp file: {}", e);
                    }
                    temp_file = Some(file);
                }
            }
            "job_name" => {
                if let Ok(data) = field.bytes().await {
                    if let Ok(name) = String::from_utf8(data.to_vec()) {
                        job_name = Some(name);
                    }
                }
            }
            _ => {
                // Consume and discard unknown fields
                let _ = field.bytes().await;
            }
        }
    }

    // Validate input
    let tf = match temp_file {
        Some(tf) => tf,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                jar,
                Json(ErrorResponse::new(
                    "No input received. Please upload a FASTA file or paste FASTA content.",
                )),
            )
                .into_response()
        }
    };

    tracing::info!(
        "Received upload: {} bytes, gzip: {}",
        total_bytes,
        is_gzip_data
    );

    // Create job
    let job_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let job = JobResponse {
        job_id: job_id.clone(),
        status: JobStatus::Pending,
        created_at: now,
        updated_at: now,
        filename: job_name
            .clone()
            .or(filename)
            .or(Some("direct_input".to_string())),
        sequence_count: 0,
        processed_count: 0,
        hash_matches: 0,
        alignment_matches: 0,
        sequences: None,
        error_message: None,
        owner_id: Some(owner_id.clone()),
    };

    // Store job (with persistence)
    state.save_job(&job);

    // Keep temp file path for background processing
    let temp_path = tf.into_temp_path();

    // Process job in background
    let state_clone = state.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        // Small delay to ensure job is stored
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Use blocking task for CPU-intensive work
        let state_for_blocking = state_clone.clone();
        let job_id_for_blocking = job_id_clone.clone();
        let path_clone = temp_path.to_path_buf();

        let result = tokio::task::spawn_blocking(move || {
            process_job_from_file(
                &state_for_blocking,
                &job_id_for_blocking,
                &path_clone,
                is_gzip_data,
            );
        })
            .await;

        // Clean up temp file
        if let Err(e) = temp_path.close() {
            tracing::warn!("Failed to clean up temp file: {}", e);
        }

        if let Err(e) = result {
            tracing::error!("Job processing panicked: {}", e);
        }
    });

    // Return response with cookie jar
    (
        StatusCode::CREATED,
        jar,
        Json(JobCreateResponse {
            job_id,
            status: JobStatus::Pending,
            message: "Job successfully created. Processing started.".to_string(),
            sequence_count: 0,
        }),
    )
        .into_response()
}

/// List all jobs (only own jobs based on cookie, paginated)
#[utoipa::path(
    get,
    path = "/api/jobs/",
    tag = "Jobs",
    params(
        ("page" = Option<usize>, Query, description = "Page (1-indexed, default: 1)"),
        ("per_page" = Option<usize>, Query, description = "Jobs per page (default: 20, max: 100)")
    ),
    responses(
        (status = 200, description = "Paginated job list", body = PaginatedJobsResponse)
    )
)]
pub async fn list_jobs(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<ListJobsQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .min(MAX_PER_PAGE)
        .max(1);

    // Get owner ID from cookie
    let owner_id = jar.get(OWNER_COOKIE_NAME).map(|c| c.value().to_string());

    let jobs = state.jobs();

    // Filter jobs by owner_id and collect as summaries
    let mut job_list: Vec<JobSummary> = jobs
        .values()
        .filter(|job| match (&job.owner_id, &owner_id) {
            (Some(job_owner), Some(cookie_owner)) => job_owner == cookie_owner,
            _ => false,
        })
        .map(JobSummary::from)
        .collect();

    // Sort by created_at descending
    job_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Calculate pagination
    let total_items = job_list.len();
    let pagination = PaginationInfo::new(page, per_page, total_items);

    // Apply pagination
    let start = (page - 1) * per_page;
    let paginated_jobs: Vec<JobSummary> = if start < job_list.len() {
        let end = (start + per_page).min(job_list.len());
        job_list[start..end].to_vec()
    } else {
        Vec::new()
    };

    Json(PaginatedJobsResponse {
        jobs: paginated_jobs,
        pagination,
    })
}

/// Delete a job (only own jobs)
#[utoipa::path(
    delete,
    path = "/api/job/{job_id}",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)")
    ),
    responses(
        (status = 204, description = "Job deleted"),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Job not found", body = ErrorResponse)
    )
)]
pub async fn delete_job(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let owner_id = jar.get(OWNER_COOKIE_NAME).map(|c| c.value().to_string());

    let jobs = state.jobs_mut();

    // First check if job exists and belongs to owner
    if let Some(job) = jobs.get(&job_id) {
        let is_owner = match (&job.owner_id, &owner_id) {
            (Some(job_owner), Some(cookie_owner)) => job_owner == cookie_owner,
            _ => false,
        };

        if !is_owner {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("Not authorized to delete this job")),
            )
                .into_response();
        } else {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(format!(
                    "Job with ID '{}' not found",
                    job_id
                ))),
            )
                .into_response();
        }
    }

    // Delete with persistence
    if state.delete_job(&job_id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!(
                "Job with ID '{}' not found",
                job_id
            ))),
        )
            .into_response()
    }
}
