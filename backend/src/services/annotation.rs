//! ============================================================================
//! Annotation services - database lookup and job processing
//! ============================================================================

use chrono::Utc;
use flate2::read::GzDecoder;
use rusqlite::Connection;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::models::{HashLookupResult, JobStatus, SequenceInfo};
use crate::services::fasta::{compute_md5, FastaIterator, BATCH_SIZE, MAX_RESULTS};
use crate::state::AppState;

/// Performs hash lookup in the Bakta database
pub fn lookup_hash_in_bakta(
    conn: &Connection,
    hash_bytes: &[u8],
    seq_length: usize,
) -> HashLookupResult {
    // Query the ups table - hash is stored as BLOB
    let query = "SELECT length, uniparc_id, ncbi_nrp_id, uniref100_id FROM ups WHERE hash = ?";

    match conn.query_row(query, [hash_bytes], |row| {
        Ok(HashLookupResult {
            found: true,
            db_length: row.get(0).ok(),
            uniparc_id: row.get(1).ok(),
            ncbi_nrp_id: row.get(2).ok(),
            uniref100_id: row.get(3).ok(),
        })
    }) {
        Ok(mut result) => {
            // Verify length matches (optional sanity check)
            if let Some(db_len) = result.db_length {
                if db_len as usize != seq_length {
                    tracing::debug!(
                        "Hash found but length mismatch: DB={}, Query={}",
                        db_len,
                        seq_length
                    );
                }
            }
            result
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => HashLookupResult::default(),
        Err(e) => {
            tracing::error!("Database query error: {}", e);
            HashLookupResult::default()
        }
    }
}

/// Creates annotation string from lookup result
pub fn format_annotation(result: &HashLookupResult) -> Option<String> {
    if !result.found {
        return None;
    }

    // Build annotation from available IDs
    let mut parts = Vec::new();

    if let Some(ref id) = result.uniref100_id {
        parts.push(format!("UniRef100:{}", id));
    }
    if let Some(ref id) = result.uniparc_id {
        parts.push(format!("UniParc:{}", id));
    }
    if let Some(ref id) = result.ncbi_nrp_id {
        parts.push(format!("NCBI:{}", id));
    }

    if parts.is_empty() {
        Some("Known protein (hash match)".to_string())
    } else {
        Some(parts.join(" | "))
    }
}

/// Processes a job from a temporary file (memory-efficient streaming)
pub fn process_job_from_file(state: &AppState, job_id: &str, file_path: &Path, is_gzip: bool) {
    // Set status to processing
    {
        let mut jobs = state.jobs_mut();
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Processing;
            job.updated_at = Utc::now();
        }
    }

    // Try to open database connection
    let db_conn = state.open_db_connection();
    let db_available = db_conn.is_some();

    if db_available {
        tracing::info!("Processing job {} with Bakta database lookup", job_id);
    } else {
        tracing::warn!("Processing job {} without database", job_id);
    }

    // Open file for streaming
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to open temp file for job {}: {}", job_id, e);
            let mut jobs = state.jobs_mut();
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = JobStatus::Failed;
                job.error_message = Some(format!("Failed to read uploaded file: {}", e));
                job.updated_at = Utc::now();
            }
            return;
        }
    };

    // Create streaming reader (with gzip support)
    let reader: Box<dyn BufRead + Send> = if is_gzip {
        Box::new(BufReader::with_capacity(64 * 1024, GzDecoder::new(file)))
    } else {
        Box::new(BufReader::with_capacity(64 * 1024, file))
    };

    let fasta_iter = FastaIterator::new(reader);

    // Process without pre-allocation (we don't know the count)
    let mut sequence_infos = Vec::new();
    let mut hash_matches = 0;
    let alignment_matches = 0;
    let mut processed_count = 0;
    let mut batch_count = 0;

    // Process sequences one at a time (streaming)
    for (header, seq) in fasta_iter {
        let (hash_hex, hash_bytes) = compute_md5(&seq);
        let seq_length = seq.len();

        // Perform database lookup if available
        let lookup_result = if let Some(ref conn) = db_conn {
            lookup_hash_in_bakta(conn, &hash_bytes, seq_length)
        } else {
            HashLookupResult::default()
        };

        let (annotation, annotation_source) = if lookup_result.found {
            hash_matches += 1;
            (
                format_annotation(&lookup_result),
                Some("hash_match".to_string()),
            )
        } else {
            (None, None)
        };

        // Only store results if we haven't hit the limit
        if sequence_infos.len() < MAX_RESULTS {
            sequence_infos.push(SequenceInfo {
                id: header,
                md5_hash: hash_hex,
                length: seq_length,
                sequence: Some(seq),
                annotation,
                annotation_source,
                uniparc_id: lookup_result.uniparc_id,
                ncbi_nrp_id: lookup_result.ncbi_nrp_id,
                uniref100_id: lookup_result.uniref100_id,
            });
        }

        processed_count += 1;
        batch_count += 1;

        // Update progress every BATCH_SIZE sequences
        if batch_count >= BATCH_SIZE {
            batch_count = 0;
            {
                let mut jobs = state.jobs_mut();
                if let Some(job) = jobs.get_mut(job_id) {
                    job.sequence_count = processed_count;
                    job.processed_count = processed_count;
                    job.hash_matches = hash_matches;
                    job.updated_at = Utc::now();
                }
            }
            tracing::debug!(
                "Job {} progress: {} sequences processed",
                job_id,
                processed_count
            );
        }
    }

    // Shrink to fit to release unused memory
    sequence_infos.shrink_to_fit();

    // Final update with results
    {
        let mut jobs = state.jobs_mut();
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = if processed_count > 0 {
                JobStatus::Completed
            } else {
                JobStatus::Failed
            };
            job.updated_at = Utc::now();
            job.sequence_count = processed_count;
            job.processed_count = processed_count;
            job.hash_matches = hash_matches;
            job.alignment_matches = alignment_matches;

            // Add warning if results were truncated
            if processed_count > MAX_RESULTS {
                job.error_message = Some(format!(
                    "Results truncated: showing first {} of {} sequences",
                    MAX_RESULTS, processed_count
                ));
            } else if processed_count == 0 {
                job.error_message = Some("No valid sequences found in input.".to_string());
            }

            job.sequences = Some(sequence_infos);
        }
    }

    tracing::info!(
        "Job {} completed: {} sequences processed, {} hash matches",
        job_id,
        processed_count,
        hash_matches
    );
}
