//! ============================================================================
//! Annotated FASTA export format
//! ============================================================================

use crate::models::JobResponse;

/// Line width for FASTA sequence wrapping
const FASTA_LINE_WIDTH: usize = 60;

/// Generate annotated FASTA output
pub fn generate_fasta(job: &JobResponse) -> String {
    let mut output = String::new();

    if let Some(ref sequences) = job.sequences {
        for seq in sequences {
            // Build header with annotations
            let mut header_parts = vec![seq.id.clone()];

            if let Some(ref source) = seq.annotation_source {
                header_parts.push(format!("source={}", source));
            }

            if let Some(ref annotation) = seq.annotation {
                // Escape special characters in annotation
                let clean_annotation = annotation.replace('|', "_").replace('\n', " ");
                header_parts.push(format!("annotation={}", clean_annotation));
            }

            if let Some(ref uniparc) = seq.uniparc_id {
                header_parts.push(format!("UniParc={}", uniparc));
            }

            if let Some(ref uniref) = seq.uniref100_id {
                header_parts.push(format!("UniRef100={}", uniref));
            }

            if let Some(ref ncbi) = seq.ncbi_nrp_id {
                header_parts.push(format!("NCBI_NRP={}", ncbi));
            }

            header_parts.push(format!("length={}", seq.length));
            header_parts.push(format!("md5={}", seq.md5_hash));

            output.push_str(&format!(">{}\n", header_parts.join(" | ")));

            // Write sequence (wrapped at 60 characters)
            if let Some(ref sequence) = seq.sequence {
                for chunk in sequence.as_bytes().chunks(FASTA_LINE_WIDTH) {
                    output.push_str(&String::from_utf8_lossy(chunk));
                    output.push('\n');
                }
            } else {
                output.push_str("# Sequence not available\n");
            }
        }
    }

    output
}
