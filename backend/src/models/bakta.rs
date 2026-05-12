//! ============================================================================
//! Data models for Bakta job persistence
//! ============================================================================

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Persisted Bakta job state – one row per AI-DB job.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StoredBaktaJob {
    pub job_id: String,
    pub bakta_job_id: String,
    pub bakta_secret: String,
    /// "nucleotide" | "protein"
    pub sequence_type: String,
    /// "INIT" | "RUNNING" | "SUCCESSFUL" | "ERROR"
    pub status: String,
    pub progress_label: String,
    pub progress_percent: i64,
    /// Serialised BaktaAnnotationSummary JSON – present only when status = SUCCESSFUL
    pub result_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Request body for POST /api/job/{job_id}/bakta
#[derive(Debug, Deserialize, ToSchema)]
pub struct SaveBaktaJobRequest {
    pub bakta_job_id: String,
    pub bakta_secret: String,
    pub sequence_type: String,
    pub status: String,
    pub progress_label: String,
    pub progress_percent: i64,
    pub result_json: Option<String>,
}

/// Response body for POST /api/job/{job_id}/bakta
#[derive(Debug, Serialize, ToSchema)]
pub struct SaveBaktaJobResponse {
    pub saved: bool,
}

/// Response body for GET /api/job/{job_id}/bakta
#[derive(Debug, Serialize, ToSchema)]
pub struct BaktaJobStateResponse {
    pub state: StoredBaktaJob,
}
