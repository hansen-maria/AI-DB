//! ============================================================================
//! Sequence-related models
//! ============================================================================

//! Sequence-related data structures

pub use aidb_types::sequence::{SequenceInfo, SOURCE_AIDB_DB, SOURCE_BAKTA_DB};
use utoipa::ToSchema;

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
    /// Release/version of the source database, or ingestion timestamp for AI-DB matches.
    /// See `SequenceInfo::annotation_release` for details.
    pub annotation_release: Option<String>,
    /// Curation status (AI-DB annotations DB matches only)
    pub annotation_status: Option<String>,
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
            annotation_release: None,
            annotation_status: None,
        }
    }
}

/// Filter type for sequences (match status / annotation source)
///
/// Query values: `all` (default), `bakta_db`, `aidb_db`, `hash_match`
/// (either DB), `alignment`, `none` (unmatched). Unknown values fall back to `all`.
#[derive(Default, Debug, Clone, PartialEq, ToSchema)]
pub enum SequenceFilter {
    #[default]
    All,
    /// Hash match in the official Bakta DB
    BaktaDb,
    /// Hash match in the AI-DB annotations DB
    AidbDb,
    /// Hash match in either DB (`bakta_db` or `aidb_db`)
    HashMatch,
    Alignment,
    NoMatch,
}

impl SequenceFilter {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "bakta_db" | "bakta" => SequenceFilter::BaktaDb,
            "aidb_db" | "aidb" => SequenceFilter::AidbDb,
            "hash_match" | "hash" => SequenceFilter::HashMatch,
            "alignment" => SequenceFilter::Alignment,
            "none" | "no_match" => SequenceFilter::NoMatch,
            _ => SequenceFilter::All,
        }
    }

    pub fn matches(&self, seq: &SequenceInfo) -> bool {
        let source = seq.annotation_source.as_deref();
        match self {
            SequenceFilter::All => true,
            SequenceFilter::BaktaDb => source == Some(SOURCE_BAKTA_DB),
            SequenceFilter::AidbDb => source == Some(SOURCE_AIDB_DB),
            // "hash_match" is accepted as legacy source value of older jobs
            SequenceFilter::HashMatch => matches!(
                source,
                Some(SOURCE_BAKTA_DB) | Some(SOURCE_AIDB_DB) | Some("hash_match")
            ),
            SequenceFilter::Alignment => source == Some("alignment"),
            SequenceFilter::NoMatch => source.is_none(),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SequenceFilter::All => "all",
            SequenceFilter::BaktaDb => SOURCE_BAKTA_DB,
            SequenceFilter::AidbDb => SOURCE_AIDB_DB,
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
            let gene_match = seq
                .gene
                .as_ref()
                .map(|g| g.to_lowercase().contains(&search_lower))
                .unwrap_or(false);
            let product_match = seq
                .product
                .as_ref()
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
                    let has_ec_class = seq_ec.split(',').any(|e| e.trim().starts_with(ec));
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
        self.search.is_some()
            || self.min_length.is_some()
            || self.max_length.is_some()
            || self.cog_category.is_some()
            || self.ec_class.is_some()
            || self.has_gene == Some(true)
            || self.has_product == Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq(source: Option<&str>) -> SequenceInfo {
        SequenceInfo {
            id: "s1".into(),
            md5_hash: None,
            length: 10,
            sequence: None,
            annotation: None,
            annotation_source: source.map(str::to_string),
            annotation_release: None,
            annotation_status: None,
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

    #[test]
    fn source_filters_match_their_db() {
        let bakta = seq(Some(SOURCE_BAKTA_DB));
        let aidb = seq(Some(SOURCE_AIDB_DB));
        let none = seq(None);

        let f = SequenceFilter::from_str("bakta_db");
        assert!(f.matches(&bakta) && !f.matches(&aidb) && !f.matches(&none));

        let f = SequenceFilter::from_str("aidb_db");
        assert!(f.matches(&aidb) && !f.matches(&bakta) && !f.matches(&none));

        let f = SequenceFilter::from_str("hash_match");
        assert!(f.matches(&bakta) && f.matches(&aidb) && !f.matches(&none));

        let f = SequenceFilter::from_str("none");
        assert!(f.matches(&none) && !f.matches(&bakta));
    }

    #[test]
    fn as_str_round_trips() {
        for v in [
            "all",
            "bakta_db",
            "aidb_db",
            "hash_match",
            "alignment",
            "none",
        ] {
            assert_eq!(SequenceFilter::from_str(v).as_str(), v);
        }
        assert_eq!(SequenceFilter::from_str("unknown"), SequenceFilter::All);
    }
}
