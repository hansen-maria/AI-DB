//! ============================================================================
//! Psos analysis result models
//! ============================================================================

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A single Psos analysis result for a sequence
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PsosResult {
    /// The sequence ID this result belongs to
    pub sequence_id: String,

    /// The Psos job ID for viewing in the Psos web interface
    pub psos_job_id: String,

    /// Protein name from homology search (e.g., "Biotin synthase")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_name: Option<String>,

    /// Best hit database reference (e.g., "UniProtKB/Swiss-Prot:B7I4I4")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_hit_dbxref: Option<String>,

    /// Best hit E-value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_hit_evalue: Option<f64>,

    /// Best hit percent identity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_hit_identity: Option<f64>,

    /// Whether a signal peptide was detected
    pub has_signal_peptide: bool,

    /// Number of transmembrane domains (0 = not a membrane protein)
    pub transmembrane_count: usize,
}

/// Request body for saving Psos results
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavePsosResultsRequest {
    /// List of Psos results to save
    pub results: Vec<PsosResult>,
}

/// Response after saving Psos results
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavePsosResultsResponse {
    /// Number of results saved
    pub saved_count: usize,

    /// Total number of Psos results for this job
    pub total_count: usize,
}

/// Response containing all Psos results for a job
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PsosResultsResponse {
    /// The job ID
    pub job_id: String,

    /// List of Psos results
    pub results: Vec<PsosResult>,

    /// Total count
    pub total_count: usize,
}
