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
            // Build header with annotations - prioritize gene and product
            let mut header_parts = vec![seq.id.clone()];

            // Add gene name if present
            if let Some(ref gene) = seq.gene {
                header_parts.push(format!("gene={}", gene));
            }

            // Add product/function description if present
            if let Some(ref product) = seq.product {
                // Escape special characters in product description
                let clean_product = product.replace('|', "_").replace('\n', " ");
                header_parts.push(format!("product={}", clean_product));
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
