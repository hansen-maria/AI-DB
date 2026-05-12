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
pub fn save_psos_result(conn: &Connection, job_id: &str, result: &PsosResult) -> Result<(), rusqlite::Error> {
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
pub fn save_psos_results(conn: &Connection, job_id: &str, results: &[PsosResult]) -> Result<(), rusqlite::Error> {
    for result in results {
        save_psos_result(conn, job_id, result)?;
    }
    Ok(())
}

/// Load all Psos results for a job
pub fn load_psos_results(conn: &Connection, job_id: &str) -> Result<Vec<PsosResult>, rusqlite::Error> {
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

use crate::models::{StoredBaktaJob, SaveBaktaJobRequest};

/// Initialize the bakta_jobs table.
/// One row per AI-DB job (UNIQUE on job_id) – upserted on every progress step.
pub fn init_bakta_table(conn: &Connection) -> Result<(), rusqlite::Error> {
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
            result_json       TEXT,
            created_at        TEXT    NOT NULL,
            updated_at        TEXT    NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_bakta_jobs_job_id  ON bakta_jobs(job_id);
        CREATE INDEX IF NOT EXISTS idx_bakta_jobs_updated ON bakta_jobs(updated_at);",
    )?;

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
              status, progress_label, progress_percent, result_json,
              created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
         ON CONFLICT(job_id) DO UPDATE SET
             bakta_job_id     = excluded.bakta_job_id,
             bakta_secret     = excluded.bakta_secret,
             sequence_type    = excluded.sequence_type,
             status           = excluded.status,
             progress_label   = excluded.progress_label,
             progress_percent = excluded.progress_percent,
             result_json      = excluded.result_json,
             updated_at       = excluded.updated_at",
        params![
            job_id,
            req.bakta_job_id,
            req.bakta_secret,
            req.sequence_type,
            req.status,
            req.progress_label,
            req.progress_percent,
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
                status, progress_label, progress_percent, result_json,
                created_at, updated_at
         FROM bakta_jobs WHERE job_id = ?1",
        [job_id],
        |row| {
            Ok(StoredBaktaJob {
                job_id:           row.get(0)?,
                bakta_job_id:     row.get(1)?,
                bakta_secret:     row.get(2)?,
                sequence_type:    row.get(3)?,
                status:           row.get(4)?,
                progress_label:   row.get(5)?,
                progress_percent: row.get(6)?,
                result_json:      row.get(7)?,
                created_at:       row.get(8)?,
                updated_at:       row.get(9)?,
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
