//! ============================================================================
//! Pagination-related models
//! ============================================================================

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::job::{JobStatus, JobSummary};
use super::sequence::SequenceInfo;
use chrono::{DateTime, Utc};

/// Default items per page
pub const DEFAULT_PER_PAGE: usize = 20;
/// Maximum items per page (supports client-side filtering for up to 10k sequences)
pub const MAX_PER_PAGE: usize = 10000;

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedJobsResponse {
    /// List of jobs (without sequences for performance)
    pub jobs: Vec<JobSummary>,
    /// Pagination information
    pub pagination: PaginationInfo,
}

/// Paginated job details response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
    /// Current filter applied ("all", "hash_match", "alignment", "none")
    pub filter: String,
    /// Number of sequences matching the current filter
    pub filtered_count: usize,
}

/// Query parameters for job list
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListJobsQuery {
    /// Page number (1-indexed, default: 1)
    pub page: Option<usize>,
    /// Items per page (default: 20, max: 100)
    pub per_page: Option<usize>,
}

/// Query parameters for job details
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetJobQuery {
    /// Page number for sequences (1-indexed, default: 1)
    pub page: Option<usize>,
    /// Sequences per page (default: 20, max: 100)
    pub per_page: Option<usize>,
    /// Filter by annotation source: "all", "hash_match", "alignment", "none" (default: "all")
    pub filter: Option<String>,
    /// Search text (searches in ID, gene, product)
    pub search: Option<String>,
    /// Minimum sequence length
    pub min_length: Option<usize>,
    /// Maximum sequence length
    pub max_length: Option<usize>,
    /// Filter by COG category (e.g., "J", "K", "L")
    pub cog: Option<String>,
    /// Filter by EC class (1-7)
    pub ec_class: Option<String>,
    /// Filter: only sequences with gene name
    pub has_gene: Option<bool>,
    /// Filter: only sequences with product/function
    pub has_product: Option<bool>,
}
