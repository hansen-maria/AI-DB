//! ============================================================================
//! JSON export format
//! ============================================================================

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::models::JobResponse;

/// JSON export structure with full metadata
#[derive(Serialize)]
pub struct JsonExport {
    pub job_id: String,
    pub filename: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub statistics: JsonExportStats,
    pub sequences: Vec<JsonExportSequence>,
}

#[derive(Serialize)]
pub struct JsonExportStats {
    pub total_sequences: usize,
    pub hash_matches: usize,
    pub alignment_matches: usize,
    pub no_matches: usize,
}

#[derive(Serialize)]
pub struct JsonExportSequence {
    pub id: String,
    pub length: usize,
    /// Source database that provided the annotation: "bakta_db", "aidb_db", or null
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gene: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    /// COG functional category code(s), e.g. "J" or "KL"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cog_category: Option<String>,
    /// Enzyme Commission numbers (parsed from comma-separated string)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ec_ids: Vec<String>,
    /// Gene Ontology term IDs (parsed from comma-separated string)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub go_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    pub database_ids: JsonExportDbIds,
    pub database_urls: JsonExportDbUrls,
}

#[derive(Serialize)]
pub struct JsonExportDbIds {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniparc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncbi_nrp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniref100: Option<String>,
}

#[derive(Serialize)]
pub struct JsonExportDbUrls {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniparc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncbi_nrp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniref100: Option<String>,
}

/// Generate JSON output with full metadata
pub fn generate_json(job: &JobResponse) -> String {
    let sequences: Vec<JsonExportSequence> = job
        .sequences
        .as_ref()
        .map(|seqs| {
            seqs.iter()
                .map(|seq| JsonExportSequence {
                    id: seq.id.clone(),
                    length: seq.length,
                    annotation_source: seq.annotation_source.clone(),
                    gene: seq.gene.clone(),
                    product: seq.product.clone(),
                    cog_category: seq.cog_category.clone(),
                    ec_ids: parse_comma_list(seq.ec_ids.as_deref()),
                    go_ids: parse_comma_list(seq.go_ids.as_deref()),
                    sequence: seq.sequence.clone(),
                    database_ids: JsonExportDbIds {
                        uniparc: seq.uniparc_id.clone(),
                        ncbi_nrp: seq.ncbi_nrp_id.clone(),
                        uniref100: seq.uniref100_id.clone(),
                    },
                    database_urls: JsonExportDbUrls {
                        uniparc: seq
                            .uniparc_id
                            .as_ref()
                            .map(|id| format!("https://www.uniprot.org/uniparc/{}", id)),
                        ncbi_nrp: seq
                            .ncbi_nrp_id
                            .as_ref()
                            .map(|id| format!("https://www.ncbi.nlm.nih.gov/protein/{}", id)),
                        uniref100: seq
                            .uniref100_id
                            .as_ref()
                            .map(|id| format!("https://www.uniprot.org/uniref/{}", id)),
                    },
                })
                .collect()
        })
        .unwrap_or_default();

    let export = JsonExport {
        job_id: job.job_id.clone(),
        filename: job.filename.clone(),
        created_at: job.created_at,
        completed_at: job.updated_at,
        statistics: JsonExportStats {
            total_sequences: job.sequence_count,
            hash_matches: job.hash_matches,
            alignment_matches: job.alignment_matches,
            no_matches: job
                .sequence_count
                .saturating_sub(job.hash_matches + job.alignment_matches),
        },
        sequences,
    };

    serde_json::to_string_pretty(&export).unwrap_or_default()
}

/// Parse a nullable comma-separated string into a `Vec<String>`,
/// trimming whitespace and skipping empty entries.
fn parse_comma_list(s: Option<&str>) -> Vec<String> {
    match s {
        Some(raw) => raw
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect(),
        None => Vec::new(),
    }
}
