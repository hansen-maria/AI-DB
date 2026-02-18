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

/// Bakta Hash Lookup Result
#[derive(Debug, Clone, ToSchema)]
pub struct HashLookupResult {
    pub found: bool,
    pub db_length: Option<i64>,
    pub uniparc_id: Option<String>,
    pub ncbi_nrp_id: Option<String>,
    pub uniref100_id: Option<String>,
    /// Product/Function description
    pub product: Option<String>,
    /// Gene name
    pub gene: Option<String>,
    /// COG category
    pub cog_category: Option<String>,
    /// EC numbers (comma-separated)
    pub ec_ids: Option<String>,
    /// GO terms (comma-separated)
    pub go_ids: Option<String>,
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
            cog_category: None,
            ec_ids: None,
            go_ids: None,
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

/// Advanced filter options for sequence search
#[derive(Debug, Clone, Default)]
pub struct AdvancedSequenceFilter {
    /// Basic filter (match status)
    pub basic: SequenceFilter,
    /// Search text (case-insensitive, searches ID, gene, product)
    pub search: Option<String>,
    /// Minimum sequence length
    pub min_length: Option<usize>,
    /// Maximum sequence length
    pub max_length: Option<usize>,
    /// COG category filter (e.g., "J", "K")
    pub cog_category: Option<String>,
    /// EC class filter (e.g., "1", "2")
    pub ec_class: Option<String>,
    /// Only sequences with gene name
    pub has_gene: Option<bool>,
    /// Only sequences with product description
    pub has_product: Option<bool>,
}

impl AdvancedSequenceFilter {
    /// Check if a sequence matches all filter criteria
    pub fn matches(&self, seq: &SequenceInfo) -> bool {
        // Basic filter (match status)
        if !self.basic.matches(seq) {
            return false;
        }

        // Text search (case-insensitive)
        if let Some(ref search) = self.search {
            let search_lower = search.to_lowercase();
            let id_match = seq.id.to_lowercase().contains(&search_lower);
            let gene_match = seq.gene.as_ref()
                .map(|g| g.to_lowercase().contains(&search_lower))
                .unwrap_or(false);
            let product_match = seq.product.as_ref()
                .map(|p| p.to_lowercase().contains(&search_lower))
                .unwrap_or(false);

            if !id_match && !gene_match && !product_match {
                return false;
            }
        }

        // Length filters
        if let Some(min) = self.min_length {
            if seq.length < min {
                return false;
            }
        }
        if let Some(max) = self.max_length {
            if seq.length > max {
                return false;
            }
        }

        // COG category filter
        if let Some(ref cog) = self.cog_category {
            match &seq.cog_category {
                Some(seq_cog) => {
                    if !seq_cog.contains(cog) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        // EC class filter (matches first digit)
        if let Some(ref ec) = self.ec_class {
            match &seq.ec_ids {
                Some(seq_ec) => {
                    let has_ec_class = seq_ec.split(',')
                        .any(|e| e.trim().starts_with(ec));
                    if !has_ec_class {
                        return false;
                    }
                }
                None => return false,
            }
        }

        // Has gene filter
        if let Some(true) = self.has_gene {
            if seq.gene.as_ref().map(|g| g.is_empty()).unwrap_or(true) {
                return false;
            }
        }

        // Has product filter
        if let Some(true) = self.has_product {
            if seq.product.as_ref().map(|p| p.is_empty()).unwrap_or(true) {
                return false;
            }
        }

        true
    }

    /// Check if any advanced filters are active
    pub fn has_advanced_filters(&self) -> bool {
        self.search.is_some() ||
            self.min_length.is_some() ||
            self.max_length.is_some() ||
            self.cog_category.is_some() ||
            self.ec_class.is_some() ||
            self.has_gene == Some(true) ||
            self.has_product == Some(true)
    }
}

impl Default for SequenceFilter {
    fn default() -> Self {
        SequenceFilter::All
    }
}
