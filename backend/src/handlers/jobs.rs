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
    AdvancedSequenceFilter, BulkDeleteRequest, BulkDeleteResponse, ErrorResponse, GetJobQuery,
    JobCreateResponse, JobResponse, JobStatus, JobSummary, PaginatedJobResponse,
    PaginatedJobsResponse, PaginationInfo, RenameJobRequest, SequenceFilter, SequenceInfo,
    DEFAULT_PER_PAGE, MAX_PER_PAGE,
};
use crate::services::process_job_from_file;
use crate::services::reannotate_sequences;
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
        .clamp(1, MAX_PER_PAGE);

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

/// Query parameters for listing jobs – defined locally so no change to models is required.
#[derive(Debug, serde::Deserialize)]
pub struct ListJobsParams {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    /// Optional status filter: pending | processing | completed | failed
    pub status: Option<String>,
    /// Optional case-insensitive substring search on filename
    pub search: Option<String>,
}

/// List all jobs (only own jobs based on cookie, paginated)
#[utoipa::path(
    get,
    path = "/api/jobs/",
    tag = "Jobs",
    params(
        ("page"     = Option<usize>,  Query, description = "Page (1-indexed, default: 1)"),
        ("per_page" = Option<usize>,  Query, description = "Jobs per page (default: 20, max: 100)"),
        ("status"   = Option<String>, Query, description = "Filter by status: pending, processing, completed, failed"),
        ("search"   = Option<String>, Query, description = "Search in filename (case-insensitive)")
    ),
    responses(
        (status = 200, description = "Paginated job list", body = PaginatedJobsResponse)
    )
)]
pub async fn list_jobs(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<ListJobsParams>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);

    // Get owner ID from cookie
    let owner_id = jar.get(OWNER_COOKIE_NAME).map(|c| c.value().to_string());

    let status_filter = query.status.as_deref().map(|s| s.to_lowercase());
    let search_filter = query.search.as_deref().map(|s| s.to_lowercase());

    let jobs = state.jobs();

    // Filter jobs by owner_id and collect as summaries
    let mut job_list: Vec<JobSummary> = jobs
        .values()
        // Ownership check
        .filter(|job| match (&job.owner_id, &owner_id) {
            (Some(job_owner), Some(cookie_owner)) => job_owner == cookie_owner,
            _ => false,
        })
        // Status filter
        .filter(|job| {
            if let Some(ref status) = status_filter {
                let job_status = match job.status {
                    JobStatus::Pending => "pending",
                    JobStatus::Processing => "processing",
                    JobStatus::Completed => "completed",
                    JobStatus::Failed => "failed",
                };
                job_status == status.as_str()
            } else {
                true
            }
        })
        // Filename search
        .filter(|job| {
            if let Some(ref search) = search_filter {
                job.filename
                    .as_ref()
                    .map(|f| f.to_lowercase().contains(search.as_str()))
                    .unwrap_or(false)
            } else {
                true
            }
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

    // Check existence first (read lock)
    {
        let jobs = state.jobs();
        if let Some(job) = jobs.get(&job_id) {
            // Only block if BOTH sides have an owner_id AND they differ.
            // If the cookie is missing (None), allow deletion – same policy as get_job.
            if let (Some(job_owner), Some(cookie_owner)) = (&job.owner_id, &owner_id) {
                if job_owner != cookie_owner {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(ErrorResponse::new("Not authorized to delete this job")),
                    )
                        .into_response();
                }
            }
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

// ── Rename job ────────────────────────────────────────────────────────────────

/// Rename a job (change its display filename)
#[utoipa::path(
    patch,
    path = "/api/job/{job_id}",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)")
    ),
    request_body = RenameJobRequest,
    responses(
        (status = 200,  description = "Job renamed",      body = JobSummary),
        (status = 400,  description = "Empty filename",   body = ErrorResponse),
        (status = 403,  description = "Not authorized",   body = ErrorResponse),
        (status = 404,  description = "Job not found",    body = ErrorResponse)
    )
)]
pub async fn rename_job(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(job_id): Path<String>,
    Json(body): Json<RenameJobRequest>,
) -> impl IntoResponse {
    let owner_id = jar.get(OWNER_COOKIE_NAME).map(|c| c.value().to_string());

    let new_filename = body.filename.trim().to_string();
    if new_filename.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("Filename must not be empty")),
        )
            .into_response();
    }

    // Clone job under read lock (avoids holding lock across await / DB call)
    let updated_job = {
        let jobs = state.jobs();
        match jobs.get(&job_id) {
            Some(job) => {
                if let (Some(job_owner), Some(cookie_owner)) = (&job.owner_id, &owner_id) {
                    if job_owner != cookie_owner {
                        return (
                            StatusCode::FORBIDDEN,
                            Json(ErrorResponse::new("Not authorized to rename this job")),
                        )
                            .into_response();
                    }
                }
                let mut updated = job.clone();
                updated.filename = Some(new_filename);
                updated.updated_at = Utc::now();
                updated
            }
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(format!(
                        "Job with ID '{}' not found",
                        job_id
                    ))),
                )
                    .into_response()
            }
        }
        // read lock released here
    };

    // Persist to DB + update in-memory cache
    state.save_job(&updated_job);

    (StatusCode::OK, Json(JobSummary::from(&updated_job))).into_response()
}

// ── Bulk delete ───────────────────────────────────────────────────────────────

/// Delete multiple jobs in one request
#[utoipa::path(
    delete,
    path = "/api/jobs/",
    tag = "Jobs",
    request_body = BulkDeleteRequest,
    responses(
        (status = 200, description = "Bulk delete result", body = BulkDeleteResponse)
    )
)]
pub async fn bulk_delete_jobs(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<BulkDeleteRequest>,
) -> impl IntoResponse {
    let owner_id = jar.get(OWNER_COOKIE_NAME).map(|c| c.value().to_string());

    let mut deleted = Vec::new();
    let mut not_found = Vec::new();
    let mut forbidden = Vec::new();

    for job_id in &body.job_ids {
        // Ownership check under read lock
        let auth_result = {
            let jobs = state.jobs();
            match jobs.get(job_id.as_str()) {
                Some(job) => match (&job.owner_id, &owner_id) {
                    (Some(job_owner), Some(cookie_owner)) if job_owner != cookie_owner => {
                        Err("forbidden")
                    }
                    _ => Ok(()),
                },
                None => Err("not_found"),
            }
        };

        match auth_result {
            Ok(()) => {
                state.delete_job(job_id);
                deleted.push(job_id.clone());
            }
            Err("not_found") => not_found.push(job_id.clone()),
            _ => forbidden.push(job_id.clone()),
        }
    }

    // 207 Multi-Status when some IDs failed; 200 when everything succeeded
    let status = if not_found.is_empty() && forbidden.is_empty() {
        StatusCode::OK
    } else {
        StatusCode::MULTI_STATUS
    };

    (
        status,
        Json(BulkDeleteResponse {
            deleted,
            not_found,
            forbidden,
        }),
    )
        .into_response()
}

// ── Single sequence detail ────────────────────────────────────────────────────

/// Get full details of a single sequence within a job
#[utoipa::path(
    get,
    path = "/api/job/{job_id}/sequence/{seq_id}",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)"),
        ("seq_id" = String, Path, description = "Sequence identifier (FASTA header ID)")
    ),
    responses(
        (status = 200, description = "Sequence found",    body = SequenceInfo),
        (status = 404, description = "Not found",         body = ErrorResponse)
    )
)]
pub async fn get_sequence(
    State(state): State<AppState>,
    Path((job_id, seq_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let jobs = state.jobs();

    match jobs.get(&job_id) {
        Some(job) => {
            match job
                .sequences
                .as_ref()
                .and_then(|seqs| seqs.iter().find(|s| s.id == seq_id))
            {
                Some(seq) => (StatusCode::OK, Json(seq.clone())).into_response(),
                None => (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(format!(
                        "Sequence '{}' not found in job '{}'",
                        seq_id, job_id
                    ))),
                )
                    .into_response(),
            }
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

// ── Retry failed job ──────────────────────────────────────────────────────────

/// Retry a failed job using the sequences already stored in the database.
///
/// The original FASTA file is no longer available after processing, so this
/// re-runs the hash lookups against the current state of the Bakta / AI-DB
/// databases.  Returns 422 if no sequences are stored (e.g. the job failed
/// before parsing completed – in that case the file must be re-uploaded).
#[utoipa::path(
    post,
    path = "/api/job/{job_id}/retry",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)")
    ),
    responses(
        (status = 202, description = "Retry started",               body = JobSummary),
        (status = 400, description = "Job is not in failed state",  body = ErrorResponse),
        (status = 403, description = "Not authorized",              body = ErrorResponse),
        (status = 404, description = "Job not found",               body = ErrorResponse),
        (status = 422, description = "No sequences to retry",       body = ErrorResponse)
    )
)]
pub async fn retry_job(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let owner_id = jar.get(OWNER_COOKIE_NAME).map(|c| c.value().to_string());

    // ── Validate ──────────────────────────────────────────────────────────────
    let sequences = {
        let jobs = state.jobs();
        match jobs.get(&job_id) {
            Some(job) => {
                // Ownership
                if let (Some(job_owner), Some(cookie_owner)) = (&job.owner_id, &owner_id) {
                    if job_owner != cookie_owner {
                        return (
                            StatusCode::FORBIDDEN,
                            Json(ErrorResponse::new("Not authorized to retry this job")),
                        )
                            .into_response();
                    }
                }
                // Must be failed
                if job.status != JobStatus::Failed {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse::new(
                            "Only jobs with status 'failed' can be retried",
                        )),
                    )
                        .into_response();
                }
                // Must have stored sequences
                match &job.sequences {
                    Some(seqs) if !seqs.is_empty() => seqs.clone(),
                    _ => {
                        return (
                            StatusCode::UNPROCESSABLE_ENTITY,
                            Json(ErrorResponse::new(
                                "Cannot retry: no sequences are stored for this job. \
                                 Please re-upload the FASTA file.",
                            )),
                        )
                            .into_response()
                    }
                }
            }
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(format!(
                        "Job with ID '{}' not found",
                        job_id
                    ))),
                )
                    .into_response()
            }
        }
        // read lock released here
    };

    // ── Reset status to Processing ────────────────────────────────────────────
    let (reset_summary, job_to_save) = {
        let mut jobs = state.jobs_mut();
        match jobs.get_mut(&job_id) {
            Some(job) => {
                job.status = JobStatus::Processing;
                job.error_message = None;
                job.processed_count = 0;
                job.hash_matches = 0;
                job.updated_at = Utc::now();
                (JobSummary::from(&*job), job.clone())
            }
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("Job disappeared during retry setup")),
                )
                    .into_response()
            }
        }
        // write lock released here
    };

    state.save_job(&job_to_save);

    // ── Spawn background re-annotation ────────────────────────────────────────
    let state_clone = state.clone();
    let job_id_clone = job_id.clone();

    tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            reannotate_sequences(&state_clone, &job_id_clone, sequences);
        })
        .await;

        if let Err(e) = result {
            tracing::error!("Retry task panicked for job {}: {}", job_id, e);
        }
    });

    (StatusCode::ACCEPTED, Json(reset_summary)).into_response()
}
