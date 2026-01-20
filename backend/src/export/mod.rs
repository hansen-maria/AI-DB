//! ============================================================================
//! Export formats for job results
//! ============================================================================

pub mod fasta;
pub mod format;
pub mod gff3;
pub mod json;
pub mod tsv;

// Re-export commonly used items
pub use fasta::generate_fasta;
pub use format::DownloadFormat;
pub use gff3::generate_gff3;
pub use json::generate_json;
pub use tsv::generate_tsv;

use crate::models::JobResponse;

/// Generate output content based on format
pub fn generate_content(job: &JobResponse, format: DownloadFormat) -> String {
    match format {
        DownloadFormat::Tsv => generate_tsv(job),
        DownloadFormat::Json => generate_json(job),
        DownloadFormat::Fasta => generate_fasta(job),
        DownloadFormat::Gff3 => generate_gff3(job),
    }
}
