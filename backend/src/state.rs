//! ============================================================================
//! Application State
//! ============================================================================

use parking_lot::RwLock;
use rusqlite::{Connection, OpenFlags};
use std::{collections::HashMap, env, path::PathBuf, sync::Arc};

use crate::models::JobResponse;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// In-memory job storage
    jobs: Arc<RwLock<HashMap<String, JobResponse>>>,
    /// Path to Bakta SQLite database
    bakta_db_path: Option<PathBuf>,
}

impl AppState {
    pub fn new() -> Self {
        // Get database path from environment variable
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

        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            bakta_db_path,
        }
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

    /// Returns a read lock on the jobs storage
    pub fn jobs(&self) -> parking_lot::RwLockReadGuard<HashMap<String, JobResponse>> {
        self.jobs.read()
    }

    /// Returns a write lock on the jobs storage
    pub fn jobs_mut(&self) -> parking_lot::RwLockWriteGuard<HashMap<String, JobResponse>> {
        self.jobs.write()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
