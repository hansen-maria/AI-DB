//! ============================================================================
//! Job-related models
//! ============================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::sequence::SequenceInfo;

/// Status of an annotation job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
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
    /// Owner ID (from cookie, not serialized to client)
    #[serde(skip_serializing)]
    pub owner_id: Option<String>,
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

/// Job summary (without sequences for list view)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobSummary {
    /// Unique job ID
    pub job_id: String,
    /// Current job status
    pub status: JobStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Uploaded filename
    pub filename: Option<String>,
    /// Total sequence count
    pub sequence_count: usize,
    /// Processed sequence count
    pub processed_count: usize,
    /// Hash match count
    pub hash_matches: usize,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

impl From<&JobResponse> for JobSummary {
    fn from(job: &JobResponse) -> Self {
        Self {
            job_id: job.job_id.clone(),
            status: job.status.clone(),
            created_at: job.created_at,
            updated_at: job.updated_at,
            filename: job.filename.clone(),
            sequence_count: job.sequence_count,
            processed_count: job.processed_count,
            hash_matches: job.hash_matches,
            error_message: job.error_message.clone(),
        }
    }
}

// ── New request / response types ─────────────────────────────────────────────

/// Request body for renaming a job
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RenameJobRequest {
    /// New display name for the job
    pub filename: String,
}

/// Request body for bulk-deleting multiple jobs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkDeleteRequest {
    /// IDs of jobs to delete
    pub job_ids: Vec<String>,
}

/// Result of a bulk-delete operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkDeleteResponse {
    /// IDs that were successfully deleted
    pub deleted: Vec<String>,
    /// IDs that were not found
    pub not_found: Vec<String>,
    /// IDs that belonged to a different owner
    pub forbidden: Vec<String>,
}
