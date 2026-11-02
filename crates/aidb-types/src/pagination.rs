//! ============================================================================
//! Pagination-related models
//! ============================================================================

use serde::{Deserialize, Serialize};

use super::job::{JobStatus, JobSummary};
use super::sequence::SequenceInfo;
use chrono::{DateTime, Utc};

/// Default items per page
pub const DEFAULT_PER_PAGE: usize = 20;
/// Maximum items per page (supports client-side filtering for up to 10k sequences)
pub const MAX_PER_PAGE: usize = 10000;

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PaginationInfo {
    /// Current page (1-indexed)
    pub page: usize,
    /// Items per page
    pub per_page: usize,
    /// Total number of items
    pub total_items: usize,
    /// Total number of pages
    pub total_pages: usize,
    /// Has next page
    pub has_next: bool,
    /// Has previous page
    pub has_prev: bool,
}

impl PaginationInfo {
    pub fn new(page: usize, per_page: usize, total_items: usize) -> Self {
        let total_pages = total_items.div_ceil(per_page);
        Self {
            page,
            per_page,
            total_items,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

/// Paginated response for job list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PaginatedJobsResponse {
    /// List of jobs (without sequences for performance)
    pub jobs: Vec<JobSummary>,
    /// Pagination information
    pub pagination: PaginationInfo,
}

/// Paginated job details response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PaginatedJobResponse {
    /// Job ID
    pub job_id: String,
    /// Current job status
    pub status: JobStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Uploaded filename
    pub filename: Option<String>,
    /// Total sequence count (unfiltered)
    pub sequence_count: usize,
    /// Processed sequence count
    pub processed_count: usize,
    /// Hash match count
    pub hash_matches: usize,
    /// Alignment match count
    pub alignment_matches: usize,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Paginated sequences (filtered)
    pub sequences: Vec<SequenceInfo>,
    /// Pagination information for sequences
    pub pagination: PaginationInfo,
    /// Current filter applied ("all", "bakta_db", "aidb_db", "hash_match", "alignment", "none")
    pub filter: String,
    /// Number of sequences matching the current filter
    pub filtered_count: usize,
}

/// `GET /api/job/{job_id}` response: the paginated job plus whether the caller
/// owns it. Non-owners (anyone who only has the job ID) get a read-only view;
/// clients use `is_owner` to hide actions that would be rejected.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct JobViewResponse {
    #[serde(flatten)]
    pub job: PaginatedJobResponse,
    /// True if the request carried the owner's cookie or API token
    pub is_owner: bool,
}

/// Query parameters for job list
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListJobsQuery {
    /// Page number (1-indexed, default: 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    /// Items per page (default: 20, max: 100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<usize>,
    /// Only jobs with this status: pending | processing | completed | failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Case-insensitive substring search in the job name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

/// Query parameters for job details
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct GetJobQuery {
    /// Page number for sequences (1-indexed, default: 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    /// Sequences per page (default: 20, max: 10000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<usize>,
    /// Match-status filter: "all" (default), "bakta_db", "aidb_db", "hash_match" (either DB), "alignment", "none"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// Search text (searches in ID, gene, product)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    /// Minimum sequence length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    /// Maximum sequence length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// Filter by COG category (e.g., "J", "K", "L")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cog: Option<String>,
    /// Filter by EC class (1-7)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ec_class: Option<String>,
    /// Filter: only sequences with gene name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_gene: Option<bool>,
    /// Filter: only sequences with product/function
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_product: Option<bool>,
}

/// Sequence filter of `GET /api/job/{job_id}/download/{format}` – the same
/// filters as the sequence table, so a filtered view can be exported as is.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SequenceFilterQuery {
    /// Match-status filter: all (default) | bakta_db | aidb_db | hash_match (either DB) | alignment | none
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// Case-insensitive text search in ID, gene, product
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// COG category code, e.g. "J"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cog: Option<String>,
    /// EC class digit, e.g. "1"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ec_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_gene: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_product: Option<bool>,
}

impl SequenceFilterQuery {
    /// `true` if any filter is set (the server then marks the file name
    /// with `_filtered`).
    pub fn is_active(&self) -> bool {
        self.filter
            .as_deref()
            .is_some_and(|f| !f.is_empty() && f != "all")
            || self.search.as_deref().is_some_and(|s| !s.is_empty())
            || self.min_length.is_some()
            || self.max_length.is_some()
            || self.cog.as_deref().is_some_and(|s| !s.is_empty())
            || self.ec_class.as_deref().is_some_and(|s| !s.is_empty())
            || self.has_gene.is_some()
            || self.has_product.is_some()
    }
}
