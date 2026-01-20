//! ============================================================================
//! TSV export format
//! ============================================================================

use crate::models::JobResponse;

/// Generate TSV output with metadata header and data rows
pub fn generate_tsv(job: &JobResponse) -> String {
    let mut output = String::new();

    // Metadata header
    output.push_str("# AI-DB Annotation Results\n");
    output.push_str(&format!("# Job ID: {}\n", job.job_id));
    output.push_str(&format!(
        "# Filename: {}\n",
        job.filename.as_deref().unwrap_or("N/A")
    ));
    output.push_str(&format!("# Created: {}\n", job.created_at));
    output.push_str(&format!("# Total Sequences: {}\n", job.sequence_count));
    output.push_str(&format!("# Hash Matches: {}\n", job.hash_matches));
    output.push_str(&format!("# Alignment Matches: {}\n", job.alignment_matches));
    output.push_str(&format!(
        "# No Matches: {}\n",
        job.sequence_count - job.hash_matches - job.alignment_matches
    ));
    output.push_str("#\n");

    // Column headers
    output.push_str("sequence_id\tlength\tmd5_hash\tannotation_source\tannotation\tuniparc_id\tncbi_nrp_id\tuniref100_id\tuniparc_url\tncbi_url\tuniref100_url\n");

    // Data rows
    if let Some(ref sequences) = job.sequences {
        for seq in sequences {
            let source = seq.annotation_source.as_deref().unwrap_or("none");
            let annotation = seq.annotation.as_deref().unwrap_or("");
            let uniparc = seq.uniparc_id.as_deref().unwrap_or("");
            let ncbi = seq.ncbi_nrp_id.as_deref().unwrap_or("");
            let uniref = seq.uniref100_id.as_deref().unwrap_or("");

            // Generate URLs
            let uniparc_url = seq
                .uniparc_id
                .as_ref()
                .map(|id| format!("https://www.uniprot.org/uniparc/{}", id))
                .unwrap_or_default();
            let ncbi_url = seq
                .ncbi_nrp_id
                .as_ref()
                .map(|id| format!("https://www.ncbi.nlm.nih.gov/protein/{}", id))
                .unwrap_or_default();
            let uniref_url = seq
                .uniref100_id
                .as_ref()
                .map(|id| format!("https://www.uniprot.org/uniref/{}", id))
                .unwrap_or_default();

            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                seq.id,
                seq.length,
                seq.md5_hash,
                source,
                annotation,
                uniparc,
                ncbi,
                uniref,
                uniparc_url,
                ncbi_url,
                uniref_url
            ));
        }
    }

    output
}
