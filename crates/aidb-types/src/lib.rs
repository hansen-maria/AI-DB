//! ============================================================================
//! aidb-types – shared request/response types of the AI-DB REST API
//!
//! Used by the backend (`ai-db-api`, with feature `openapi`) and by API
//! clients (`aidb-client`, `aidb-cli`). Keeping both sides on the same types
//! means a breaking change to the API shows up as a compile error in the
//! client crates of the same workspace.
//!
//! Features:
//!   * `openapi` – derives `utoipa::ToSchema` for the OpenAPI documentation
//!     (server only; clients do not need utoipa).
//!
//! ============================================================================

pub mod auth;
pub mod bakta;
pub mod custom_db;
pub mod error;
pub mod health;
pub mod job;
pub mod naming;
pub mod pagination;
pub mod psos;
pub mod sequence;
pub mod stats;

/// Version of the HTTP API contract with external clients (CLI, scripts),
/// reported by `GET /api/health`. Clients compare it with their own
/// compiled-in value to detect incompatible servers.
///
/// The contract covers `/api/health`, `/api/db/info`, `POST /api/job/`,
/// `/api/job/{id}` (GET, PATCH, DELETE), `/api/job/{id}/retry`,
/// `/download/{format}`, `/stats`, `/stats/export`, `/sequence/{seq_id}` and
/// `/api/jobs/` (GET, DELETE).
///
/// Bump it for incompatible changes to these routes:
/// * a route is removed or renamed;
/// * a required response field is removed, renamed or changes type or meaning;
/// * a new required request parameter;
/// * an existing value changes its meaning;
/// * a new value in a closed set, e.g. another `JobStatus`.
///
/// Do not bump it for:
/// * new routes, fields or optional parameters;
/// * new values in open string fields (`annotation_source`, filters such as
///   `bakrep_db`);
/// * removing optional response fields (e.g. `annotation_status`);
/// * routes used only by the web frontend, which is deployed together with
///   the backend (`/bakta*`, `/psos*`, `/api/auth/token`) – once the CLI uses
///   Bakta/Psos routes, they become part of the contract;
/// * admin routes (`/api/admin/*`).
///
/// Clients ignore unknown fields and read optional fields with defaults.
pub const API_VERSION: u32 = 1;

pub use auth::AuthTokenResponse;
pub use bakta::{BaktaJobStateResponse, SaveBaktaJobRequest, SaveBaktaJobResponse, StoredBaktaJob};
pub use custom_db::{
    CustomAnnotationEntry, IngestCustomAnnotationsRequest, IngestCustomAnnotationsResponse,
};
pub use error::ErrorResponse;
pub use health::{
    AidbDbInfo, BakrepDbHealth, BakrepDbInfo, BaktaDbHealth, DbInfoResponse, HealthCheckResponse,
};
pub use job::{
    BulkDeleteRequest, BulkDeleteResponse, JobCreateResponse, JobResponse, JobStatus, JobSummary,
    RenameJobRequest,
};
pub use naming::{result_base_name, result_file_name};
pub use pagination::{
    GetJobQuery, JobViewResponse, ListJobsQuery, PaginatedJobResponse, PaginatedJobsResponse,
    PaginationInfo, SequenceFilterQuery, DEFAULT_PER_PAGE, MAX_PER_PAGE,
};
pub use psos::{PsosResult, PsosResultsResponse, SavePsosResultsRequest, SavePsosResultsResponse};
pub use sequence::{SequenceInfo, SOURCE_AIDB_DB, SOURCE_BAKREP_DB, SOURCE_BAKTA_DB};
pub use stats::{
    cog_category_name, ec_class_name, CogCategory, CountItem, FunctionalStats, GoTermStats,
};
