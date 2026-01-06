//! AI-DB REST API Backend
//! Hash-Based Annotation Service for Microbial Sequencing Data

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use md5::{Digest, Md5};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

// ============================================================================
// Data Models
// ============================================================================

/// Status of an annotation job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Information about the sequence
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SequenceInfo {
    /// Sequence identifier
    pub id: String,
    /// MD5-Hash
    pub md5_hash: String,
    /// Length in bp / aa
    pub length: usize,
    /// Found annotations
    pub annotation: Option<String>,
    /// Source of annotation
    pub annotation_source: Option<String>,
}

/// Job details and results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobResponse {
    /// Unique job ID (UUID)
    pub job_id: String,
    /// Current status of the job
    pub status: JobStatus,
    /// Timestamp of creation
    pub created_at: DateTime<Utc>,
    /// Last updated at
    pub updated_at: DateTime<Utc>,
    /// Name of the uploaded file
    pub filename: Option<String>,
    /// Number of sequences
    pub sequence_count: usize,
    /// Number of processed sequences
    pub processed_count: usize,
    /// Number of hash matches
    pub hash_matches: usize,
    /// Number of alignment matches
    pub alignment_matches: usize,
    /// Details about the sequences
    pub sequences: Option<Vec<SequenceInfo>>,
    /// Error messages (if an error occurred)
    pub error_message: Option<String>,
}

/// Response after job creation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobCreateResponse {
    /// Unique job ID (UUID)
    pub job_id: String,
    /// Initial status (pending)
    pub status: JobStatus,
    /// Confirmation message
    pub message: String,
    /// Number of found sequences
    pub sequence_count: usize,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Error description
    pub detail: String,
}

/// Query parameters for job lists
#[derive(Debug, Deserialize)]
pub struct ListJobsQuery {
    limit: Option<usize>,
}

// ============================================================================
// Application State
// ============================================================================

#[derive(Clone)]
pub struct AppState {
    jobs: Arc<RwLock<HashMap<String, JobResponse>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parses FASTA content and returns a list of (header, sequence) tuples
fn parse_fasta(content: &str) -> Vec<(String, String)> {
    let mut sequences = Vec::new();
    let mut current_header: Option<String> = None;
    let mut current_sequence = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('>') {
            if let Some(header) = current_header.take() {
                if !current_sequence.is_empty() {
                    sequences.push((header, current_sequence.clone()));
                }
            }
            current_header = Some(
                line[1..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string(),
            );
            current_sequence.clear();
        } else {
            current_sequence.push_str(&line.to_uppercase());
        }
    }

    if let Some(header) = current_header {
        if !current_sequence.is_empty() {
            sequences.push((header, current_sequence));
        }
    }

    sequences
}

/// Calculates the MD5 hash of a sequence
fn compute_md5(sequence: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(sequence.as_bytes());
    hex::encode(hasher.finalize())
}

/// Simulates annotation lookup
fn simulate_annotation(seq_hash: &str) -> (Option<String>, String) {
    let hash_int = u32::from_str_radix(&seq_hash[..8], 16).unwrap_or(0);

    if hash_int % 10 < 7 {
        let annotations = [
            "Hypothetical protein",
            "ATP synthase subunit alpha",
            "DNA polymerase III subunit beta",
            "Ribosomal protein S12",
            "Chaperone protein DnaK",
            "Elongation factor Tu",
            "RNA polymerase sigma factor",
        ];
        let annotation = annotations[(hash_int as usize) % annotations.len()];
        (Some(annotation.to_string()), "hash_match".to_string())
    } else if hash_int % 10 < 9 {
        (
            Some("Putative membrane protein".to_string()),
            "alignment".to_string(),
        )
    } else {
        (None, "no_match".to_string())
    }
}

/// Processing a job (simulated)
fn process_job(state: &AppState, job_id: &str, sequences: Vec<(String, String)>) {
    // Set status to processing
    {
        let mut jobs = state.jobs.write();
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Processing;
            job.updated_at = Utc::now();
        }
    }

    let mut sequence_infos = Vec::new();
    let mut hash_matches = 0;
    let mut alignment_matches = 0;

    for (header, seq) in &sequences {
        let seq_hash = compute_md5(seq);
        let (annotation, source) = simulate_annotation(&seq_hash);

        if source == "hash_match" {
            hash_matches += 1;
        } else if source == "alignment" {
            alignment_matches += 1;
        }

        sequence_infos.push(SequenceInfo {
            id: header.clone(),
            md5_hash: seq_hash,
            length: seq.len(),
            annotation,
            annotation_source: if source != "no_match" {
                Some(source)
            } else {
                None
            },
        });
    }

    // Update job with results
    {
        let mut jobs = state.jobs.write();
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Completed;
            job.updated_at = Utc::now();
            job.processed_count = sequences.len();
            job.hash_matches = hash_matches;
            job.alignment_matches = alignment_matches;
            job.sequences = Some(sequence_infos);
        }
    }
}

// ============================================================================
// API Handlers
// ============================================================================

/// Check job status and results
#[utoipa::path(
    get,
    path = "/api/job/{job_id}",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)")
    ),
    responses(
        (status = 200, description = "Job found", body = JobResponse),
        (status = 404, description = "Job not found", body = ErrorResponse)
    )
)]
async fn get_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let jobs = state.jobs.read();

    match jobs.get(&job_id) {
        Some(job) => (StatusCode::OK, Json(serde_json::to_value(job).unwrap())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                detail: format!("Job with ID '{}' not found", job_id),
            }),
        )
            .into_response(),
    }
}

/// Create a new annotation job
#[utoipa::path(
    post,
    path = "/api/job/",
    tag = "Jobs",
    request_body(content_type = "multipart/form-data", content = String, description = "FASTA file or FASTA content"),
    responses(
        (status = 201, description = "Job created", body = JobCreateResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse)
    )
)]
async fn create_job(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut fasta_content: Option<String> = None;
    let mut filename: Option<String> = None;
    let mut job_name: Option<String> = None;

    // Process multipart form data
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();
        let file_name = field.file_name().map(|s| s.to_string());

        match field.bytes().await {
            Ok(data) => match field_name.as_str() {
                "file" => {
                    filename = file_name;
                    if let Ok(content) = String::from_utf8(data.to_vec()) {
                        if !content.is_empty() {
                            fasta_content = Some(content);
                        }
                    }
                }
                "fasta_content" => {
                    if let Ok(content) = String::from_utf8(data.to_vec()) {
                        if !content.is_empty() {
                            fasta_content = Some(content);
                        }
                    }
                }
                "job_name" => {
                    if let Ok(name) = String::from_utf8(data.to_vec()) {
                        job_name = Some(name);
                    }
                }
                _ => {}
            },
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        detail: format!("Error reading data: {}", e),
                    }),
                )
                    .into_response()
            }
        }
    }

    // Validate input
    let content = match fasta_content {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    detail: "No input received. Please upload FASTA file or send FASTA content.".to_string(),
                }),
            )
                .into_response()
        }
    };

    // Parse FASTA
    let sequences = parse_fasta(&content);

    if sequences.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                detail: "No valid sequences found in the input.".to_string(),
            }),
        )
            .into_response();
    }

    // Create job
    let job_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let sequence_count = sequences.len();

    let job = JobResponse {
        job_id: job_id.clone(),
        status: JobStatus::Pending,
        created_at: now,
        updated_at: now,
        filename: filename
            .or(job_name.clone())
            .or(Some("direct_input".to_string())),
        sequence_count,
        processed_count: 0,
        hash_matches: 0,
        alignment_matches: 0,
        sequences: None,
        error_message: None,
    };

    // Store job
    {
        let mut jobs = state.jobs.write();
        jobs.insert(job_id.clone(), job);
    }

    // Process job in background
    let state_clone = state.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        // Simulate some processing delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        process_job(&state_clone, &job_id_clone, sequences);
    });

    (
        StatusCode::CREATED,
        Json(JobCreateResponse {
            job_id,
            status: JobStatus::Pending,
            message: "Job successfully created. Processing started.".to_string(),
            sequence_count,
        }),
    )
        .into_response()
}

/// List all jobs
#[utoipa::path(
    get,
    path = "/api/jobs/",
    tag = "Jobs",
    params(
        ("limit" = Option<usize>, Query, description = "Maximum number of jobs")
    ),
    responses(
        (status = 200, description = "List of jobs", body = Vec<JobResponse>)
    )
)]
async fn list_jobs(
    State(state): State<AppState>,
    Query(query): Query<ListJobsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100);

    let jobs = state.jobs.read();
    let mut job_list: Vec<JobResponse> = jobs.values().cloned().collect();

    // Sort by created_at descending
    job_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Apply limit
    job_list.truncate(limit);

    Json(job_list)
}

/// Delete a job by ID
#[utoipa::path(
    delete,
    path = "/api/job/{job_id}",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)")
    ),
    responses(
        (status = 204, description = "Job deleted"),
        (status = 404, description = "Job not found", body = ErrorResponse)
    )
)]
async fn delete_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let mut jobs = state.jobs.write();

    match jobs.remove(&job_id) {
        Some(_) => StatusCode::NO_CONTENT.into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                detail: format!("Job with ID ‘{}’ not found", job_id),
            }),
        )
            .into_response(),
    }
}

/// Health Check
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "ai-db-api"
    }))
}

// ============================================================================
// OpenAPI Documentation
// ============================================================================

#[derive(OpenApi)]
#[openapi(
    info(
        title = "AI-DB REST API",
        version = "1.0.0",
        description = "Hash-Based Annotation Service for Microbial Sequencing Data\n\n\
            AI-DB accelerates the analysis of microbial sequencing data while \
            maintaining data sovereignty through cryptographic hash-based annotations.\n\n\
            ## Features\n\n\
            - **Data protection**: Sequence data is processed as MD5 hashes\n\
            - **Fast**: Hash-based annotations in seconds instead of hours\n\
            - **Comprehensive**: Access to UniRef protein annotations\n\
            - **Fallback**: Diamond alignment for new sequences",
        license(name = "MIT"),
        contact(name = "AI-DB Team", url = "https://github.com/hansen-maria/AI-DB-Web")
    ),
    tags(
        (name = "Jobs", description = "Annotation Job Management - Creating and Querying Jobs")
    ),
    paths(get_job, create_job, list_jobs, delete_job),
    components(schemas(
        JobStatus,
        SequenceInfo,
        JobResponse,
        JobCreateResponse,
        ErrorResponse
    ))
)]
struct ApiDoc;

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let state = AppState::new();

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // API routes
        .route("/api/health", get(health_check))
        .route("/api/job/", post(create_job))
        .route("/api/job/:job_id", get(get_job).delete(delete_job))
        .route("/api/jobs/", get(list_jobs))
        // Swagger UI
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", ApiDoc::openapi()))
        // Middleware
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // State
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("Starting AI-DB API server on http://{}", addr);
    tracing::info!("Swagger UI available at http://{}/api/docs/", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}