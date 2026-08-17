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

/// Reads the Bakta DB release/version label from `version.json`.
///
/// Bakta stores its release metadata NOT inside the SQLite DB, but as a separate
/// `version.json` file living alongside `bakta.db` in the same database directory
/// (format: `{"date": "2025-01-15", "major": 6, "minor": 0, ...}`). This file is
/// part of every official Bakta DB release/update, so it is always current after
/// a `bakta_db update` – no extra deployment or cron changes are needed.
///
/// Returns a human-readable label such as `"6.0 (2025-01-15)"`, or `None` if the
/// file is missing or malformed.
pub fn get_bakta_db_release(bakta_db_dir: &Path) -> Option<String> {
    let version_path = bakta_db_dir.join("version.json");
    let content = std::fs::read_to_string(&version_path)
        .map_err(|e| {
            tracing::debug!(
                "Could not read Bakta DB version file {:?}: {}",
                version_path,
                e
            );
            e
        })
        .ok()?;

    let value: serde_json::Value = serde_json::from_str(&content).ok()?;

    let major = value.get("major").and_then(|v| v.as_i64());
    let minor = value.get("minor").and_then(|v| v.as_i64());
    let date = value.get("date").and_then(|v| v.as_str());

    match (major, minor, date) {
        (Some(maj), Some(min), Some(d)) => Some(format!("{maj}.{min} ({d})")),
        (Some(maj), Some(min), None) => Some(format!("{maj}.{min}")),
        _ => None,
    }
}

/// Performs hash lookup in a single database connection (Bakta or AI-DB annotations DB).
/// Reads the standard ups schema (no product column).
fn lookup_in_db(conn: &Connection, hash_bytes: &[u8], seq_length: usize) -> HashLookupResult {
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
            annotation_release: None,
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

/// Performs hash lookup in the AI-DB annotations DB.
/// Reads the extended ups schema which includes a direct `product` column.
/// When uniref100_id is set the ips/psc chain is followed as usual.
/// When uniref100_id is null the product column is used directly (hypothetical proteins).
fn lookup_in_aidb(conn: &Connection, hash_bytes: &[u8], seq_length: usize) -> HashLookupResult {
    // `updated_at` reflects when this entry was last (re-)annotated via a Bakta ingest –
    // used as the per-row "release" timestamp shown to the user for AI-DB matches.
    let query = "SELECT length, uniparc_id, ncbi_nrp_id, uniref100_id, product, updated_at \
                 FROM ups WHERE hash = ?";

    match conn.query_row(query, [hash_bytes], |row| {
        Ok(HashLookupResult {
            found: true,
            db_length: row.get(0).ok(),
            uniparc_id: row.get(1).ok(),
            ncbi_nrp_id: row.get(2).ok(),
            uniref100_id: row.get(3).ok(),
            product: row.get(4).ok(),
            gene: None,
            cog_category: None,
            ec_ids: None,
            go_ids: None,
            annotation_release: row.get(5).ok(),
        })
    }) {
        Ok(mut result) => {
            if let Some(db_len) = result.db_length {
                if db_len as usize != seq_length {
                    tracing::debug!(
                        "Hash found but length mismatch: DB={}, Query={}",
                        db_len,
                        seq_length
                    );
                }
            }
            if let Some(ref uniref_id) = result.uniref100_id {
                if let Some(annotation) = lookup_full_annotation(conn, uniref_id) {
                    result.product = annotation.product.or(result.product);
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
            tracing::error!("AI-DB annotations DB query error: {}", e);
            HashLookupResult::default()
        }
    }
}

/// Performs hash lookup in the Bakta database (kept for backward compat).
pub fn lookup_hash_in_bakta(
    conn: &Connection,
    hash_bytes: &[u8],
    seq_length: usize,
) -> HashLookupResult {
    lookup_in_db(conn, hash_bytes, seq_length)
}

/// Performs hash lookup: checks Bakta DB first, then falls back to AI-DB annotations DB.
///
/// Returns `(result, source)` where `source` is:
///   - `"bakta_db"`  – found in the official Bakta database
///   - `"aidb_db"`   – found in the local AI-DB annotations database
///   - `""`          – not found in either database (unmatched)
pub fn lookup_hash(
    bakta_conn: Option<&Connection>,
    aidb_conn: Option<&Connection>,
    hash_bytes: &[u8],
    seq_length: usize,
) -> (HashLookupResult, &'static str) {
    // 1. Try official Bakta DB first (standard schema)
    if let Some(conn) = bakta_conn {
        let result = lookup_in_db(conn, hash_bytes, seq_length);
        if result.found {
            return (result, "bakta_db");
        }
    }
    // 2. Fall back to AI-DB annotations DB (extended schema with ups.product)
    if let Some(conn) = aidb_conn {
        let result = lookup_in_aidb(conn, hash_bytes, seq_length);
        if result.found {
            return (result, "aidb_db");
        }
    }
    (HashLookupResult::default(), "")
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
    let ips_query =
        "SELECT uniref90_id, gene, product, ec_ids, go_ids FROM ips WHERE uniref100_id = ? LIMIT 1";

    match conn.query_row(ips_query, [uniref100_id], |row| {
        Ok((
            row.get::<_, Option<String>>(0).ok().flatten(), // uniref90_id
            row.get::<_, Option<String>>(1).ok().flatten(), // gene
            row.get::<_, Option<String>>(2).ok().flatten(), // product
            row.get::<_, Option<String>>(3).ok().flatten(), // ec_ids
            row.get::<_, Option<String>>(4).ok().flatten(), // go_ids
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
            if annotation.gene.is_some()
                || annotation.product.is_some()
                || annotation.cog_category.is_some()
                || annotation.ec_ids.is_some()
                || annotation.go_ids.is_some()
            {
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

    // When only a direct product is set (e.g. hypothetical protein from AI-DB annotations DB)
    // and no cross-reference IDs are present, return the product directly.
    let has_ids = result.uniref100_id.is_some()
        || result.uniparc_id.is_some()
        || result.ncbi_nrp_id.is_some();

    if !has_ids {
        return result
            .product
            .clone()
            .or_else(|| Some("Known protein (hash match)".to_string()));
    }

    // Build annotation from cross-reference IDs
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

    Some(parts.join(" | "))
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
    for line in reader.lines().flatten() {
        if line.starts_with('>') {
            count += 1;
        }
    }
    count
}

/// Processes a job from a temporary file (memory-efficient streaming)
pub fn process_job_from_file(state: &AppState, job_id: &str, file_path: &Path, is_gzip: bool) {
    // First, quickly count total sequences for progress tracking
    let total_sequences = count_sequences(file_path, is_gzip);
    tracing::info!(
        "Job {} has {} sequences to process",
        job_id,
        total_sequences
    );

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

    // Try to open database connections
    let db_conn = state.open_db_connection();
    let aidb_conn = state.open_custom_annotations_db();
    let _db_available = db_conn.is_some() || aidb_conn.is_some();

    // Read the Bakta DB release/version once for the whole job (cheap, avoids
    // re-reading version.json for every single sequence).
    let bakta_release = state
        .bakta_db_path()
        .and_then(|p| p.parent())
        .and_then(get_bakta_db_release);
    if let Some(ref release) = bakta_release {
        tracing::info!("Job {}: Bakta DB release = {}", job_id, release);
    }

    match (db_conn.is_some(), aidb_conn.is_some()) {
        (true,  true)  => tracing::info!(
            "Job {}: Bakta DB ✓  AI-DB annotations DB ✓  (two-tier lookup active)", job_id
        ),
        (true,  false) => tracing::info!(
            "Job {}: Bakta DB ✓  AI-DB annotations DB –  (custom_annotations.db not found, run setup script)", job_id
        ),
        (false, true)  => tracing::warn!(
            "Job {}: Bakta DB –  AI-DB annotations DB ✓  (Bakta DB unavailable, using AI-DB annotations DB only)", job_id
        ),
        (false, false) => tracing::warn!(
            "Job {}: Bakta DB –  AI-DB annotations DB –  (no database available, all sequences will be unmatched)", job_id
        ),
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
    let mut bakta_db_matches: usize = 0;
    let mut aidb_db_matches: usize = 0;
    let alignment_matches = 0;
    let mut processed_count = 0;
    let mut batch_count = 0;

    // Process sequences one at a time (streaming)
    for (header, seq) in fasta_iter {
        let (hash_hex, hash_bytes) = compute_md5(&seq);
        let seq_length = seq.len();

        // Lookup: Bakta DB first, AI-DB annotations DB as automatic fallback.
        // Sequences not found in either DB are listed as unmatched.
        let (lookup_result, lookup_source) = lookup_hash(
            db_conn.as_ref(),
            aidb_conn.as_ref(),
            &hash_bytes,
            seq_length,
        );

        let (annotation, annotation_source, annotation_release) = match lookup_source {
            "bakta_db" => {
                bakta_db_matches += 1;
                (
                    format_annotation(&lookup_result),
                    Some("bakta_db".to_string()),
                    bakta_release.clone(),
                )
            }
            "aidb_db" => {
                aidb_db_matches += 1;
                (
                    format_annotation(&lookup_result),
                    Some("aidb_db".to_string()),
                    lookup_result.annotation_release.clone(),
                )
            }
            _ => (None, None, None),
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
                annotation_release,
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
            let total_matches = bakta_db_matches + aidb_db_matches;
            {
                let mut jobs = state.jobs_mut();
                if let Some(job) = jobs.get_mut(job_id) {
                    job.processed_count = processed_count;
                    job.hash_matches = total_matches;
                    job.updated_at = Utc::now();
                }
            }
            tracing::debug!(
                "Job {} progress: {}/{} sequences | Bakta DB: {} | AI-DB annotations DB: {} | Unmatched so far: {}",
                job_id,
                processed_count,
                total_sequences,
                bakta_db_matches,
                aidb_db_matches,
                processed_count.saturating_sub(total_matches),
            );
        }
    }

    // Shrink to fit to release unused memory
    sequence_infos.shrink_to_fit();

    let total_matches = bakta_db_matches + aidb_db_matches;
    let unmatched = processed_count.saturating_sub(total_matches);

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
            job.sequence_count = processed_count;
            job.processed_count = processed_count;
            job.hash_matches = total_matches;
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
        state.record_job_kpi(&job, bakta_db_matches, aidb_db_matches);
    }

    tracing::info!(
        "Job {} completed: {} sequences | {} Bakta DB | {} AI-DB annotations DB | {} unmatched ({:.1}% matched)",
        job_id,
        processed_count,
        bakta_db_matches,
        aidb_db_matches,
        unmatched,
        if processed_count > 0 {
            total_matches as f64 / processed_count as f64 * 100.0
        } else {
            0.0
        },
    );

    if aidb_db_matches > 0 {
        tracing::info!(
            "Job {} AI-DB annotations DB contribution: {} sequences ({:.1}% of all matches)",
            job_id,
            aidb_db_matches,
            if total_matches > 0 {
                aidb_db_matches as f64 / total_matches as f64 * 100.0
            } else {
                0.0
            },
        );
    }
}

// ── Retry / Re-annotation ─────────────────────────────────────────────────────

/// Re-runs hash lookups for sequences already stored in a failed job.
///
/// Called by `POST /api/job/{id}/retry`. Because the original temp file has
/// already been deleted, this function works entirely from the in-memory
/// sequence data (stored sequence string or MD5 hex hash).
pub fn reannotate_sequences(state: &AppState, job_id: &str, sequences: Vec<SequenceInfo>) {
    tracing::info!(
        "Job {}: retrying annotation for {} stored sequences",
        job_id,
        sequences.len()
    );

    let db_conn = state.open_db_connection();
    let aidb_conn = state.open_custom_annotations_db();
    let bakta_release = state
        .bakta_db_path()
        .and_then(|p| p.parent())
        .and_then(get_bakta_db_release);

    let total = sequences.len();
    let mut updated = Vec::with_capacity(total);
    let mut bakta_matches = 0usize;
    let mut aidb_matches = 0usize;
    let mut batch_count = 0usize;

    for (i, seq) in sequences.into_iter().enumerate() {
        // Prefer re-hashing the stored sequence; fall back to decoding stored MD5 hex
        let hash_bytes: Option<Vec<u8>> = seq
            .sequence
            .as_deref()
            .map(|s| {
                let (_, b) = compute_md5(s);
                b
            })
            .or_else(|| {
                seq.md5_hash.as_deref().and_then(|hex| {
                    if hex.len() == 32 {
                        (0..32)
                            .step_by(2)
                            .map(|j| u8::from_str_radix(&hex[j..j + 2], 16).ok())
                            .collect()
                    } else {
                        None
                    }
                })
            });

        let updated_seq = if let Some(ref hb) = hash_bytes {
            let (result, source) =
                lookup_hash(db_conn.as_ref(), aidb_conn.as_ref(), hb, seq.length);
            match source {
                "bakta_db" => bakta_matches += 1,
                "aidb_db" => aidb_matches += 1,
                _ => {}
            }
            if result.found {
                let annotation_release = match source {
                    "bakta_db" => bakta_release.clone(),
                    "aidb_db" => result.annotation_release.clone(),
                    _ => None,
                };
                SequenceInfo {
                    annotation: format_annotation(&result),
                    annotation_source: if source.is_empty() {
                        None
                    } else {
                        Some(source.to_string())
                    },
                    annotation_release,
                    uniparc_id: result.uniparc_id,
                    ncbi_nrp_id: result.ncbi_nrp_id,
                    uniref100_id: result.uniref100_id,
                    product: result.product,
                    gene: result.gene,
                    cog_category: result.cog_category,
                    ec_ids: result.ec_ids,
                    go_ids: result.go_ids,
                    ..seq
                }
            } else {
                SequenceInfo {
                    annotation: None,
                    annotation_source: None,
                    annotation_release: None,
                    uniparc_id: None,
                    ncbi_nrp_id: None,
                    uniref100_id: None,
                    product: None,
                    gene: None,
                    cog_category: None,
                    ec_ids: None,
                    go_ids: None,
                    ..seq
                }
            }
        } else {
            seq // no hash available – keep as-is
        };

        updated.push(updated_seq);
        batch_count += 1;

        if batch_count >= BATCH_SIZE {
            batch_count = 0;
            let matches = bakta_matches + aidb_matches;
            let mut jobs = state.jobs_mut();
            if let Some(job) = jobs.get_mut(job_id) {
                job.processed_count = i + 1;
                job.hash_matches = matches;
                job.updated_at = Utc::now();
            }
        }
    }

    let total_matches = bakta_matches + aidb_matches;

    let final_job = {
        let mut jobs = state.jobs_mut();
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Completed;
            job.processed_count = total;
            job.sequence_count = total;
            job.hash_matches = total_matches;
            job.alignment_matches = 0;
            job.updated_at = Utc::now();
            job.error_message = None;
            job.sequences = Some(updated);
            Some(job.clone())
        } else {
            None
        }
    };

    if let Some(job) = final_job {
        state.save_job(&job);
        state.record_job_retry_kpi(&job, bakta_matches, aidb_matches);
    }

    tracing::info!(
        "Job {}: retry complete – {} Bakta DB, {} AI-DB annotations DB ({:.1}% matched)",
        job_id,
        bakta_matches,
        aidb_matches,
        if total > 0 {
            total_matches as f64 / total as f64 * 100.0
        } else {
            0.0
        },
    );
}
