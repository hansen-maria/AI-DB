//! ============================================================================
//! Export formats for job results
//! ============================================================================

//! Download format definitions

/// Supported download formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DownloadFormat {
    Tsv,
    Json,
    Fasta,
    Gff3,
}

impl DownloadFormat {
    /// Parse format from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tsv" => Some(DownloadFormat::Tsv),
            "json" => Some(DownloadFormat::Json),
            "fasta" => Some(DownloadFormat::Fasta),
            "gff3" => Some(DownloadFormat::Gff3),
            _ => None,
        }
    }

    /// Returns the MIME content type
    pub fn content_type(&self) -> &'static str {
        match self {
            DownloadFormat::Tsv => "text/tab-separated-values",
            DownloadFormat::Json => "application/json",
            DownloadFormat::Fasta => "text/x-fasta",
            DownloadFormat::Gff3 => "text/x-gff3",
        }
    }

    /// Returns the file extension
    pub fn file_extension(&self) -> &'static str {
        match self {
            DownloadFormat::Tsv => "tsv",
            DownloadFormat::Json => "json",
            DownloadFormat::Fasta => "fasta",
            DownloadFormat::Gff3 => "gff3",
        }
    }
}
