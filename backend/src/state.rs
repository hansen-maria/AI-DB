//! ============================================================================
//! Application State
//! ============================================================================

use parking_lot::RwLock;
use rusqlite::{Connection, OpenFlags};
use std::{collections::HashMap, env, path::PathBuf, sync::Arc};

use crate::models::JobResponse;
use crate::storage;

/// Default path for jobs database
const DEFAULT_JOBS_DB: &str = "/data/jobs.db";

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// In-memory job cache
    jobs: Arc<RwLock<HashMap<String, JobResponse>>>,
    /// Path to Bakta SQLite database
    bakta_db_path: Option<PathBuf>,
    /// Path to jobs database (open connections on-demand)
    jobs_db_path: PathBuf,
}

impl AppState {
    pub fn new() -> Self {
        // Get Bakta database path from environment variable
        let bakta_db_path = env::var("BAKTA_DB")
            .ok()
            .map(|p| PathBuf::from(p).join("bakta.db"))
            .or_else(|| {
                // Fallback paths
                let fallback_paths = [
                    PathBuf::from("/bakta-db/bakta.db"),
                    PathBuf::from("/opt/bakta-db/bakta.db"),
                    PathBuf::from("/mnt/bakta-db/db/bakta.db"),
                ];
                fallback_paths.into_iter().find(|p| p.exists())
            });

        if let Some(ref path) = bakta_db_path {
            if path.exists() {
                tracing::info!("Bakta database found at: {:?}", path);
            } else {
                tracing::warn!(
                    "Bakta database path configured but file not found: {:?}",
                    path
                );
            }
        } else {
            tracing::warn!("No Bakta database configured. Set BAKTA_DB environment variable.");
            tracing::warn!("Hash lookups will return no matches.");
        }

        // Initialize jobs database path
        let jobs_db_path = env::var("AI_DB_JOBS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_JOBS_DB));

        // Ensure parent directory exists
        if let Some(parent) = jobs_db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).ok();
            }
        }

        // Initialize the database schema
        let jobs_db =
            storage::init_database(&jobs_db_path).expect("Failed to initialize jobs database");

        // Cleanup old jobs on startup
        if let Err(e) = storage::cleanup_old_jobs(&jobs_db) {
            tracing::warn!("Failed to cleanup old jobs: {}", e);
        }

        // Load existing jobs into memory
        let jobs_map = Self::load_jobs_from_db(&jobs_db);
        let job_count = jobs_map.len();

        // Drop the connection - we'll open new ones as needed
        drop(jobs_db);

        tracing::info!("Loaded {} existing jobs from database", job_count);

        Self {
            jobs: Arc::new(RwLock::new(jobs_map)),
            bakta_db_path,
            jobs_db_path,
        }
    }

    /// Load all jobs from database
    fn load_jobs_from_db(conn: &Connection) -> HashMap<String, JobResponse> {
        use crate::models::{JobStatus, SequenceInfo};
        use chrono::Utc;

        let mut jobs_map = HashMap::new();

        let mut stmt = match conn.prepare(
            "SELECT job_id, owner_id, status, created_at, updated_at, filename,
                    sequence_count, processed_count, hash_matches, alignment_matches,
                    error_message, sequences FROM jobs",
        ) {
            Ok(stmt) => stmt,
            Err(e) => {
                tracing::error!("Failed to prepare statement: {}", e);
                return jobs_map;
            }
        };

        let job_iter = match stmt.query_map([], |row| {
            let status_str: String = row.get(2)?;
            let status = match status_str.as_str() {
                "pending" => JobStatus::Pending,
                "processing" => JobStatus::Processing,
                "completed" => JobStatus::Completed,
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
        }) {
            Ok(iter) => iter,
            Err(e) => {
                tracing::error!("Failed to query jobs: {}", e);
                return jobs_map;
            }
        };

        for job_result in job_iter {
            if let Ok(job) = job_result {
                jobs_map.insert(job.job_id.clone(), job);
            }
        }

        jobs_map
    }

    /// Opens a new connection to the jobs database
    fn open_jobs_db(&self) -> Option<Connection> {
        Connection::open(&self.jobs_db_path)
            .map_err(|e| {
                tracing::error!("Failed to open jobs database: {}", e);
                e
            })
            .ok()
    }

    /// Opens a read-only connection to the Bakta database
    pub fn open_db_connection(&self) -> Option<Connection> {
        self.bakta_db_path.as_ref().and_then(|path| {
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| {
                    tracing::error!("Failed to open Bakta database: {}", e);
                    e
                })
                .ok()
        })
    }

    /// Returns the Bakta database path if configured
    pub fn bakta_db_path(&self) -> Option<&PathBuf> {
        self.bakta_db_path.as_ref()
    }

    /// Returns a read lock on the jobs cache
    pub fn jobs(&self) -> parking_lot::RwLockReadGuard<HashMap<String, JobResponse>> {
        self.jobs.read()
    }

    /// Returns a write lock on the jobs cache
    pub fn jobs_mut(&self) -> parking_lot::RwLockWriteGuard<HashMap<String, JobResponse>> {
        self.jobs.write()
    }

    /// Saves a job to both memory and database
    pub fn save_job(&self, job: &JobResponse) {
        // Save to database
        if let Some(conn) = self.open_jobs_db() {
            if let Err(e) = storage::save_job(&conn, job) {
                tracing::error!("Failed to persist job {}: {}", job.job_id, e);
            }
        }

        // Update memory cache
        self.jobs.write().insert(job.job_id.clone(), job.clone());
    }

    /// Deletes a job from both memory and database
    pub fn delete_job(&self, job_id: &str) -> bool {
        // Delete from database
        let db_deleted = if let Some(conn) = self.open_jobs_db() {
            storage::delete_job(&conn, job_id).unwrap_or(false)
        } else {
            false
        };

        // Remove from memory
        let mem_deleted = self.jobs.write().remove(job_id).is_some();

        db_deleted || mem_deleted
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
