//! ============================================================================
//! Data models for AI-DB Annotations DB ingestion
//!
//! Curation model (see README "Community curation"):
//!   * Every submission is verified server-side (the hash must belong to an
//!     unmatched / unreviewed sequence of the submitting job, fields are
//!     validated) and stored with provenance.
//!   * Existing entries are never silently overwritten.
//!   * An entry is `candidate` until N independent contributors submitted the
//!     same annotation; then it becomes `confirmed`. Competing annotations mark
//!     it `conflicted`. Admins can review (`confirmed` / `rejected`).
//! ============================================================================

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Maximum accepted protein length (aa). Titin is ~35 k aa.
const MAX_LENGTH: usize = 100_000;

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
    // ── Provenance (optional, client-reported; contributor, job and time are
    //    stamped by the server and cannot be forged) ─────────────────────────
    /// Annotation source, currently only "bakta-web"
    #[serde(default)]
    pub source: Option<String>,
    /// Version label of the annotating tool / database, if known
    #[serde(default)]
    pub tool_version: Option<String>,
    /// "bakta" | "baktfold"
    #[serde(default)]
    pub workflow_mode: Option<String>,
    /// Identity of the PSC (UniRef90) hit, as reported by Bakta
    #[serde(default)]
    pub psc_identity: Option<f64>,
    /// E-value of the PSC hit, as reported by Bakta
    #[serde(default)]
    pub psc_evalue: Option<f64>,
}

fn clean(s: &Option<String>) -> Option<String> {
    s.as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Printable text without control characters (tab/newline would corrupt the TSV/GFF3 exports).
fn is_clean_text(s: &str, max_chars: usize) -> bool {
    !s.is_empty() && s.chars().count() <= max_chars && s.chars().all(|c| !c.is_control())
}

/// Reject values spreadsheets would interpret as formulas (CSV/TSV injection).
fn no_formula_prefix(s: &str) -> bool {
    !matches!(s.chars().next(), Some('=' | '+' | '-' | '@'))
}

fn valid_uniref(s: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|p| {
        s.strip_prefix(p).is_some_and(|rest| {
            !rest.is_empty()
                && rest.len() <= 40
                && rest
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        })
    })
}

fn valid_ec(list: &str) -> bool {
    let items: Vec<&str> = list.split(',').map(str::trim).collect();
    if items.is_empty() || items.len() > 20 {
        return false;
    }
    items.iter().all(|ec| {
        let parts: Vec<&str> = ec.split('.').collect();
        parts.len() == 4
            && parts.iter().enumerate().all(|(i, p)| {
                if p.is_empty() || p.len() > 6 {
                    return false;
                }
                // 1.2.3.n4 (preliminary numbers) is allowed in the 4th position
                let digits = if i == 3 { p.strip_prefix('n').unwrap_or(p) } else { p };
                *p == "-" || (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
            })
    })
}

fn valid_go(list: &str) -> bool {
    let items: Vec<&str> = list.split(',').map(str::trim).collect();
    if items.is_empty() || items.len() > 100 {
        return false;
    }
    items.iter().all(|go| {
        go.strip_prefix("GO:")
            .is_some_and(|d| d.len() == 7 && d.chars().all(|c| c.is_ascii_digit()))
    })
}

impl CustomAnnotationEntry {
    /// Normalises (trim, lowercase hash, empty → None) and validates all fields.
    ///
    /// Returns a short, static reason on failure so callers can count / log
    /// rejections without echoing client-controlled text.
    pub fn normalized(&self) -> Result<Self, &'static str> {
        let mut e = self.clone();
        e.md5_hash = e.md5_hash.trim().to_lowercase();
        if e.md5_hash.len() != 32 || !e.md5_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("invalid md5_hash");
        }
        if e.length == 0 || e.length > MAX_LENGTH {
            return Err("invalid length");
        }

        e.uniparc_id = clean(&e.uniparc_id);
        e.ncbi_nrp_id = clean(&e.ncbi_nrp_id);
        e.uniref100_id = clean(&e.uniref100_id);
        e.uniref90_id = clean(&e.uniref90_id);
        e.gene = clean(&e.gene);
        e.product = clean(&e.product);
        e.ec_ids = clean(&e.ec_ids);
        e.go_ids = clean(&e.go_ids);
        e.cog_category = clean(&e.cog_category);
        e.source = clean(&e.source);
        e.tool_version = clean(&e.tool_version);
        e.workflow_mode = clean(&e.workflow_mode);

        if let Some(v) = &e.uniparc_id {
            let ok = v.len() == 13
                && v.starts_with("UPI")
                && v[3..].chars().all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c));
            if !ok {
                return Err("invalid uniparc_id");
            }
        }
        if let Some(v) = &e.ncbi_nrp_id {
            if v.len() > 32 || !v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.')) {
                return Err("invalid ncbi_nrp_id");
            }
        }
        // The client stores the UniRef90 id in both fields (see mapping above)
        if let Some(v) = &e.uniref100_id {
            if !valid_uniref(v, &["UniRef100_", "UniRef90_"]) {
                return Err("invalid uniref100_id");
            }
        }
        if let Some(v) = &e.uniref90_id {
            if !valid_uniref(v, &["UniRef90_"]) {
                return Err("invalid uniref90_id");
            }
        }
        if let Some(v) = &e.gene {
            let ok = v.chars().count() <= 64
                && v.chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'));
            if !ok {
                return Err("invalid gene");
            }
        }
        if let Some(v) = &e.product {
            if !is_clean_text(v, 256) || !no_formula_prefix(v) {
                return Err("invalid product");
            }
        }
        if let Some(v) = &e.ec_ids {
            if !valid_ec(v) {
                return Err("invalid ec_ids");
            }
        }
        if let Some(v) = &e.go_ids {
            if !valid_go(v) {
                return Err("invalid go_ids");
            }
        }
        if let Some(v) = &e.cog_category {
            if v.len() > 3 || !v.chars().all(|c| c.is_ascii_uppercase()) {
                return Err("invalid cog_category");
            }
        }
        if let Some(v) = &e.source {
            if v != "bakta-web" {
                return Err("unsupported source");
            }
        }
        if let Some(v) = &e.workflow_mode {
            if v != "bakta" && v != "baktfold" {
                return Err("invalid workflow_mode");
            }
        }
        if let Some(v) = &e.tool_version {
            if !is_clean_text(v, 64) {
                return Err("invalid tool_version");
            }
        }
        if let Some(v) = e.psc_identity {
            if !v.is_finite() || !(0.0..=100.0).contains(&v) {
                return Err("invalid psc_identity");
            }
        }
        if let Some(v) = e.psc_evalue {
            if !v.is_finite() || v < 0.0 {
                return Err("invalid psc_evalue");
            }
        }
        Ok(e)
    }
}

/// Request body for POST /api/job/{job_id}/bakta/ingest
#[derive(Debug, Deserialize, ToSchema)]
pub struct IngestCustomAnnotationsRequest {
    pub entries: Vec<CustomAnnotationEntry>,
}

/// Response body for POST /api/job/{job_id}/bakta/ingest
#[derive(Debug, Serialize, ToSchema)]
pub struct IngestCustomAnnotationsResponse {
    /// New sequences added as unreviewed `candidate` entries
    pub ingested: usize,
    /// Existing entries that received an additional (or changed) contribution
    /// from this job. Existing annotations are never overwritten by a single
    /// submission.
    pub updated: usize,
    /// Contributions identical to what this contributor already submitted
    pub unchanged: usize,
    /// Entries that became `confirmed` through this request
    pub newly_confirmed: usize,
    /// Entries rejected by server-side verification (hash not part of this
    /// job's unmatched/unreviewed sequences, length mismatch, invalid fields)
    pub rejected: usize,
    /// Total entries received
    pub total: usize,
}
