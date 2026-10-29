//! ============================================================================
//! Sequence-related wire types
//! ============================================================================

use serde::{Deserialize, Serialize};

/// Sequence information and annotation results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SequenceInfo {
    /// Sequence identifier
    pub id: String,
    /// MD5-Hash (used internally for matching, not displayed to user)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5_hash: Option<String>,
    /// Length in bp / aa
    pub length: usize,
    /// The actual sequence (amino acids or nucleotides)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    /// Annotation description (if found) - legacy field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<String>,
    /// Source of annotation (used for filtering, not displayed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation_source: Option<String>,
    /// Release/version of the source database.
    /// - `bakta_db` matches: version label of the Bakta DB (e.g. "6.0 (2025-01-15)")
    /// - `aidb_db` matches: ISO-8601 timestamp of when the entry was (re-)annotated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation_release: Option<String>,
    /// Curation status of `aidb_db` matches: `confirmed` (reproduced by several
    /// independent contributors or reviewed by an admin), `candidate` (single
    /// contribution, unreviewed), `conflicted` (competing annotations) or
    /// `legacy` (added before curation tracking). Absent for Bakta DB matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotation_status: Option<String>,
    /// UniParc ID (if found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniparc_id: Option<String>,
    /// NCBI NRP ID (if found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncbi_nrp_id: Option<String>,
    /// UniRef100 ID (if found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniref100_id: Option<String>,
    /// Product/Function description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    /// Gene name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gene: Option<String>,
    /// COG category (e.g., "J" for Translation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cog_category: Option<String>,
    /// EC numbers (enzyme classification, comma-separated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ec_ids: Option<String>,
    /// GO terms (Gene Ontology, comma-separated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub go_ids: Option<String>,
}

/// `annotation_source` value of matches from the official Bakta DB
pub const SOURCE_BAKTA_DB: &str = "bakta_db";
/// `annotation_source` value of matches from the AI-DB annotations DB
pub const SOURCE_AIDB_DB: &str = "aidb_db";
