//! ============================================================================
//! Persistent storage for jobs using SQLite
//! ============================================================================

use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

use crate::models::{JobResponse, JobStatus, SequenceInfo};

/// Number of days to retain jobs
const JOB_RETENTION_DAYS: i64 = 30;

/// Initialize the jobs database, creating tables if needed
pub fn init_database(path: &Path) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(path)?;

    // Create jobs table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS jobs (
            job_id TEXT PRIMARY KEY,
            owner_id TEXT,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            filename TEXT,
            sequence_count INTEGER NOT NULL DEFAULT 0,
            processed_count INTEGER NOT NULL DEFAULT 0,
            hash_matches INTEGER NOT NULL DEFAULT 0,
            alignment_matches INTEGER NOT NULL DEFAULT 0,
            error_message TEXT,
            sequences TEXT
        )",
        [],
    )?;

    // Create indexes for common queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_jobs_owner ON jobs(owner_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_jobs_created ON jobs(created_at)",
        [],
    )?;

    tracing::info!("Jobs database initialized at {:?}", path);

    Ok(conn)
}

/// Save a job to the database
pub fn save_job(conn: &Connection, job: &JobResponse) -> Result<(), rusqlite::Error> {
    let status = match job.status {
        JobStatus::Pending => "pending",
        JobStatus::Processing => "processing",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
    };

    // Serialize sequences to JSON
    let sequences_json = job
        .sequences
        .as_ref()
        .map(|seqs| serde_json::to_string(seqs).unwrap_or_default());

    conn.execute(
        "INSERT OR REPLACE INTO jobs 
         (job_id, owner_id, status, created_at, updated_at, filename, 
          sequence_count, processed_count, hash_matches, alignment_matches, 
          error_message, sequences)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            job.job_id,
            job.owner_id,
            status,
            job.created_at.to_rfc3339(),
            job.updated_at.to_rfc3339(),
            job.filename,
            job.sequence_count as i64,
            job.processed_count as i64,
            job.hash_matches as i64,
            job.alignment_matches as i64,
            job.error_message,
            sequences_json,
        ],
    )?;

    Ok(())
}

/// Load a job from the database
pub fn load_job(conn: &Connection, job_id: &str) -> Result<Option<JobResponse>, rusqlite::Error> {
    conn.query_row(
        "SELECT job_id, owner_id, status, created_at, updated_at, filename,
                sequence_count, processed_count, hash_matches, alignment_matches,
                error_message, sequences
         FROM jobs WHERE job_id = ?1",
        [job_id],
        |row| {
            let status_str: String = row.get(2)?;
            let status = match status_str.as_str() {
                "pending" => JobStatus::Pending,
                "processing" => JobStatus::Processing,
                "completed" => JobStatus::Completed,
                "failed" => JobStatus::Failed,
                _ => JobStatus::Failed,
            };

            let created_at_str: String = row.get(3)?;
            let updated_at_str: String = row.get(4)?;

            let sequences_json: Option<String> = row.get(11)?;
            let sequences: Option<Vec<SequenceInfo>> =
                sequences_json.and_then(|json| serde_json::from_str(&json).ok());

            Ok(JobResponse {
                job_id: row.get(0)?,
                owner_id: row.get(1)?,
                status,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                filename: row.get(5)?,
                sequence_count: row.get::<_, i64>(6)? as usize,
                processed_count: row.get::<_, i64>(7)? as usize,
                hash_matches: row.get::<_, i64>(8)? as usize,
                alignment_matches: row.get::<_, i64>(9)? as usize,
                error_message: row.get(10)?,
                sequences,
            })
        },
    )
    .optional()
}

/// Load all jobs for a specific owner
pub fn load_jobs_by_owner(
    conn: &Connection,
    owner_id: &str,
) -> Result<Vec<JobResponse>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT job_id, owner_id, status, created_at, updated_at, filename,
                sequence_count, processed_count, hash_matches, alignment_matches,
                error_message, sequences
         FROM jobs WHERE owner_id = ?1 ORDER BY created_at DESC",
    )?;

    let jobs = stmt
        .query_map([owner_id], |row| {
            let status_str: String = row.get(2)?;
            let status = match status_str.as_str() {
                "pending" => JobStatus::Pending,
                "processing" => JobStatus::Processing,
                "completed" => JobStatus::Completed,
                "failed" => JobStatus::Failed,
                _ => JobStatus::Failed,
            };

            let created_at_str: String = row.get(3)?;
            let updated_at_str: String = row.get(4)?;

            let sequences_json: Option<String> = row.get(11)?;
            let sequences: Option<Vec<SequenceInfo>> =
                sequences_json.and_then(|json| serde_json::from_str(&json).ok());

            Ok(JobResponse {
                job_id: row.get(0)?,
                owner_id: row.get(1)?,
                status,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                filename: row.get(5)?,
                sequence_count: row.get::<_, i64>(6)? as usize,
                processed_count: row.get::<_, i64>(7)? as usize,
                hash_matches: row.get::<_, i64>(8)? as usize,
                alignment_matches: row.get::<_, i64>(9)? as usize,
                error_message: row.get(10)?,
                sequences,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(jobs)
}

/// Delete a job from the database
pub fn delete_job(conn: &Connection, job_id: &str) -> Result<bool, rusqlite::Error> {
    let rows_affected = conn.execute("DELETE FROM jobs WHERE job_id = ?1", [job_id])?;
    Ok(rows_affected > 0)
}

/// Delete jobs older than retention period
pub fn cleanup_old_jobs(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let cutoff = Utc::now() - Duration::days(JOB_RETENTION_DAYS);
    let cutoff_str = cutoff.to_rfc3339();

    let rows_deleted = conn.execute("DELETE FROM jobs WHERE created_at < ?1", [&cutoff_str])?;

    if rows_deleted > 0 {
        tracing::info!(
            "Cleaned up {} jobs older than {} days",
            rows_deleted,
            JOB_RETENTION_DAYS
        );
    }

    Ok(rows_deleted)
}

/// Get count of all jobs
pub fn count_jobs(conn: &Connection) -> Result<usize, rusqlite::Error> {
    conn.query_row("SELECT COUNT(*) FROM jobs", [], |row| {
        row.get::<_, i64>(0).map(|c| c as usize)
    })
}

// ============================================================================
// Psos Results Storage
// ============================================================================

use crate::models::PsosResult;

/// Initialize the psos_results table
pub fn init_psos_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS psos_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id TEXT NOT NULL,
            sequence_id TEXT NOT NULL,
            psos_job_id TEXT NOT NULL,
            protein_name TEXT,
            best_hit_dbxref TEXT,
            best_hit_evalue REAL,
            best_hit_identity REAL,
            has_signal_peptide INTEGER NOT NULL DEFAULT 0,
            transmembrane_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            UNIQUE(job_id, sequence_id)
        )",
        [],
    )?;

    // Create index for faster job lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_psos_job ON psos_results(job_id)",
        [],
    )?;

    tracing::info!("Psos results table initialized");
    Ok(())
}

/// Save a single Psos result
pub fn save_psos_result(
    conn: &Connection,
    job_id: &str,
    result: &PsosResult,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO psos_results
         (job_id, sequence_id, psos_job_id, protein_name, best_hit_dbxref,
          best_hit_evalue, best_hit_identity, has_signal_peptide, transmembrane_count, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            job_id,
            result.sequence_id,
            result.psos_job_id,
            result.protein_name,
            result.best_hit_dbxref,
            result.best_hit_evalue,
            result.best_hit_identity,
            result.has_signal_peptide as i32,
            result.transmembrane_count as i32,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Save multiple Psos results at once
pub fn save_psos_results(
    conn: &Connection,
    job_id: &str,
    results: &[PsosResult],
) -> Result<(), rusqlite::Error> {
    for result in results {
        save_psos_result(conn, job_id, result)?;
    }
    Ok(())
}

/// Load all Psos results for a job
pub fn load_psos_results(
    conn: &Connection,
    job_id: &str,
) -> Result<Vec<PsosResult>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT sequence_id, psos_job_id, protein_name, best_hit_dbxref,
                best_hit_evalue, best_hit_identity, has_signal_peptide, transmembrane_count
         FROM psos_results WHERE job_id = ?1 ORDER BY sequence_id",
    )?;

    let results = stmt
        .query_map([job_id], |row| {
            Ok(PsosResult {
                sequence_id: row.get(0)?,
                psos_job_id: row.get(1)?,
                protein_name: row.get(2)?,
                best_hit_dbxref: row.get(3)?,
                best_hit_evalue: row.get(4)?,
                best_hit_identity: row.get(5)?,
                has_signal_peptide: row.get::<_, i32>(6)? != 0,
                transmembrane_count: row.get::<_, i32>(7)? as usize,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
}

/// Delete all Psos results for a job
pub fn delete_psos_results(conn: &Connection, job_id: &str) -> Result<usize, rusqlite::Error> {
    let rows_deleted = conn.execute("DELETE FROM psos_results WHERE job_id = ?1", [job_id])?;
    Ok(rows_deleted)
}

/// Cleanup Psos results for deleted jobs (orphaned results)
pub fn cleanup_orphaned_psos_results(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let rows_deleted = conn.execute(
        "DELETE FROM psos_results WHERE job_id NOT IN (SELECT job_id FROM jobs)",
        [],
    )?;

    if rows_deleted > 0 {
        tracing::info!("Cleaned up {} orphaned Psos results", rows_deleted);
    }

    Ok(rows_deleted)
}

/// Count Psos results for a job
pub fn count_psos_results(conn: &Connection, job_id: &str) -> Result<usize, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*) FROM psos_results WHERE job_id = ?1",
        [job_id],
        |row| row.get::<_, i64>(0).map(|c| c as usize),
    )
}

// ============================================================================
// Bakta Job State Storage
// ============================================================================

use crate::models::{SaveBaktaJobRequest, StoredBaktaJob};

/// Initialize the bakta_jobs table.
/// One row per AI-DB job (UNIQUE on job_id) – upserted on every progress step.
pub fn init_bakta_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Create table without result_files_json first (for compatibility with existing DBs)
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS bakta_jobs (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id            TEXT    NOT NULL UNIQUE,
            bakta_job_id      TEXT    NOT NULL,
            bakta_secret      TEXT    NOT NULL,
            sequence_type     TEXT    NOT NULL,
            status            TEXT    NOT NULL DEFAULT 'INIT',
            progress_label    TEXT    NOT NULL DEFAULT '',
            progress_percent  INTEGER NOT NULL DEFAULT 0,
            result_files_json TEXT,
            result_json       TEXT,
            created_at        TEXT    NOT NULL,
            updated_at        TEXT    NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_bakta_jobs_job_id  ON bakta_jobs(job_id);
        CREATE INDEX IF NOT EXISTS idx_bakta_jobs_updated ON bakta_jobs(updated_at);",
    )?;

    // Migration: add result_files_json to tables created before this column existed.
    // We intentionally ignore the error here: SQLite returns "duplicate column name"
    // when the column already exists, which is the normal case after the first migration.
    // `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` requires SQLite >= 3.37.0 and is
    // therefore not used here for maximum compatibility.
    if let Err(e) = conn.execute(
        "ALTER TABLE bakta_jobs ADD COLUMN result_files_json TEXT",
        [],
    ) {
        // "duplicate column name" → already migrated, nothing to do
        if !e.to_string().contains("duplicate column name") {
            tracing::warn!("Unexpected error during bakta_jobs migration: {}", e);
        }
    } else {
        tracing::info!("Bakta jobs table: migrated – added result_files_json column");
    }

    tracing::info!("Bakta jobs table initialized");
    Ok(())
}

/// Upsert Bakta job state (INSERT … ON CONFLICT … DO UPDATE).
/// Safe to call on every progress tick.
pub fn upsert_bakta_job(
    conn: &Connection,
    job_id: &str,
    req: &SaveBaktaJobRequest,
) -> Result<(), rusqlite::Error> {
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO bakta_jobs
             (job_id, bakta_job_id, bakta_secret, sequence_type,
              status, progress_label, progress_percent,
              result_files_json, result_json,
              created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
         ON CONFLICT(job_id) DO UPDATE SET
             bakta_job_id      = excluded.bakta_job_id,
             bakta_secret      = excluded.bakta_secret,
             sequence_type     = excluded.sequence_type,
             status            = excluded.status,
             progress_label    = excluded.progress_label,
             progress_percent  = excluded.progress_percent,
             result_files_json = excluded.result_files_json,
             result_json       = excluded.result_json,
             updated_at        = excluded.updated_at",
        params![
            job_id,
            req.bakta_job_id,
            req.bakta_secret,
            req.sequence_type,
            req.status,
            req.progress_label,
            req.progress_percent,
            req.result_files_json,
            req.result_json,
            now,
        ],
    )?;

    Ok(())
}

/// Load persisted Bakta state for an AI-DB job. Returns None when no row exists.
pub fn load_bakta_job(
    conn: &Connection,
    job_id: &str,
) -> Result<Option<StoredBaktaJob>, rusqlite::Error> {
    conn.query_row(
        "SELECT job_id, bakta_job_id, bakta_secret, sequence_type,
                status, progress_label, progress_percent,
                result_files_json, result_json,
                created_at, updated_at
         FROM bakta_jobs WHERE job_id = ?1",
        [job_id],
        |row| {
            Ok(StoredBaktaJob {
                job_id: row.get(0)?,
                bakta_job_id: row.get(1)?,
                bakta_secret: row.get(2)?,
                sequence_type: row.get(3)?,
                status: row.get(4)?,
                progress_label: row.get(5)?,
                progress_percent: row.get(6)?,
                result_files_json: row.get(7)?,
                result_json: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        },
    )
    .optional()
}

/// Delete Bakta state for an AI-DB job. Idempotent.
pub fn delete_bakta_job(conn: &Connection, job_id: &str) -> Result<usize, rusqlite::Error> {
    conn.execute("DELETE FROM bakta_jobs WHERE job_id = ?1", [job_id])
}

/// Delete orphaned Bakta rows whose parent job no longer exists.
pub fn cleanup_orphaned_bakta_jobs(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let rows = conn.execute(
        "DELETE FROM bakta_jobs WHERE job_id NOT IN (SELECT job_id FROM jobs)",
        [],
    )?;
    if rows > 0 {
        tracing::info!("Cleaned up {} orphaned Bakta job states", rows);
    }
    Ok(rows)
}

// ============================================================================
// AI-DB Annotations DB
// Mirrors the Bakta DB schema (ups / ips / psc) so the same lookup code works.
//
// The database file and schema are created by setup-custom-annotations-db.sh.
// This module only reads and writes data – never creates or migrates the DB.
// ============================================================================

use crate::models::CustomAnnotationEntry;

/// Decode a 32-char hex MD5 string into a 16-byte Vec.
fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    if hex.len() != 32 {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// Upsert one entry into the AI-DB annotations DB.
///
/// Populates all three tables of the Bakta DB schema:
///   ups  – hash → IDs                                (always; updates annotation IDs)
///   ips  – UniRef100 ID → gene / product / EC / GO   (when uniref100_id is present)
///   psc  – UniRef90 ID → COG category                (when uniref90_id is present)
///
/// Returns `true` when the hash was new (inserted), `false` when it already existed (updated).
/// All three tables use upsert so repeated Bakta jobs always store the latest annotation.
pub fn ingest_custom_annotation(
    conn: &Connection,
    entry: &CustomAnnotationEntry,
) -> Result<bool, rusqlite::Error> {
    let hash_bytes = match hex_to_bytes(&entry.md5_hash) {
        Some(b) => b,
        None => {
            tracing::warn!(
                "AI-DB annotations DB: invalid MD5 hex '{}' – skipping",
                entry.md5_hash
            );
            return Ok(false);
        }
    };

    // ups – upsert with product column (extended AI-DB annotations DB schema).
    // product is stored directly here for entries without a UniRef ID (hypotheticals).
    // INSERT OR REPLACE deletes+inserts, so we check existence beforehand.
    let existing: Option<i64> = conn
        .query_row(
            "SELECT rowid FROM ups WHERE hash = ?1",
            params![hash_bytes],
            |row| row.get(0),
        )
        .optional()?;

    conn.execute(
        "INSERT OR REPLACE INTO ups (hash, length, uniparc_id, ncbi_nrp_id, uniref100_id, product)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            hash_bytes,
            entry.length as i64,
            entry.uniparc_id,
            entry.ncbi_nrp_id,
            entry.uniref100_id,
            entry.product,
        ],
    )?;

    let is_new = existing.is_none();

    // ips – upsert: overwrite annotation fields with latest values.
    // NULL values in the new entry do NOT overwrite existing data (COALESCE guard)
    // so a re-ingest with less data never degrades an existing annotation.
    if let Some(ref uniref100) = entry.uniref100_id {
        conn.execute(
            "INSERT INTO ips (uniref100_id, uniref90_id, gene, product, ec_ids, go_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(uniref100_id) DO UPDATE SET
                 uniref90_id = COALESCE(?2, excluded.uniref90_id),
                 gene        = COALESCE(?3, excluded.gene),
                 product     = COALESCE(?4, excluded.product),
                 ec_ids      = COALESCE(?5, excluded.ec_ids),
                 go_ids      = COALESCE(?6, excluded.go_ids)",
            params![
                uniref100,
                entry.uniref90_id,
                entry.gene,
                entry.product,
                entry.ec_ids,
                entry.go_ids,
            ],
        )?;
    }

    // psc – upsert: overwrite with latest values, same COALESCE guard.
    if let Some(ref uniref90) = entry.uniref90_id {
        conn.execute(
            "INSERT INTO psc (uniref90_id, gene, product, cog_category, ec_ids, go_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(uniref90_id) DO UPDATE SET
                 gene         = COALESCE(?2, excluded.gene),
                 product      = COALESCE(?3, excluded.product),
                 cog_category = COALESCE(?4, excluded.cog_category),
                 ec_ids       = COALESCE(?5, excluded.ec_ids),
                 go_ids       = COALESCE(?6, excluded.go_ids)",
            params![
                uniref90,
                entry.gene,
                entry.product,
                entry.cog_category,
                entry.ec_ids,
                entry.go_ids,
            ],
        )?;
    }

    Ok(is_new)
}

/// Bulk-upsert a slice of entries into the AI-DB annotations DB.
/// Returns (inserted, updated).
pub fn ingest_custom_annotations(
    conn: &Connection,
    entries: &[CustomAnnotationEntry],
) -> Result<(usize, usize), rusqlite::Error> {
    let mut inserted = 0usize;
    let mut updated = 0usize;
    for entry in entries {
        if ingest_custom_annotation(conn, entry)? {
            inserted += 1;
        } else {
            updated += 1;
        }
    }
    tracing::info!(
        "AI-DB annotations DB: {} new entries inserted, {} existing entries updated",
        inserted,
        updated
    );
    Ok((inserted, updated))
}
