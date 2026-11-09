//! ============================================================================
//! Persistent storage for jobs using SQLite
//! ============================================================================

use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

use crate::auth::owner_digest;
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

/// One-time, idempotent migration: replace raw owner secrets (cookie values /
/// API tokens, recognisable by their hyphens) in `jobs.owner_id` with their
/// digest. Afterwards no credential is stored in jobs.db.
pub fn migrate_owner_ids_to_digest(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT job_id, owner_id FROM jobs WHERE owner_id LIKE '%-%'")?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);
    if rows.is_empty() {
        return Ok(0);
    }
    let tx = conn.unchecked_transaction()?;
    for (job_id, raw) in &rows {
        tx.execute(
            "UPDATE jobs SET owner_id = ?1 WHERE job_id = ?2",
            params![owner_digest(raw), job_id],
        )?;
    }
    tx.commit()?;
    tracing::info!("Migrated {} job owner ids to digests", rows.len());
    Ok(rows.len())
}

/// Same migration for the KPI database (`kpi_monthly_owners.owner_id`).
pub fn migrate_kpi_owner_ids_to_digest(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT month, owner_id FROM kpi_monthly_owners WHERE owner_id LIKE '%-%'")?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);
    if rows.is_empty() {
        return Ok(0);
    }
    let tx = conn.unchecked_transaction()?;
    for (month, raw) in &rows {
        tx.execute(
            "UPDATE kpi_monthly_owners SET owner_id = ?1 WHERE month = ?2 AND owner_id = ?3",
            params![owner_digest(raw), month, raw],
        )?;
    }
    tx.commit()?;
    tracing::info!("Migrated {} KPI owner ids to digests", rows.len());
    Ok(rows.len())
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

    // Migration: add workflow_mode ("bakta" | "baktfold") and workflow_stage
    // (only used for the protein two-step baktfold chain). Existing rows get
    // workflow_mode = 'bakta' via the DEFAULT so old, in-progress Bakta jobs
    // keep behaving exactly as before.
    if let Err(e) = conn.execute(
        "ALTER TABLE bakta_jobs ADD COLUMN workflow_mode TEXT NOT NULL DEFAULT 'bakta'",
        [],
    ) {
        if !e.to_string().contains("duplicate column name") {
            tracing::warn!(
                "Unexpected error during bakta_jobs workflow_mode migration: {}",
                e
            );
        }
    } else {
        tracing::info!("Bakta jobs table: migrated – added workflow_mode column");
    }

    if let Err(e) = conn.execute("ALTER TABLE bakta_jobs ADD COLUMN workflow_stage TEXT", []) {
        if !e.to_string().contains("duplicate column name") {
            tracing::warn!(
                "Unexpected error during bakta_jobs workflow_stage migration: {}",
                e
            );
        }
    } else {
        tracing::info!("Bakta jobs table: migrated – added workflow_stage column");
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
              result_files_json, result_json, workflow_mode, workflow_stage,
              created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
         ON CONFLICT(job_id) DO UPDATE SET
             bakta_job_id      = excluded.bakta_job_id,
             bakta_secret      = excluded.bakta_secret,
             sequence_type     = excluded.sequence_type,
             status            = excluded.status,
             progress_label    = excluded.progress_label,
             progress_percent  = excluded.progress_percent,
             result_files_json = excluded.result_files_json,
             result_json       = excluded.result_json,
             workflow_mode     = excluded.workflow_mode,
             workflow_stage    = excluded.workflow_stage,
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
            req.workflow_mode,
            req.workflow_stage,
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
                result_files_json, result_json, workflow_mode, workflow_stage,
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
                workflow_mode: row.get(9)?,
                workflow_stage: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
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
// The database file and base schema are created by setup-custom-annotations-db.sh.
// The backend only adds the `hash_sightings` table (see `ensure_annotations_schema`).
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

/// Idempotent, additive migration of the AI-DB annotations DB.
///
/// Creates `hash_sightings`, which counts in how many jobs AI-DB has seen a
/// hash (lookup hits in the Bakta DB or the AI-DB annotations DB, and ingests).
/// When the table is created for the first time, every existing AI-DB entry is
/// seeded with the number of distinct jobs that contributed it (from the former
/// `annotation_submissions` table, if present), at least 1.
pub fn ensure_annotations_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    let table_exists = |c: &Connection, name: &str| -> Result<bool, rusqlite::Error> {
        c.query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![name],
            |_| Ok(()),
        )
        .optional()
        .map(|r| r.is_some())
    };

    if table_exists(conn, "hash_sightings")? {
        return Ok(());
    }

    // IMMEDIATE takes the write lock before the existence check, so concurrent
    // first connections (several jobs starting at once) do not both try to
    // create and seed the table; the loser sees it and returns.
    let tx = rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)?;
    if table_exists(&*tx, "hash_sightings")? {
        return Ok(());
    }
    tx.execute_batch(
        "CREATE TABLE hash_sightings (
            hash          BLOB    PRIMARY KEY,
            seen_count    INTEGER NOT NULL DEFAULT 1,
            first_seen_at TEXT    NOT NULL,
            last_seen_at  TEXT    NOT NULL,
            last_job_id   TEXT
        ) WITHOUT ROWID;",
    )?;

    let now = Utc::now().to_rfc3339();
    let seeded = if table_exists(&*tx, "annotation_submissions")? {
        tx.execute(
            "INSERT OR IGNORE INTO hash_sightings (hash, seen_count, first_seen_at, last_seen_at)
             SELECT u.hash,
                    MAX(1, (SELECT COUNT(DISTINCT s.job_id)
                            FROM annotation_submissions s WHERE s.hash = u.hash)),
                    COALESCE(u.created_at, ?1),
                    COALESCE(u.updated_at, u.created_at, ?1)
             FROM ups u",
            params![now],
        )?
    } else {
        tx.execute(
            "INSERT OR IGNORE INTO hash_sightings (hash, seen_count, first_seen_at, last_seen_at)
             SELECT hash, 1, COALESCE(created_at, ?1), COALESCE(updated_at, created_at, ?1)
             FROM ups",
            params![now],
        )?
    };
    tx.commit()?;
    tracing::info!(
        "AI-DB annotations DB: created hash_sightings, seeded {} existing entries",
        seeded
    );
    Ok(())
}

// ── Sightings counter ────────────────────────────────────────────────────────
//
// `seen_count` is incremented at most once per job: `last_job_id` remembers the
// job that last saw the hash, so duplicate sequences within a job, a repeated
// ingest of the same job or a retry do not inflate the counter.

const SIGHTING_SQL: &str =
    "INSERT INTO hash_sightings (hash, seen_count, first_seen_at, last_seen_at, last_job_id)
     VALUES (?1, 1, ?2, ?2, ?3)
     ON CONFLICT(hash) DO UPDATE SET
         seen_count   = seen_count + (last_job_id IS NOT excluded.last_job_id),
         last_seen_at = excluded.last_seen_at,
         last_job_id  = excluded.last_job_id
     RETURNING seen_count";

fn record_sighting(
    conn: &Connection,
    hash_bytes: &[u8],
    job_id: &str,
    now: &str,
) -> Result<i64, rusqlite::Error> {
    conn.prepare_cached(SIGHTING_SQL)?
        .query_row(params![hash_bytes, now, job_id], |row| row.get(0))
}

/// Records that job `job_id` has seen each of `hashes` (16-byte MD5) and
/// returns the updated `seen_count` per hash, in input order. One transaction.
pub fn record_sightings(
    conn: &Connection,
    hashes: &[Vec<u8>],
    job_id: &str,
) -> Result<Vec<i64>, rusqlite::Error> {
    let now = Utc::now().to_rfc3339();
    let tx = conn.unchecked_transaction()?;
    let mut counts = Vec::with_capacity(hashes.len());
    for hash in hashes {
        counts.push(record_sighting(&tx, hash, job_id, &now)?);
    }
    tx.commit()?;
    Ok(counts)
}

// ── Ingest: insert new entries, fill gaps in existing ones ───────────────────

/// Counters returned by [`ingest_custom_annotations`].
#[derive(Debug, Default, Clone)]
pub struct IngestStats {
    /// New hashes added to the AI-DB annotations DB
    pub inserted: usize,
    /// Existing entries where previously empty fields were filled in
    pub updated: usize,
    /// Existing entries that already contained everything submitted
    pub unchanged: usize,
}

/// Ingest one entry. Existing values are never overwritten; only empty (NULL)
/// fields are filled in. `updated_at` changes only when something was added.
fn ingest_one(
    conn: &Connection,
    e: &CustomAnnotationEntry,
    job_id: &str,
    now: &str,
    stats: &mut IngestStats,
) -> Result<(), rusqlite::Error> {
    let Some(hash_bytes) = hex_to_bytes(&e.md5_hash) else {
        tracing::warn!("AI-DB annotations DB: invalid MD5 hex – skipping");
        return Ok(());
    };

    let exists = conn
        .query_row(
            "SELECT 1 FROM ups WHERE hash = ?1",
            params![hash_bytes],
            |_| Ok(()),
        )
        .optional()?
        .is_some();

    // ips / psc rows are shared by all proteins of a UniRef cluster: insert
    // when missing, otherwise only fill empty fields (the WHERE clause makes
    // the upsert a no-op – 0 changes – when there is nothing to fill).
    let mut cluster_changed = false;
    if let Some(ref uniref100) = e.uniref100_id {
        cluster_changed |= conn.execute(
            "INSERT INTO ips (uniref100_id, uniref90_id, gene, product, ec_ids, go_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(uniref100_id) DO UPDATE SET
                 uniref90_id = COALESCE(ips.uniref90_id, excluded.uniref90_id),
                 gene        = COALESCE(ips.gene,        excluded.gene),
                 product     = COALESCE(ips.product,     excluded.product),
                 ec_ids      = COALESCE(ips.ec_ids,      excluded.ec_ids),
                 go_ids      = COALESCE(ips.go_ids,      excluded.go_ids)
             WHERE (ips.uniref90_id IS NULL AND excluded.uniref90_id IS NOT NULL)
                OR (ips.gene        IS NULL AND excluded.gene        IS NOT NULL)
                OR (ips.product     IS NULL AND excluded.product     IS NOT NULL)
                OR (ips.ec_ids      IS NULL AND excluded.ec_ids      IS NOT NULL)
                OR (ips.go_ids      IS NULL AND excluded.go_ids      IS NOT NULL)",
            params![
                uniref100,
                e.uniref90_id,
                e.gene,
                e.product,
                e.ec_ids,
                e.go_ids
            ],
        )? > 0;
    }
    if let Some(ref uniref90) = e.uniref90_id {
        cluster_changed |= conn.execute(
            "INSERT INTO psc (uniref90_id, gene, product, cog_category, ec_ids, go_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(uniref90_id) DO UPDATE SET
                 gene         = COALESCE(psc.gene,         excluded.gene),
                 product      = COALESCE(psc.product,      excluded.product),
                 cog_category = COALESCE(psc.cog_category, excluded.cog_category),
                 ec_ids       = COALESCE(psc.ec_ids,       excluded.ec_ids),
                 go_ids       = COALESCE(psc.go_ids,       excluded.go_ids)
             WHERE (psc.gene         IS NULL AND excluded.gene         IS NOT NULL)
                OR (psc.product      IS NULL AND excluded.product      IS NOT NULL)
                OR (psc.cog_category IS NULL AND excluded.cog_category IS NOT NULL)
                OR (psc.ec_ids       IS NULL AND excluded.ec_ids       IS NOT NULL)
                OR (psc.go_ids       IS NULL AND excluded.go_ids       IS NOT NULL)",
            params![
                uniref90,
                e.gene,
                e.product,
                e.cog_category,
                e.ec_ids,
                e.go_ids
            ],
        )? > 0;
    }

    if !exists {
        conn.execute(
            "INSERT INTO ups
                 (hash, length, uniparc_id, ncbi_nrp_id, uniref100_id, product, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                hash_bytes,
                e.length as i64,
                e.uniparc_id,
                e.ncbi_nrp_id,
                e.uniref100_id,
                e.product,
                now
            ],
        )?;
        stats.inserted += 1;
    } else {
        let ups_changed = conn.execute(
            "UPDATE ups SET
                 uniparc_id   = COALESCE(uniparc_id,   ?2),
                 ncbi_nrp_id  = COALESCE(ncbi_nrp_id,  ?3),
                 uniref100_id = COALESCE(uniref100_id, ?4),
                 product      = COALESCE(product,      ?5),
                 updated_at   = ?6
             WHERE hash = ?1
               AND (   (uniparc_id   IS NULL AND ?2 IS NOT NULL)
                    OR (ncbi_nrp_id  IS NULL AND ?3 IS NOT NULL)
                    OR (uniref100_id IS NULL AND ?4 IS NOT NULL)
                    OR (product      IS NULL AND ?5 IS NOT NULL)
                    OR ?7)",
            params![
                hash_bytes,
                e.uniparc_id,
                e.ncbi_nrp_id,
                e.uniref100_id,
                e.product,
                now,
                cluster_changed
            ],
        )? > 0;
        if ups_changed {
            stats.updated += 1;
        } else {
            stats.unchanged += 1;
        }
    }

    record_sighting(conn, &hash_bytes, job_id, now)?;
    Ok(())
}

/// Ingest verified entries from job `job_id` into the AI-DB annotations DB,
/// in a single transaction.
///
/// * New hashes are inserted.
/// * Existing entries are never overwritten: only empty fields are filled in
///   (ups, and the shared ips / psc cluster rows).
/// * Every entry counts as a sighting of the hash by this job (`hash_sightings`).
pub fn ingest_custom_annotations(
    conn: &Connection,
    entries: &[CustomAnnotationEntry],
    job_id: &str,
) -> Result<IngestStats, rusqlite::Error> {
    let mut stats = IngestStats::default();
    let now = Utc::now().to_rfc3339();
    let tx = conn.unchecked_transaction()?;
    for entry in entries {
        ingest_one(&tx, entry, job_id, &now, &mut stats)?;
    }
    tx.commit()?;
    tracing::info!(
        "AI-DB annotations DB: {} inserted, {} updated (gaps filled), {} unchanged",
        stats.inserted,
        stats.updated,
        stats.unchanged
    );
    Ok(stats)
}

// ============================================================================
// KPI / Analytics Storage
//
// Monthly-bucketed counters, incremented at the moment an event happens
// (job completes, Bakta job starts, Psos results are saved). This is
// deliberately NOT derived from the `jobs` table by aggregation, because
// `jobs` rows are purged after JOB_RETENTION_DAYS (30 days) – aggregating
// after the fact would silently lose all data older than ~1 month.
// ============================================================================

/// Initialize the kpi_monthly and kpi_monthly_owners tables.
pub fn init_kpi_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS kpi_monthly (
            month               TEXT PRIMARY KEY,   -- 'YYYY-MM'
            jobs_created        INTEGER NOT NULL DEFAULT 0,
            jobs_failed         INTEGER NOT NULL DEFAULT 0,
            sequences_processed INTEGER NOT NULL DEFAULT 0,
            hash_matches_bakta  INTEGER NOT NULL DEFAULT 0,
            hash_matches_aidb   INTEGER NOT NULL DEFAULT 0,
            bakta_jobs_started  INTEGER NOT NULL DEFAULT 0,
            psos_analyses       INTEGER NOT NULL DEFAULT 0,
            updated_at          TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS kpi_monthly_owners (
            month    TEXT NOT NULL,
            owner_id TEXT NOT NULL,
            PRIMARY KEY (month, owner_id)
        );",
    )?;
    tracing::info!("KPI tables initialized");
    Ok(())
}

/// Current month bucket key, e.g. "2026-08".
fn current_month() -> String {
    Utc::now().format("%Y-%m").to_string()
}

/// Ensures a row for the given month exists (idempotent no-op if it already does).
fn ensure_month_row(conn: &Connection, month: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR IGNORE INTO kpi_monthly (month, updated_at) VALUES (?1, ?2)",
        params![month, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Records a finalized job (Completed or Failed) in the current month's bucket.
/// Called once per job, when it reaches its final state – not on every progress tick.
pub fn record_job_completion(
    conn: &Connection,
    job: &JobResponse,
    bakta_matches: usize,
    aidb_matches: usize,
) -> Result<(), rusqlite::Error> {
    let month = current_month();
    ensure_month_row(conn, &month)?;

    let failed_increment = if job.status == JobStatus::Failed {
        1
    } else {
        0
    };

    conn.execute(
        "UPDATE kpi_monthly SET
             jobs_created        = jobs_created + 1,
             jobs_failed         = jobs_failed + ?2,
             sequences_processed = sequences_processed + ?3,
             hash_matches_bakta  = hash_matches_bakta + ?4,
             hash_matches_aidb   = hash_matches_aidb + ?5,
             updated_at          = ?6
         WHERE month = ?1",
        params![
            month,
            failed_increment,
            job.processed_count as i64,
            bakta_matches as i64,
            aidb_matches as i64,
            Utc::now().to_rfc3339(),
        ],
    )?;

    if let Some(ref owner_id) = job.owner_id {
        conn.execute(
            "INSERT OR IGNORE INTO kpi_monthly_owners (month, owner_id) VALUES (?1, ?2)",
            params![month, owner_id],
        )?;
    }

    Ok(())
}

/// Records a job *retry* completion (via reannotate_sequences). Unlike
/// `record_job_completion`, this does NOT increment jobs_created/jobs_failed –
/// the job was already counted once when it first reached a final state.
/// Only the resulting sequence/match counts are (re-)recorded.
pub fn record_job_retry_completion(
    conn: &Connection,
    job: &JobResponse,
    bakta_matches: usize,
    aidb_matches: usize,
) -> Result<(), rusqlite::Error> {
    let month = current_month();
    ensure_month_row(conn, &month)?;

    conn.execute(
        "UPDATE kpi_monthly SET
             sequences_processed = sequences_processed + ?2,
             hash_matches_bakta  = hash_matches_bakta + ?3,
             hash_matches_aidb   = hash_matches_aidb + ?4,
             updated_at          = ?5
         WHERE month = ?1",
        params![
            month,
            job.processed_count as i64,
            bakta_matches as i64,
            aidb_matches as i64,
            Utc::now().to_rfc3339(),
        ],
    )?;

    if let Some(ref owner_id) = job.owner_id {
        conn.execute(
            "INSERT OR IGNORE INTO kpi_monthly_owners (month, owner_id) VALUES (?1, ?2)",
            params![month, owner_id],
        )?;
    }

    Ok(())
}

/// Records that a new Bakta job was started (call only on first save of a
/// given AI-DB job's Bakta state, not on every progress-tick upsert).
pub fn record_bakta_job_started(conn: &Connection) -> Result<(), rusqlite::Error> {
    let month = current_month();
    ensure_month_row(conn, &month)?;
    conn.execute(
        "UPDATE kpi_monthly SET bakta_jobs_started = bakta_jobs_started + 1, updated_at = ?2
         WHERE month = ?1",
        params![month, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Records `count` Psos-analyzed sequences in the current month's bucket.
pub fn record_psos_analyses(conn: &Connection, count: usize) -> Result<(), rusqlite::Error> {
    if count == 0 {
        return Ok(());
    }
    let month = current_month();
    ensure_month_row(conn, &month)?;
    conn.execute(
        "UPDATE kpi_monthly SET psos_analyses = psos_analyses + ?2, updated_at = ?3
         WHERE month = ?1",
        params![month, count as i64, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// One row of the monthly KPI overview, combining `jobs.db` counters with the
/// distinct-owner count for that month.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KpiMonthRow {
    pub month: String,
    pub jobs_created: i64,
    pub jobs_failed: i64,
    pub sequences_processed: i64,
    pub hash_matches_bakta: i64,
    pub hash_matches_aidb: i64,
    pub bakta_jobs_started: i64,
    pub psos_analyses: i64,
    pub active_owners: i64,
}

/// Loads all monthly KPI rows (from `jobs.db`), newest month first.
pub fn get_kpi_overview(conn: &Connection) -> Result<Vec<KpiMonthRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT m.month, m.jobs_created, m.jobs_failed, m.sequences_processed,
                m.hash_matches_bakta, m.hash_matches_aidb, m.bakta_jobs_started, m.psos_analyses,
                (SELECT COUNT(*) FROM kpi_monthly_owners o WHERE o.month = m.month) AS active_owners
         FROM kpi_monthly m
         ORDER BY m.month DESC",
    )?;

    let rows = stmt
        .query_map([], |row| {
            Ok(KpiMonthRow {
                month: row.get(0)?,
                jobs_created: row.get(1)?,
                jobs_failed: row.get(2)?,
                sequences_processed: row.get(3)?,
                hash_matches_bakta: row.get(4)?,
                hash_matches_aidb: row.get(5)?,
                bakta_jobs_started: row.get(6)?,
                psos_analyses: row.get(7)?,
                active_owners: row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

/// Monthly growth of the AI-DB annotations DB (new sequences saved per month).
/// Reads `ups.created_at`, which is only populated by the migration added
/// alongside `annotation_release` – rows ingested before that migration have
/// `created_at IS NULL` and are excluded here (they still count towards the
/// all-time total, just not attributable to a specific month).
pub fn get_aidb_growth_by_month(conn: &Connection) -> Result<Vec<(String, i64)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT strftime('%Y-%m', created_at) AS month, COUNT(*) AS new_sequences
         FROM ups
         WHERE created_at IS NOT NULL
         GROUP BY month
         ORDER BY month DESC",
    )?;

    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

#[cfg(test)]
mod annotations_db_tests {
    use super::*;

    /// Minimal Bakta-like schema + the sightings migration, in memory.
    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE ups(hash BLOB PRIMARY KEY, length INTEGER, uniparc_id TEXT,
                 ncbi_nrp_id TEXT, uniref100_id TEXT, product TEXT, created_at TEXT, updated_at TEXT);
             CREATE TABLE ips(uniref100_id TEXT PRIMARY KEY, uniref90_id TEXT, gene TEXT,
                 product TEXT, ec_ids TEXT, go_ids TEXT);
             CREATE TABLE psc(uniref90_id TEXT PRIMARY KEY, gene TEXT, product TEXT,
                 cog_category TEXT, ec_ids TEXT, go_ids TEXT);",
        )
        .unwrap();
        ensure_annotations_schema(&conn).unwrap();
        // idempotent
        ensure_annotations_schema(&conn).unwrap();
        conn
    }

    fn entry(hash: &str, gene: Option<&str>, product: &str) -> CustomAnnotationEntry {
        CustomAnnotationEntry {
            md5_hash: hash.to_string(),
            length: 100,
            uniparc_id: None,
            ncbi_nrp_id: None,
            uniref100_id: Some("UniRef90_A".into()),
            uniref90_id: Some("UniRef90_A".into()),
            gene: gene.map(str::to_string),
            product: Some(product.into()),
            ec_ids: None,
            go_ids: None,
            cog_category: Some("J".into()),
            source: Some("bakta-web".into()),
            tool_version: None,
            workflow_mode: Some("bakta".into()),
            psc_identity: Some(0.99),
            psc_evalue: Some(1e-30),
        }
    }

    const H: &str = "0123456789abcdef0123456789abcdef";

    fn hash() -> Vec<u8> {
        hex_to_bytes(H).unwrap()
    }

    fn ips_of(conn: &Connection) -> (Option<String>, Option<String>) {
        conn.query_row(
            "SELECT gene, product FROM ips WHERE uniref100_id = 'UniRef90_A'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    fn seen_count(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT seen_count FROM hash_sightings WHERE hash = ?1",
            params![hash()],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn new_hash_is_inserted_and_seen_once() {
        let conn = db();
        let s =
            ingest_custom_annotations(&conn, &[entry(H, Some("dnaK"), "chaperone")], "j1").unwrap();
        assert_eq!(s.inserted, 1);
        assert_eq!(seen_count(&conn), 1);
    }

    #[test]
    fn existing_values_are_never_overwritten() {
        let conn = db();
        ingest_custom_annotations(&conn, &[entry(H, Some("dnaK"), "chaperone")], "j1").unwrap();
        let s = ingest_custom_annotations(&conn, &[entry(H, Some("evil"), "junk")], "j2").unwrap();
        assert_eq!(s.unchanged, 1);
        assert_eq!(
            ips_of(&conn),
            (Some("dnaK".to_string()), Some("chaperone".to_string()))
        );
        let ups_product: String = conn
            .query_row(
                "SELECT product FROM ups WHERE hash = ?1",
                params![hash()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ups_product, "chaperone");
    }

    #[test]
    fn empty_fields_are_filled_in() {
        let conn = db();
        ingest_custom_annotations(&conn, &[entry(H, None, "chaperone")], "j1").unwrap();
        let s = ingest_custom_annotations(&conn, &[entry(H, Some("dnaK"), "other")], "j2").unwrap();
        assert_eq!(s.updated, 1);
        assert_eq!(
            ips_of(&conn),
            (Some("dnaK".to_string()), Some("chaperone".to_string()))
        );
    }

    #[test]
    fn counter_increments_once_per_job() {
        let conn = db();
        let e = entry(H, Some("dnaK"), "chaperone");
        ingest_custom_annotations(&conn, &[e.clone()], "j1").unwrap();
        // same job again (e.g. auto-save + manual save): not counted twice
        ingest_custom_annotations(&conn, &[e.clone()], "j1").unwrap();
        assert_eq!(seen_count(&conn), 1);
        // lookup hits of other jobs, duplicates within one job count once
        let counts = record_sightings(&conn, &[hash(), hash()], "j2").unwrap();
        assert_eq!(counts, vec![2, 2]);
        ingest_custom_annotations(&conn, &[e], "j3").unwrap();
        assert_eq!(seen_count(&conn), 3);
    }

    #[test]
    fn existing_entries_are_seeded_on_first_migration() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE ups(hash BLOB PRIMARY KEY, length INTEGER, uniparc_id TEXT,
                 ncbi_nrp_id TEXT, uniref100_id TEXT, product TEXT, created_at TEXT, updated_at TEXT);
             CREATE TABLE annotation_submissions(hash BLOB, job_id TEXT, contributor TEXT);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ups (hash, length) VALUES (?1, 100), (x'00000000000000000000000000000000', 5)",
            params![hash()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO annotation_submissions VALUES (?1, 'j1', 'c1'), (?1, 'j2', 'c2'), (?1, 'j2', 'c3')",
            params![hash()],
        )
        .unwrap();
        ensure_annotations_schema(&conn).unwrap();
        assert_eq!(seen_count(&conn), 2);
        let other: i64 = conn
            .query_row(
                "SELECT seen_count FROM hash_sightings WHERE hash = x'00000000000000000000000000000000'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(other, 1);
    }

    #[test]
    fn entry_validation_rejects_hostile_values() {
        let ok = entry(H, Some("dnaK"), "chaperone");
        assert!(ok.normalized().is_ok());
        for bad in [
            CustomAnnotationEntry {
                product: Some("=HYPERLINK(\"x\")".into()),
                ..ok.clone()
            },
            CustomAnnotationEntry {
                product: Some("a\tb".into()),
                ..ok.clone()
            },
            CustomAnnotationEntry {
                ec_ids: Some("not-an-ec".into()),
                ..ok.clone()
            },
            CustomAnnotationEntry {
                go_ids: Some("GO:12".into()),
                ..ok.clone()
            },
            CustomAnnotationEntry {
                md5_hash: "zz".into(),
                ..ok.clone()
            },
            CustomAnnotationEntry {
                uniref90_id: Some("bad".into()),
                ..ok.clone()
            },
            CustomAnnotationEntry {
                source: Some("manual".into()),
                ..ok.clone()
            },
        ] {
            assert!(bad.normalized().is_err());
        }
    }
}

#[cfg(test)]
mod owner_migration_tests {
    use super::*;

    #[test]
    fn raw_owner_secrets_are_replaced_by_digests_idempotently() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE jobs (job_id TEXT PRIMARY KEY, owner_id TEXT);
             CREATE TABLE kpi_monthly_owners (month TEXT NOT NULL, owner_id TEXT NOT NULL,
                 PRIMARY KEY (month, owner_id));",
        )
        .unwrap();
        let raw = "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c";
        conn.execute(
            "INSERT INTO jobs VALUES ('j1', ?1), ('j2', NULL)",
            params![raw],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO kpi_monthly_owners VALUES ('2026-10', ?1)",
            params![raw],
        )
        .unwrap();

        assert_eq!(migrate_owner_ids_to_digest(&conn).unwrap(), 1);
        assert_eq!(migrate_kpi_owner_ids_to_digest(&conn).unwrap(), 1);
        let job_owner: String = conn
            .query_row("SELECT owner_id FROM jobs WHERE job_id = 'j1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(job_owner, owner_digest(raw));
        let kpi_owner: String = conn
            .query_row("SELECT owner_id FROM kpi_monthly_owners", [], |r| r.get(0))
            .unwrap();
        assert_eq!(kpi_owner, owner_digest(raw));

        // second run: nothing left to migrate
        assert_eq!(migrate_owner_ids_to_digest(&conn).unwrap(), 0);
        assert_eq!(migrate_kpi_owner_ids_to_digest(&conn).unwrap(), 0);
    }
}
