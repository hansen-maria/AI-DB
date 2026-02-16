//! ============================================================================
//! Sequence-related models
//! ============================================================================

//! Sequence-related data structures

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Sequence information and annotation results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
    /// UniParc ID (if found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniparc_id: Option<String>,
    /// NCBI NRP ID (if found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncbi_nrp_id: Option<String>,
    /// UniRef100 ID (if found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniref100_id: Option<String>,
    /// Product/Function description (from PSC table)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    /// Gene name (from PSC table)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gene: Option<String>,
}

/// Bakta Hash Lookup Result
#[derive(Debug, Clone, ToSchema)]
pub struct HashLookupResult {
    pub found: bool,
    pub db_length: Option<i64>,
    pub uniparc_id: Option<String>,
    pub ncbi_nrp_id: Option<String>,
    pub uniref100_id: Option<String>,
    /// Product/Function description (from PSC table)
    pub product: Option<String>,
    /// Gene name (from PSC table)
    pub gene: Option<String>,
}

impl Default for HashLookupResult {
    fn default() -> Self {
        Self {
            found: false,
            db_length: None,
            uniparc_id: None,
            ncbi_nrp_id: None,
            uniref100_id: None,
            product: None,
            gene: None,
        }
    }
}

/// Filter type for sequences
#[derive(Debug, Clone, PartialEq, ToSchema)]
pub enum SequenceFilter {
    All,
    HashMatch,
    Alignment,
    NoMatch,
}

impl SequenceFilter {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "hash_match" | "hash" => SequenceFilter::HashMatch,
            "alignment" => SequenceFilter::Alignment,
            "none" | "no_match" => SequenceFilter::NoMatch,
            _ => SequenceFilter::All,
        }
    }

    pub fn matches(&self, seq: &SequenceInfo) -> bool {
        match self {
            SequenceFilter::All => true,
            SequenceFilter::HashMatch => seq.annotation_source.as_deref() == Some("hash_match"),
            SequenceFilter::Alignment => seq.annotation_source.as_deref() == Some("alignment"),
            SequenceFilter::NoMatch => seq.annotation_source.is_none(),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SequenceFilter::All => "all",
            SequenceFilter::HashMatch => "hash_match",
            SequenceFilter::Alignment => "alignment",
            SequenceFilter::NoMatch => "none",
        }
    }
}
