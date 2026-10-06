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
    let mut stmt = conn
        .prepare("SELECT month, owner_id FROM kpi_monthly_owners WHERE owner_id LIKE '%-%'")?;
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
            tracing::warn!("Unexpected error during bakta_jobs workflow_mode migration: {}", e);
        }
    } else {
        tracing::info!("Bakta jobs table: migrated – added workflow_mode column");
    }

    if let Err(e) = conn.execute("ALTER TABLE bakta_jobs ADD COLUMN workflow_stage TEXT", []) {
        if !e.to_string().contains("duplicate column name") {
            tracing::warn!("Unexpected error during bakta_jobs workflow_stage migration: {}", e);
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

// ── Curation model ───────────────────────────────────────────────────────────
//
// ups.status:
//   legacy      – added before curation tracking (unreviewed)
//   candidate   – one contribution (or fewer than the threshold)
//   confirmed   – the same annotation was contributed by >= N independent
//                 contributors (AI_DB_CONFIRMATIONS, default 2) or an admin confirmed it
//   conflicted  – competing annotations, none (or several) reached the threshold
//   rejected    – an admin rejected it; hidden from lookups
//
// Contributions live in `annotation_submissions` (one vote per contributor and
// hash; a changed vote replaces the earlier one). The annotation stored in
// ups/ips/psc is the first one submitted, unless a different annotation
// reaches the threshold on its own, in which case consensus replaces it.

pub const STATUS_LEGACY: &str = "legacy";
pub const STATUS_CANDIDATE: &str = "candidate";
pub const STATUS_CONFIRMED: &str = "confirmed";
pub const STATUS_CONFLICTED: &str = "conflicted";
pub const STATUS_REJECTED: &str = "rejected";

/// Independent contributors required to confirm an annotation.
pub fn confirmation_threshold() -> i64 {
    std::env::var("AI_DB_CONFIRMATIONS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(2)
}

/// Idempotent, additive migration of the AI-DB annotations DB: curation
/// columns on `ups` and the `annotation_submissions` table. Existing rows are
/// preserved and marked `legacy`.
pub fn ensure_provenance_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    let alters = [
        "ALTER TABLE ups ADD COLUMN status TEXT NOT NULL DEFAULT 'legacy'",
        "ALTER TABLE ups ADD COLUMN confirmations INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE ups ADD COLUMN annotation_key TEXT",
        "ALTER TABLE ups ADD COLUMN reviewed_at TEXT",
    ];
    for ddl in alters {
        if let Err(e) = conn.execute(ddl, []) {
            // "duplicate column name" → already migrated
            if !e.to_string().contains("duplicate column name") {
                return Err(e);
            }
        }
    }

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS annotation_submissions (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            hash            BLOB    NOT NULL,
            annotation_key  TEXT    NOT NULL,
            contributor     TEXT    NOT NULL,
            job_id          TEXT    NOT NULL,
            submitted_at    TEXT    NOT NULL,
            length          INTEGER NOT NULL,
            uniparc_id      TEXT,
            ncbi_nrp_id     TEXT,
            uniref100_id    TEXT,
            uniref90_id     TEXT,
            gene            TEXT,
            product         TEXT,
            ec_ids          TEXT,
            go_ids          TEXT,
            cog_category    TEXT,
            source          TEXT,
            tool_version    TEXT,
            workflow_mode   TEXT,
            psc_identity    REAL,
            psc_evalue      REAL,
            UNIQUE(hash, contributor)
        );
        CREATE INDEX IF NOT EXISTS idx_submissions_hash
            ON annotation_submissions(hash);
        CREATE INDEX IF NOT EXISTS idx_submissions_contributor
            ON annotation_submissions(contributor, submitted_at);",
    )?;
    Ok(())
}

/// Canonical fingerprint of the annotation content of an entry. Two
/// contributions "agree" iff their keys are equal.
fn annotation_key(e: &CustomAnnotationEntry) -> String {
    fn norm_list(v: &Option<String>) -> String {
        let mut items: Vec<String> = v
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        items.sort();
        items.dedup();
        items.join(",")
    }
    fn norm(v: &Option<String>) -> String {
        v.as_deref().unwrap_or("").trim().to_lowercase()
    }
    let joined = [
        norm(&e.uniref90_id),
        norm(&e.gene),
        norm(&e.product),
        norm_list(&e.ec_ids),
        norm_list(&e.go_ids),
        norm(&e.cog_category),
    ]
    .join("\u{1f}");
    format!("{:x}", md5::compute(joined.as_bytes()))
}

/// Counters returned by [`ingest_custom_annotations`].
#[derive(Debug, Default, Clone)]
pub struct IngestStats {
    /// New hashes added as unreviewed candidates
    pub inserted: usize,
    /// Existing hashes that received an additional or changed contribution
    pub reinforced: usize,
    /// Contributions identical to the contributor's earlier one
    pub unchanged: usize,
    /// Hashes that became `confirmed` through this call
    pub newly_confirmed: usize,
}

/// Writes the annotation of `e` into ups / ips / psc.
///
/// `overwrite == false`: only inserts; existing ips/psc rows keep their values
/// and only missing (NULL) fields are filled in.
/// `overwrite == true`: used solely when consensus replaces the stored annotation.
fn write_annotation(
    conn: &Connection,
    hash_bytes: &[u8],
    e: &CustomAnnotationEntry,
    key: &str,
    status: &str,
    confirmations: i64,
    now: &str,
    overwrite: bool,
) -> Result<(), rusqlite::Error> {
    if overwrite {
        conn.execute(
            "UPDATE ups SET length = ?2, uniparc_id = ?3, ncbi_nrp_id = ?4, uniref100_id = ?5,
                    product = ?6, updated_at = ?7, annotation_key = ?8, status = ?9,
                    confirmations = ?10
             WHERE hash = ?1",
            params![
                hash_bytes,
                e.length as i64,
                e.uniparc_id,
                e.ncbi_nrp_id,
                e.uniref100_id,
                e.product,
                now,
                key,
                status,
                confirmations
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO ups
                 (hash, length, uniparc_id, ncbi_nrp_id, uniref100_id, product,
                  created_at, updated_at, status, confirmations, annotation_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8, ?9, ?10)",
            params![
                hash_bytes,
                e.length as i64,
                e.uniparc_id,
                e.ncbi_nrp_id,
                e.uniref100_id,
                e.product,
                now,
                status,
                confirmations,
                key
            ],
        )?;
    }

    // ips / psc rows are shared by all proteins of a UniRef cluster. They are
    // only filled where missing – except when consensus replaces the annotation.
    let (ips_sql, psc_sql) = if overwrite {
        (
            "INSERT INTO ips (uniref100_id, uniref90_id, gene, product, ec_ids, go_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(uniref100_id) DO UPDATE SET
                 uniref90_id = excluded.uniref90_id, gene = excluded.gene,
                 product = excluded.product, ec_ids = excluded.ec_ids, go_ids = excluded.go_ids",
            "INSERT INTO psc (uniref90_id, gene, product, cog_category, ec_ids, go_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(uniref90_id) DO UPDATE SET
                 gene = excluded.gene, product = excluded.product,
                 cog_category = excluded.cog_category, ec_ids = excluded.ec_ids,
                 go_ids = excluded.go_ids",
        )
    } else {
        (
            "INSERT INTO ips (uniref100_id, uniref90_id, gene, product, ec_ids, go_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(uniref100_id) DO UPDATE SET
                 uniref90_id = COALESCE(ips.uniref90_id, excluded.uniref90_id),
                 gene        = COALESCE(ips.gene,        excluded.gene),
                 product     = COALESCE(ips.product,     excluded.product),
                 ec_ids      = COALESCE(ips.ec_ids,      excluded.ec_ids),
                 go_ids      = COALESCE(ips.go_ids,      excluded.go_ids)",
            "INSERT INTO psc (uniref90_id, gene, product, cog_category, ec_ids, go_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(uniref90_id) DO UPDATE SET
                 gene         = COALESCE(psc.gene,         excluded.gene),
                 product      = COALESCE(psc.product,      excluded.product),
                 cog_category = COALESCE(psc.cog_category, excluded.cog_category),
                 ec_ids       = COALESCE(psc.ec_ids,       excluded.ec_ids),
                 go_ids       = COALESCE(psc.go_ids,       excluded.go_ids)",
        )
    };

    if let Some(ref uniref100) = e.uniref100_id {
        conn.execute(
            ips_sql,
            params![uniref100, e.uniref90_id, e.gene, e.product, e.ec_ids, e.go_ids],
        )?;
    }
    if let Some(ref uniref90) = e.uniref90_id {
        conn.execute(
            psc_sql,
            params![uniref90, e.gene, e.product, e.cog_category, e.ec_ids, e.go_ids],
        )?;
    }
    Ok(())
}

enum VoteChange {
    New,
    Changed,
    Same,
}

/// Records / replaces the contributor's vote for this hash.
fn upsert_submission(
    conn: &Connection,
    hash_bytes: &[u8],
    e: &CustomAnnotationEntry,
    key: &str,
    contributor: &str,
    job_id: &str,
    now: &str,
) -> Result<VoteChange, rusqlite::Error> {
    let previous: Option<String> = conn
        .query_row(
            "SELECT annotation_key FROM annotation_submissions
             WHERE hash = ?1 AND contributor = ?2",
            params![hash_bytes, contributor],
            |row| row.get(0),
        )
        .optional()?;

    if previous.as_deref() == Some(key) {
        return Ok(VoteChange::Same);
    }

    conn.execute(
        "INSERT INTO annotation_submissions
             (hash, annotation_key, contributor, job_id, submitted_at, length,
              uniparc_id, ncbi_nrp_id, uniref100_id, uniref90_id, gene, product,
              ec_ids, go_ids, cog_category, source, tool_version, workflow_mode,
              psc_identity, psc_evalue)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
         ON CONFLICT(hash, contributor) DO UPDATE SET
             annotation_key = excluded.annotation_key, job_id = excluded.job_id,
             submitted_at = excluded.submitted_at, length = excluded.length,
             uniparc_id = excluded.uniparc_id, ncbi_nrp_id = excluded.ncbi_nrp_id,
             uniref100_id = excluded.uniref100_id, uniref90_id = excluded.uniref90_id,
             gene = excluded.gene, product = excluded.product, ec_ids = excluded.ec_ids,
             go_ids = excluded.go_ids, cog_category = excluded.cog_category,
             source = excluded.source, tool_version = excluded.tool_version,
             workflow_mode = excluded.workflow_mode, psc_identity = excluded.psc_identity,
             psc_evalue = excluded.psc_evalue",
        params![
            hash_bytes,
            key,
            contributor,
            job_id,
            now,
            e.length as i64,
            e.uniparc_id,
            e.ncbi_nrp_id,
            e.uniref100_id,
            e.uniref90_id,
            e.gene,
            e.product,
            e.ec_ids,
            e.go_ids,
            e.cog_category,
            e.source,
            e.tool_version,
            e.workflow_mode,
            e.psc_identity,
            e.psc_evalue,
        ],
    )?;

    Ok(if previous.is_some() {
        VoteChange::Changed
    } else {
        VoteChange::New
    })
}

/// Rebuilds a full entry from the newest submission carrying `key` (used when
/// consensus replaces the stored annotation).
fn load_submission_entry(
    conn: &Connection,
    hash_bytes: &[u8],
    key: &str,
) -> Result<Option<CustomAnnotationEntry>, rusqlite::Error> {
    conn.query_row(
        "SELECT length, uniparc_id, ncbi_nrp_id, uniref100_id, uniref90_id, gene, product,
                ec_ids, go_ids, cog_category
         FROM annotation_submissions
         WHERE hash = ?1 AND annotation_key = ?2
         ORDER BY submitted_at DESC LIMIT 1",
        params![hash_bytes, key],
        |row| {
            Ok(CustomAnnotationEntry {
                md5_hash: String::new(),
                length: row.get::<_, i64>(0)? as usize,
                uniparc_id: row.get(1)?,
                ncbi_nrp_id: row.get(2)?,
                uniref100_id: row.get(3)?,
                uniref90_id: row.get(4)?,
                gene: row.get(5)?,
                product: row.get(6)?,
                ec_ids: row.get(7)?,
                go_ids: row.get(8)?,
                cog_category: row.get(9)?,
                source: None,
                tool_version: None,
                workflow_mode: None,
                psc_identity: None,
                psc_evalue: None,
            })
        },
    )
    .optional()
}

/// Ingest one entry as a contribution. Never overwrites an existing annotation
/// on a single submission.
fn ingest_one(
    conn: &Connection,
    e: &CustomAnnotationEntry,
    contributor: &str,
    job_id: &str,
    threshold: i64,
    stats: &mut IngestStats,
) -> Result<(), rusqlite::Error> {
    let Some(hash_bytes) = hex_to_bytes(&e.md5_hash) else {
        tracing::warn!("AI-DB annotations DB: invalid MD5 hex – skipping");
        return Ok(());
    };
    let key = annotation_key(e);
    let now = Utc::now().to_rfc3339();

    let existing: Option<(String, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT status, annotation_key, reviewed_at FROM ups WHERE hash = ?1",
            params![hash_bytes],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;

    // ── New hash: insert as candidate (or confirmed if threshold is 1) ───────
    let Some((old_status, canonical_key, reviewed_at)) = existing else {
        let status = if threshold <= 1 { STATUS_CONFIRMED } else { STATUS_CANDIDATE };
        write_annotation(conn, &hash_bytes, e, &key, status, 1, &now, false)?;
        upsert_submission(conn, &hash_bytes, e, &key, contributor, job_id, &now)?;
        stats.inserted += 1;
        if status == STATUS_CONFIRMED {
            stats.newly_confirmed += 1;
        }
        return Ok(());
    };

    // ── Existing hash: record the vote, never overwrite on its own ───────────
    let change = upsert_submission(conn, &hash_bytes, e, &key, contributor, job_id, &now)?;
    if matches!(change, VoteChange::Same) {
        stats.unchanged += 1;
        return Ok(());
    }
    stats.reinforced += 1;

    // Admin-reviewed and legacy entries (no recorded annotation key) are not
    // changed by votes; the contribution is kept for later review.
    let Some(canonical_key) = canonical_key else { return Ok(()) };
    if reviewed_at.is_some() {
        return Ok(());
    }

    // ── Consensus ────────────────────────────────────────────────────────────
    let mut stmt = conn.prepare(
        "SELECT annotation_key, COUNT(*) FROM annotation_submissions
         WHERE hash = ?1 GROUP BY annotation_key",
    )?;
    let counts: Vec<(String, i64)> = stmt
        .query_map(params![hash_bytes], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    let canonical_count = counts
        .iter()
        .find(|(k, _)| *k == canonical_key)
        .map(|(_, c)| *c)
        .unwrap_or(0);
    let qualified: Vec<&(String, i64)> = counts.iter().filter(|(_, c)| *c >= threshold).collect();

    let (new_status, confirmations, replace_with): (&str, i64, Option<&str>) = match qualified.len()
    {
        0 => {
            let s = if counts.len() > 1 { STATUS_CONFLICTED } else { STATUS_CANDIDATE };
            (s, canonical_count, None)
        }
        1 => {
            let (k, c) = qualified[0];
            if *k == canonical_key {
                (STATUS_CONFIRMED, *c, None)
            } else {
                // A different annotation reached consensus on its own: it replaces
                // the first-come annotation.
                (STATUS_CONFIRMED, *c, Some(k.as_str()))
            }
        }
        _ => (STATUS_CONFLICTED, canonical_count, None),
    };

    if let Some(winner_key) = replace_with {
        if let Some(winner) = load_submission_entry(conn, &hash_bytes, winner_key)? {
            write_annotation(
                conn, &hash_bytes, &winner, winner_key, new_status, confirmations, &now, true,
            )?;
        }
    } else {
        conn.execute(
            "UPDATE ups SET status = ?2, confirmations = ?3 WHERE hash = ?1",
            params![hash_bytes, new_status, confirmations],
        )?;
    }

    if new_status == STATUS_CONFIRMED && old_status != STATUS_CONFIRMED {
        stats.newly_confirmed += 1;
    }
    Ok(())
}

/// Ingest verified entries as contributions of `contributor` (pseudonymous id)
/// from job `job_id`, in a single transaction.
///
/// * New hashes are added as `candidate` (or `confirmed` if the threshold is 1).
/// * Existing entries are NOT overwritten: the submission is recorded as a
///   vote; once `AI_DB_CONFIRMATIONS` independent contributors agree the entry
///   becomes `confirmed`.
/// * Re-submitting the same annotation is idempotent.
pub fn ingest_custom_annotations(
    conn: &Connection,
    entries: &[CustomAnnotationEntry],
    contributor: &str,
    job_id: &str,
) -> Result<IngestStats, rusqlite::Error> {
    let threshold = confirmation_threshold();
    let mut stats = IngestStats::default();
    let tx = conn.unchecked_transaction()?;
    for entry in entries {
        ingest_one(&tx, entry, contributor, job_id, threshold, &mut stats)?;
    }
    tx.commit()?;
    tracing::info!(
        "AI-DB annotations DB: {} new candidates, {} reinforced, {} unchanged, {} newly confirmed",
        stats.inserted,
        stats.reinforced,
        stats.unchanged,
        stats.newly_confirmed
    );
    Ok(stats)
}

/// Contributions by `contributor` within the last `hours` hours (rate limiting).
pub fn count_recent_contributions(
    conn: &Connection,
    contributor: &str,
    hours: i64,
) -> Result<usize, rusqlite::Error> {
    let since = (Utc::now() - Duration::hours(hours)).to_rfc3339();
    conn.query_row(
        "SELECT COUNT(*) FROM annotation_submissions
         WHERE contributor = ?1 AND submitted_at > ?2",
        params![contributor, since],
        |row| row.get::<_, i64>(0).map(|c| c as usize),
    )
}

/// One row of the admin listing of annotation entries.
#[derive(Debug, Clone)]
pub struct AdminAnnotationRow {
    pub md5_hash: String,
    pub status: String,
    pub confirmations: i64,
    pub length: i64,
    pub product: Option<String>,
    pub uniref100_id: Option<String>,
    pub contributors: i64,
    pub updated_at: Option<String>,
}

/// Lists entries with the given status (newest first) for admin review.
pub fn list_annotations_by_status(
    conn: &Connection,
    status: &str,
    limit: usize,
) -> Result<Vec<AdminAnnotationRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT lower(hex(u.hash)), u.status, u.confirmations, u.length,
                COALESCE(u.product, i.product), u.uniref100_id,
                (SELECT COUNT(*) FROM annotation_submissions s WHERE s.hash = u.hash),
                u.updated_at
         FROM ups u LEFT JOIN ips i ON i.uniref100_id = u.uniref100_id
         WHERE u.status = ?1
         ORDER BY u.updated_at DESC
         LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(params![status, limit as i64], |row| {
            Ok(AdminAnnotationRow {
                md5_hash: row.get(0)?,
                status: row.get(1)?,
                confirmations: row.get(2)?,
                length: row.get(3)?,
                product: row.get(4)?,
                uniref100_id: row.get(5)?,
                contributors: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

/// Admin review of an entry: `confirmed`, `rejected` or `candidate`.
/// Locks the entry against automatic consensus changes.
/// Exposed via `POST /api/admin/annotations/{md5}/review` (admin secret).
pub fn review_annotation(
    conn: &Connection,
    md5_hex: &str,
    status: &str,
) -> Result<bool, rusqlite::Error> {
    if ![STATUS_CONFIRMED, STATUS_REJECTED, STATUS_CANDIDATE].contains(&status) {
        return Ok(false);
    }
    let Some(hash_bytes) = hex_to_bytes(&md5_hex.to_lowercase()) else {
        return Ok(false);
    };
    let rows = conn.execute(
        "UPDATE ups SET status = ?2, reviewed_at = ?3 WHERE hash = ?1",
        params![hash_bytes, status, Utc::now().to_rfc3339()],
    )?;
    Ok(rows > 0)
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

    let failed_increment = if job.status == JobStatus::Failed { 1 } else { 0 };

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
pub fn get_aidb_growth_by_month(
    conn: &Connection,
) -> Result<Vec<(String, i64)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT strftime('%Y-%m', created_at) AS month, COUNT(*) AS new_sequences
         FROM ups
         WHERE created_at IS NOT NULL
         GROUP BY month
         ORDER BY month DESC",
    )?;

    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

#[cfg(test)]
mod curation_tests {
    use super::*;

    /// Minimal Bakta-like schema + the curation migration, in memory.
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
        ensure_provenance_schema(&conn).unwrap();
        // idempotent
        ensure_provenance_schema(&conn).unwrap();
        conn
    }

    fn entry(hash: &str, gene: &str, product: &str) -> CustomAnnotationEntry {
        CustomAnnotationEntry {
            md5_hash: hash.to_string(),
            length: 100,
            uniparc_id: None,
            ncbi_nrp_id: None,
            uniref100_id: Some("UniRef90_A".into()),
            uniref90_id: Some("UniRef90_A".into()),
            gene: Some(gene.into()),
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

    fn state_of(conn: &Connection) -> (String, i64) {
        let hash = hex_to_bytes(H).unwrap();
        conn.query_row(
            "SELECT status, confirmations FROM ups WHERE hash = ?1",
            params![hash],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    fn product_of(conn: &Connection) -> String {
        let hash = hex_to_bytes(H).unwrap();
        conn.query_row("SELECT product FROM ups WHERE hash = ?1", params![hash], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn first_contribution_is_a_candidate() {
        let conn = db();
        let s = ingest_custom_annotations(&conn, &[entry(H, "dnaK", "chaperone")], "c1", "j1").unwrap();
        assert_eq!(s.inserted, 1);
        assert_eq!(state_of(&conn), ("candidate".to_string(), 1));
    }

    #[test]
    fn resubmission_by_same_contributor_is_idempotent() {
        let conn = db();
        let e = entry(H, "dnaK", "chaperone");
        ingest_custom_annotations(&conn, &[e.clone()], "c1", "j1").unwrap();
        let s = ingest_custom_annotations(&conn, &[e], "c1", "j1").unwrap();
        assert_eq!(s.unchanged, 1);
        assert_eq!(state_of(&conn), ("candidate".to_string(), 1));
    }

    #[test]
    fn independent_agreement_confirms() {
        let conn = db();
        let e = entry(H, "dnaK", "chaperone");
        ingest_custom_annotations(&conn, &[e.clone()], "c1", "j1").unwrap();
        let s = ingest_custom_annotations(&conn, &[e], "c2", "j2").unwrap();
        assert_eq!(s.newly_confirmed, 1);
        assert_eq!(state_of(&conn), ("confirmed".to_string(), 2));
    }

    #[test]
    fn single_conflicting_submission_never_overwrites() {
        let conn = db();
        ingest_custom_annotations(&conn, &[entry(H, "dnaK", "chaperone")], "c1", "j1").unwrap();
        ingest_custom_annotations(&conn, &[entry(H, "evil", "junk")], "c2", "j2").unwrap();
        assert_eq!(product_of(&conn), "chaperone");
        assert_eq!(state_of(&conn).0, "conflicted");
    }

    #[test]
    fn consensus_replaces_a_poisoned_first_annotation() {
        let conn = db();
        ingest_custom_annotations(&conn, &[entry(H, "evil", "junk")], "attacker", "j0").unwrap();
        ingest_custom_annotations(&conn, &[entry(H, "dnaK", "chaperone")], "c1", "j1").unwrap();
        ingest_custom_annotations(&conn, &[entry(H, "dnaK", "chaperone")], "c2", "j2").unwrap();
        assert_eq!(product_of(&conn), "chaperone");
        assert_eq!(state_of(&conn), ("confirmed".to_string(), 2));
    }

    #[test]
    fn changed_vote_of_same_contributor_replaces_the_old_one() {
        let conn = db();
        ingest_custom_annotations(&conn, &[entry(H, "evil", "junk")], "c1", "j1").unwrap();
        ingest_custom_annotations(&conn, &[entry(H, "dnaK", "chaperone")], "c1", "j2").unwrap();
        let votes: i64 = conn
            .query_row("SELECT COUNT(*) FROM annotation_submissions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(votes, 1);
    }

    #[test]
    fn admin_review_locks_and_rejected_entries_are_hidden() {
        let conn = db();
        ingest_custom_annotations(&conn, &[entry(H, "dnaK", "chaperone")], "c1", "j1").unwrap();
        assert!(review_annotation(&conn, H, STATUS_REJECTED).unwrap());
        // further agreement does not change an admin decision
        ingest_custom_annotations(&conn, &[entry(H, "dnaK", "chaperone")], "c2", "j2").unwrap();
        assert_eq!(state_of(&conn).0, "rejected");
    }

    #[test]
    fn recent_contributions_are_counted_per_contributor() {
        let conn = db();
        ingest_custom_annotations(&conn, &[entry(H, "dnaK", "chaperone")], "c1", "j1").unwrap();
        assert_eq!(count_recent_contributions(&conn, "c1", 24).unwrap(), 1);
        assert_eq!(count_recent_contributions(&conn, "c2", 24).unwrap(), 0);
    }

    #[test]
    fn entry_validation_rejects_hostile_values() {
        let ok = entry(H, "dnaK", "chaperone");
        assert!(ok.normalized().is_ok());
        for bad in [
            CustomAnnotationEntry { product: Some("=HYPERLINK(\"x\")".into()), ..ok.clone() },
            CustomAnnotationEntry { product: Some("a\tb".into()), ..ok.clone() },
            CustomAnnotationEntry { ec_ids: Some("not-an-ec".into()), ..ok.clone() },
            CustomAnnotationEntry { go_ids: Some("GO:12".into()), ..ok.clone() },
            CustomAnnotationEntry { md5_hash: "zz".into(), ..ok.clone() },
            CustomAnnotationEntry { uniref90_id: Some("bad".into()), ..ok.clone() },
            CustomAnnotationEntry { source: Some("manual".into()), ..ok.clone() },
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
        conn.execute("INSERT INTO jobs VALUES ('j1', ?1), ('j2', NULL)", params![raw]).unwrap();
        conn.execute("INSERT INTO kpi_monthly_owners VALUES ('2026-10', ?1)", params![raw]).unwrap();

        assert_eq!(migrate_owner_ids_to_digest(&conn).unwrap(), 1);
        assert_eq!(migrate_kpi_owner_ids_to_digest(&conn).unwrap(), 1);
        let job_owner: String = conn
            .query_row("SELECT owner_id FROM jobs WHERE job_id = 'j1'", [], |r| r.get(0))
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
