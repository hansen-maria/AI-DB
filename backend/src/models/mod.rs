//! ============================================================================
//! Data models for the AI-DB API
//! ============================================================================

pub mod error;
pub mod health;
pub mod job;
pub mod pagination;
pub mod sequence;
pub mod stats;

pub use error::ErrorResponse;
pub use health::{BaktaDbHealth, DbInfoResponse, HealthCheckResponse};
pub use job::{JobCreateResponse, JobResponse, JobStatus, JobSummary};
pub use pagination::{
    GetJobQuery, ListJobsQuery, PaginatedJobResponse, PaginatedJobsResponse, PaginationInfo,
    DEFAULT_PER_PAGE, MAX_PER_PAGE,
};
pub use sequence::{AdvancedSequenceFilter, HashLookupResult, SequenceFilter, SequenceInfo};
pub use stats::{cog_category_name, ec_class_name, CogCategory, CountItem, FunctionalStats, GoTermStats};
