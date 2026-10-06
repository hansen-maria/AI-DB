//! AI-DB REST API Backend
//!
//! Hash-Based Annotation Service for Microbial Sequencing Data
//!
//! # Module Structure
//!
//! - `models` - Data structures (Job, Sequence, Pagination, Error)
//! - `handlers` - API endpoint handlers (jobs, download, health)
//! - `services` - Business logic (FASTA parsing, annotation)
//! - `export` - Export formats (TSV, JSON, FASTA, GFF3)
//! - `state` - Application state and database connection
//! - `auth` - Owner authentication (cookie or API token) and read-only sharing
//! - `storage` - Logic to persist jobs for 30 days using SQLite

pub mod auth;
pub mod export;
pub mod handlers;
pub mod models;
pub mod services;
pub mod state;
pub mod storage;

use axum::extract::DefaultBodyLimit;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::{
    bulk_delete_jobs, create_job, db_info, delete_bakta_job, delete_job, delete_psos_results,
    download_job, export_job_stats, get_bakta_job, get_job, get_job_stats, get_kpi_overview,
    get_psos_results, get_sequence, health_check, ingest_bakta_results, list_jobs, rename_job,
    retry_job, save_bakta_job, save_psos_results,
};
use crate::models::{
    BaktaJobStateResponse, BulkDeleteRequest, BulkDeleteResponse, CustomAnnotationEntry,
    ErrorResponse, FunctionalStats, IngestCustomAnnotationsRequest,
    IngestCustomAnnotationsResponse, JobCreateResponse, JobResponse, JobStatus, JobSummary,
    PaginatedJobResponse, PaginatedJobsResponse, PaginationInfo, PsosResult, PsosResultsResponse,
    RenameJobRequest, SaveBaktaJobRequest, SaveBaktaJobResponse, SavePsosResultsRequest,
    SavePsosResultsResponse, SequenceInfo, StoredBaktaJob,
};
use crate::state::AppState;

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    info(
        title = "AI-DB REST API",
        version = "1.0.0",
        description = "Hash-Based Annotation Service for Microbial Sequencing Data\n\n\
            AI-DB accelerates microbial sequencing data analysis while preserving data \
            sovereignty through cryptographic hash-based annotations.\n\n\
            ## Features\n\n\
            - **Privacy**: Sequence data processed as MD5 hashes\n\
            - **Fast**: Hash-based annotations in seconds instead of hours\n\
            - **Comprehensive**: Access to Bakta UniRef protein annotations (~350M sequences)\n\
            - **Fallback**: LookUp in the AI-DB database\n\n\
            ## Authentication\n\n\
            Jobs belong to a random identifier (UUIDv4). Browsers receive it automatically \
            as an HTTP-only cookie. Scripts and pipelines generate their own UUIDv4 \
            (e.g. `uuidgen`) and send it with **every** request as \
            `Authorization: Bearer <token>` or `X-API-Key: <token>` (no cookies needed). \
            Keep the token secret: it is the key to your jobs.\n\n\
            Anyone who knows a job ID can **view** that job (read-only). Downloading, \
            renaming, deleting, retrying, and Psos/Bakta analysis state require the owner.",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
        contact(name = "AI-DB Team", url = "https://github.com/hansen-maria/AI-DB-Web")
    ),
    tags(
        (name = "Jobs", description = "Annotation job management - create and query jobs (viewing by job ID, changes owner-only)"),
        (name = "psos", description = "Psos analysis results storage"),
        (name = "bakta", description = "Bakta job state persistence"),
        (name = "admin", description = "Admin-only endpoints (shared-secret protected)"),
        (name = "Health", description = "Health check and database info")
    ),
    paths(
        handlers::jobs::get_job,
        handlers::jobs::create_job,
        handlers::jobs::list_jobs,
        handlers::jobs::delete_job,
        handlers::jobs::rename_job,
        handlers::jobs::bulk_delete_jobs,
        handlers::jobs::get_sequence,
        handlers::jobs::retry_job,
        handlers::download::download_job,
        handlers::stats::get_job_stats,
        handlers::stats::export_job_stats,
        handlers::psos::save_psos_results,
        handlers::psos::get_psos_results,
        handlers::psos::delete_psos_results,
        handlers::bakta::save_bakta_job,
        handlers::bakta::get_bakta_job,
        handlers::bakta::delete_bakta_job,
        handlers::bakta::ingest_bakta_results,
        handlers::kpi::get_kpi_overview,
        handlers::kpi::list_admin_annotations,
        handlers::kpi::review_admin_annotation,
        handlers::health::health_check,
        handlers::health::db_info
    ),
    components(schemas(
        JobStatus,
        SequenceInfo,
        JobResponse,
        JobCreateResponse,
        RenameJobRequest,
        BulkDeleteRequest,
        BulkDeleteResponse,
        ErrorResponse,
        PaginationInfo,
        PaginatedJobsResponse,
        JobSummary,
        PaginatedJobResponse,
        crate::handlers::jobs::JobViewResponse,
        FunctionalStats,
        PsosResult,
        PsosResultsResponse,
        SavePsosResultsRequest,
        SavePsosResultsResponse,
        StoredBaktaJob,
        SaveBaktaJobRequest,
        SaveBaktaJobResponse,
        BaktaJobStateResponse,
        CustomAnnotationEntry,
        IngestCustomAnnotationsRequest,
        IngestCustomAnnotationsResponse,
        crate::handlers::kpi::KpiMonthEntry,
        crate::handlers::kpi::KpiOverviewResponse,
        crate::handlers::kpi::AdminAnnotationEntry,
        crate::handlers::kpi::AdminAnnotationsResponse,
        crate::handlers::kpi::ReviewAnnotationRequest,
        crate::handlers::kpi::ReviewAnnotationResponse,
    ))
)]
struct ApiDoc;

/// Registers the owner-token schemes (`bearer_token`, `api_key`) referenced by
/// the `security(...)` attributes of the owner-only endpoints.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_token",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("UUIDv4")
                    .description(Some(
                        "Self-generated random UUIDv4 identifying the job owner",
                    ))
                    .build(),
            ),
        );
        components.add_security_scheme(
            "api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "X-API-Key",
                "Alternative to the bearer token: the same UUIDv4 in an X-API-Key header",
            ))),
        );
        components.add_security_scheme(
            "admin_secret",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "X-Admin-Secret",
                "Shared admin secret (ADMIN_KPI_SECRET) for KPI and curation review routes",
            ))),
        );
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let state = AppState::new();

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::mirror_request())
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderName::from_static("x-api-key"),
        ])
        .allow_credentials(true);

    // Build router
    let app = Router::new()
        // Health & info routes
        .route("/api/health", get(health_check))
        .route("/api/db/info", get(db_info))
        // Job management routes
        .route("/api/job/", post(create_job))
        .route(
            "/api/job/{job_id}",
            get(get_job).delete(delete_job).patch(rename_job),
        )
        .route("/api/job/{job_id}/download/{format}", get(download_job))
        .route("/api/job/{job_id}/stats", get(get_job_stats))
        .route("/api/job/{job_id}/stats/export", get(export_job_stats))
        .route("/api/job/{job_id}/retry", post(retry_job))
        .route("/api/job/{job_id}/sequence/{seq_id}", get(get_sequence))
        // Psos results routes
        .route(
            "/api/job/{job_id}/psos",
            get(get_psos_results)
                .post(save_psos_results)
                .delete(delete_psos_results),
        )
        // Bakta job state routes
        .route(
            "/api/job/{job_id}/bakta",
            get(get_bakta_job)
                .post(save_bakta_job)
                .delete(delete_bakta_job),
        )
        // Bakta → custom annotations ingest
        .route("/api/job/{job_id}/bakta/ingest", post(ingest_bakta_results))
        .route("/api/jobs/", get(list_jobs).delete(bulk_delete_jobs))
        // Admin KPI overview (shared-secret protected, see handlers::kpi)
        .route("/api/admin/kpis", get(get_kpi_overview))
        // Admin review of community-curated annotations (same shared secret)
        .route(
            "/api/admin/annotations",
            get(crate::handlers::kpi::list_admin_annotations),
        )
        .route(
            "/api/admin/annotations/{md5}/review",
            post(crate::handlers::kpi::review_admin_annotation),
        )
        // Swagger UI
        .merge(SwaggerUi::new("/api/docs/").url("/api/openapi.json", ApiDoc::openapi()))
        // Middleware
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100 MB Limit
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
