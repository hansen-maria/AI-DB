//! ============================================================================
//! Data models for AI-DB Annotations DB ingestion
//! ============================================================================

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A single annotation entry to ingest into the AI-DB annotations DB.
///
/// Mapping from Bakta protein JSON to DB schema:
///   feature.aa_hexdigest        → md5_hash  (hash for ups table – no sequence matching needed)
///   feature.length              → length
///   feature.psc.uniref90_id     → uniref100_id AND uniref90_id (used as lookup-chain key)
///   feature.gene / psc.gene     → gene
///   feature.product             → product
///   feature.psc.ec_ids          → ec_ids (comma-separated)
///   feature.psc.go_ids          → go_ids (comma-separated)
///   feature.psc.cog_category    → cog_category  → psc table
///   hypothetical features       → md5_hash + length only, no annotation IDs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CustomAnnotationEntry {
    /// MD5 hash of the protein sequence as hex string (32 chars).
    /// Taken directly from feature.aa_hexdigest in the Bakta JSON.
    pub md5_hash: String,
    /// Protein sequence length in amino acids
    pub length: usize,
    // ups table fields
    pub uniparc_id: Option<String>,
    pub ncbi_nrp_id: Option<String>,
    /// UniRef90 ID stored here to act as the ups→ips lookup key.
    /// (Bakta protein workflow provides UniRef90 as the highest resolution ID.)
    pub uniref100_id: Option<String>,
    // ips table fields
    /// UniRef90 ID – stored separately so the ips→psc lookup chain works.
    pub uniref90_id: Option<String>,
    pub gene: Option<String>,
    pub product: Option<String>,
    pub ec_ids: Option<String>,
    pub go_ids: Option<String>,
    // psc table field
    pub cog_category: Option<String>,
}

/// Request body for POST /api/job/{job_id}/bakta/ingest
#[derive(Debug, Deserialize, ToSchema)]
pub struct IngestCustomAnnotationsRequest {
    pub entries: Vec<CustomAnnotationEntry>,
}

/// Response body for POST /api/job/{job_id}/bakta/ingest
#[derive(Debug, Serialize, ToSchema)]
pub struct IngestCustomAnnotationsResponse {
    /// Number of new sequences inserted into the AI-DB annotations DB
    pub ingested: usize,
    /// Number of sequences that already existed and were updated with new annotation data
    pub updated: usize,
    /// Total entries received
    pub total: usize,
}
