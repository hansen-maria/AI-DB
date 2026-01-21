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
