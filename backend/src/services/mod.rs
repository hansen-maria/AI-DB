//! ============================================================================
//! Business logic services
//! ============================================================================

pub mod annotation;
pub mod fasta;

pub use annotation::{format_annotation, lookup_hash_in_bakta, process_job_from_file};
pub use fasta::{compute_md5, FastaIterator, BATCH_SIZE, MAX_RESULTS, MAX_SEQUENCE_LENGTH};
