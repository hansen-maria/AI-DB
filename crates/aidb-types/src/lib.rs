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
//! ============================================================================

pub mod auth;
pub mod bakta;
pub mod custom_db;
pub mod error;
pub mod health;
pub mod job;
pub mod pagination;
pub mod psos;
pub mod sequence;
pub mod stats;

/// Version of the HTTP API contract, reported by `GET /api/health`.
///
/// Bump this when a change is incompatible for existing clients (a removed or
/// renamed route or field, a changed meaning). Additive changes (new routes,
/// new optional fields) do not require a bump. Clients compare it with their
/// own compiled-in value to detect incompatible servers.
pub const API_VERSION: u32 = 1;

pub use auth::AuthTokenResponse;
pub use bakta::{BaktaJobStateResponse, SaveBaktaJobRequest, SaveBaktaJobResponse, StoredBaktaJob};
pub use custom_db::{
    CustomAnnotationEntry, IngestCustomAnnotationsRequest, IngestCustomAnnotationsResponse,
};
pub use error::ErrorResponse;
pub use health::{AidbDbInfo, BaktaDbHealth, DbInfoResponse, HealthCheckResponse};
pub use job::{
    BulkDeleteRequest, BulkDeleteResponse, JobCreateResponse, JobResponse, JobStatus, JobSummary,
    RenameJobRequest,
};
pub use pagination::{
    GetJobQuery, ListJobsQuery, PaginatedJobResponse, PaginatedJobsResponse, PaginationInfo,
    DEFAULT_PER_PAGE, MAX_PER_PAGE,
};
pub use psos::{PsosResult, PsosResultsResponse, SavePsosResultsRequest, SavePsosResultsResponse};
pub use sequence::{SequenceInfo, SOURCE_AIDB_DB, SOURCE_BAKTA_DB};
pub use stats::{
    cog_category_name, ec_class_name, CogCategory, CountItem, FunctionalStats, GoTermStats,
};
