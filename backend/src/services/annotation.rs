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
            product: None,
            gene: None,
            cog_category: None,
            ec_ids: None,
            go_ids: None,
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

            // Try to get annotation information via IPS → PSC lookup
            if let Some(ref uniref_id) = result.uniref100_id {
                if let Some(annotation) = lookup_full_annotation(conn, uniref_id) {
                    result.product = annotation.product;
                    result.gene = annotation.gene;
                    result.cog_category = annotation.cog_category;
                    result.ec_ids = annotation.ec_ids;
                    result.go_ids = annotation.go_ids;
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

/// Full annotation data from IPS and PSC tables
#[derive(Default)]
struct FullAnnotation {
    gene: Option<String>,
    product: Option<String>,
    cog_category: Option<String>,
    ec_ids: Option<String>,
    go_ids: Option<String>,
}

/// Lookup full annotation information via IPS → PSC tables
///
/// Database structure:
/// - ups: hash → uniref100_id
/// - ips: uniref100_id → uniref90_id, gene, product, ec_ids, go_ids
/// - psc: uniref90_id → gene, product, cog_category, ec_ids, go_ids
///
/// Strategy: Query IPS for direct data and uniref90_id mapping, then enrich from PSC
fn lookup_full_annotation(conn: &Connection, uniref100_id: &str) -> Option<FullAnnotation> {
    // Step 1: Query IPS table for direct data and uniref90_id mapping
    let ips_query = "SELECT uniref90_id, gene, product, ec_ids, go_ids FROM ips WHERE uniref100_id = ? LIMIT 1";

    match conn.query_row(ips_query, [uniref100_id], |row| {
        Ok((
            row.get::<_, Option<String>>(0).ok().flatten(),  // uniref90_id
            row.get::<_, Option<String>>(1).ok().flatten(),  // gene
            row.get::<_, Option<String>>(2).ok().flatten(),  // product
            row.get::<_, Option<String>>(3).ok().flatten(),  // ec_ids
            row.get::<_, Option<String>>(4).ok().flatten(),  // go_ids
        ))
    }) {
        Ok((uniref90_id, ips_gene, ips_product, ips_ec_ids, ips_go_ids)) => {
            let mut annotation = FullAnnotation {
                gene: ips_gene,
                product: ips_product,
                ec_ids: ips_ec_ids,
                go_ids: ips_go_ids,
                cog_category: None,
            };

            // Step 2: Query PSC using uniref90_id to get additional data
            if let Some(ref uniref90) = uniref90_id {
                let psc_query = "SELECT gene, product, cog_category, ec_ids, go_ids FROM psc WHERE uniref90_id = ? LIMIT 1";

                if let Ok((psc_gene, psc_product, psc_cog, psc_ec, psc_go)) =
                    conn.query_row(psc_query, [uniref90], |row| {
                        Ok((
                            row.get::<_, Option<String>>(0).ok().flatten(),
                            row.get::<_, Option<String>>(1).ok().flatten(),
                            row.get::<_, Option<String>>(2).ok().flatten(),
                            row.get::<_, Option<String>>(3).ok().flatten(),
                            row.get::<_, Option<String>>(4).ok().flatten(),
                        ))
                    })
                {
                    // Merge: prefer IPS data, fall back to PSC
                    if annotation.gene.is_none() {
                        annotation.gene = psc_gene;
                    }
                    if annotation.product.is_none() {
                        annotation.product = psc_product;
                    }
                    // COG category is only in PSC
                    annotation.cog_category = psc_cog;
                    // Merge EC and GO IDs
                    if annotation.ec_ids.is_none() {
                        annotation.ec_ids = psc_ec;
                    }
                    if annotation.go_ids.is_none() {
                        annotation.go_ids = psc_go;
                    }
                }
            }

            // Only return if we found any annotation data
            if annotation.gene.is_some() || annotation.product.is_some() ||
                annotation.cog_category.is_some() || annotation.ec_ids.is_some() ||
                annotation.go_ids.is_some() {
                Some(annotation)
            } else {
                None
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => {
            tracing::debug!("IPS lookup failed: {}", e);
            None
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

/// Quickly count sequences in a FASTA file (counts '>' at start of lines)
fn count_sequences(file_path: &Path, is_gzip: bool) -> usize {
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return 0,
    };

    let reader: Box<dyn BufRead> = if is_gzip {
        Box::new(BufReader::with_capacity(64 * 1024, GzDecoder::new(file)))
    } else {
        Box::new(BufReader::with_capacity(64 * 1024, file))
    };

    let mut count = 0;
    for line in reader.lines() {
        if let Ok(line) = line {
            if line.starts_with('>') {
                count += 1;
            }
        }
    }
    count
}

/// Processes a job from a temporary file (memory-efficient streaming)
pub fn process_job_from_file(state: &AppState, job_id: &str, file_path: &Path, is_gzip: bool) {
    // First, quickly count total sequences for progress tracking
    let total_sequences = count_sequences(file_path, is_gzip);
    tracing::info!("Job {} has {} sequences to process", job_id, total_sequences);

    // Set status to processing with total count
    {
        let job_clone = {
            let mut jobs = state.jobs_mut();
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = JobStatus::Processing;
                job.sequence_count = total_sequences;
                job.processed_count = 0;
                job.updated_at = Utc::now();
                Some(job.clone())
            } else {
                None
            }
        };
        // Persist the status change (outside the lock)
        if let Some(job) = job_clone {
            state.save_job(&job);
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

    // Open file for streaming (second pass for actual processing)
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
                md5_hash: Some(hash_hex),
                length: seq_length,
                sequence: Some(seq),
                annotation,
                annotation_source,
                uniparc_id: lookup_result.uniparc_id,
                ncbi_nrp_id: lookup_result.ncbi_nrp_id,
                uniref100_id: lookup_result.uniref100_id,
                product: lookup_result.product,
                gene: lookup_result.gene,
                cog_category: lookup_result.cog_category,
                ec_ids: lookup_result.ec_ids,
                go_ids: lookup_result.go_ids,
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
                    job.processed_count = processed_count;
                    job.hash_matches = hash_matches;
                    job.updated_at = Utc::now();
                }
            }
            tracing::debug!(
                "Job {} progress: {}/{} sequences processed",
                job_id,
                processed_count,
                total_sequences
            );
        }
    }

    // Shrink to fit to release unused memory
    sequence_infos.shrink_to_fit();

    // Final update with results
    let final_job = {
        let mut jobs = state.jobs_mut();
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = if processed_count > 0 {
                JobStatus::Completed
            } else {
                JobStatus::Failed
            };
            job.updated_at = Utc::now();
            job.sequence_count = processed_count; // Final accurate count
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
            Some(job.clone())
        } else {
            None
        }
    };

    // Persist final results to database
    if let Some(job) = final_job {
        state.save_job(&job);
    }

    tracing::info!(
        "Job {} completed: {} sequences processed, {} hash matches",
        job_id,
        processed_count,
        hash_matches
    );
}
