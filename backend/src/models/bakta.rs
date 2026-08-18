//! ============================================================================
//! Data models for Bakta job persistence
//! ============================================================================

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

fn default_workflow_mode() -> String {
    "bakta".to_string()
}

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
    /// JSON of BaktaResultFiles | BaktaProteinsResultFiles | BaktfoldResultFiles (all S3 URLs).
    /// Refreshed on every reload of a completed job so presigned URLs stay valid.
    pub result_files_json: Option<String>,
    /// Full BaktaAnnotationSummary JSON (stats + features + file URLs).
    /// Set once on first SUCCESSFUL completion.
    pub result_json: Option<String>,
    /// "bakta" (default, plain annotation) | "baktfold" (also runs Baktfold –
    /// finds more but takes considerably longer). Defaults to "bakta" for rows
    /// persisted before this field existed.
    #[serde(default = "default_workflow_mode")]
    pub workflow_mode: String,
    /// Only meaningful when workflow_mode == "baktfold" and sequence_type ==
    /// "protein": Baktfold has no combined protein workflow on the Bakta API,
    /// so it runs as two chained jobs. "bakta_proteins" | "baktfold" tracks
    /// which leg is currently in flight so a resumed session continues into
    /// stage 2 automatically once stage 1 finishes. Unused (None) for the
    /// nucleotide combined workflow, which is a single job.
    #[serde(default)]
    pub workflow_stage: Option<String>,
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
    /// Fresh S3 result file URLs – sent whenever new URLs are obtained from Bakta.
    pub result_files_json: Option<String>,
    /// Full summary JSON – sent once on first successful completion.
    pub result_json: Option<String>,
    /// See `StoredBaktaJob::workflow_mode`. Defaults to "bakta" so older
    /// frontend builds that don't send this field keep working unchanged.
    #[serde(default = "default_workflow_mode")]
    pub workflow_mode: String,
    /// See `StoredBaktaJob::workflow_stage`.
    #[serde(default)]
    pub workflow_stage: Option<String>,
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
