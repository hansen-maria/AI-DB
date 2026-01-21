//! ============================================================================
//! Health-Check types and responses
//! ============================================================================

use serde::Serialize;
use utoipa::ToSchema;

/// Service Health Check Response
#[derive(Serialize, ToSchema)]
pub struct HealthCheckResponse {
    pub status: String,
    pub service: String,
    pub bakta_db: BaktaDbHealth,
}

/// Bakta Database Health Check Response
#[derive(Serialize, ToSchema)]
pub struct BaktaDbHealth {
    /// Database connection status
    /// Possible values: connected, error, not_found, not_configured
    pub status: String,
    /// Filesystem path to the Bakta database
    pub path: Option<String>,
}

/// Database Info Response
#[derive(Serialize, ToSchema)]
pub struct DbInfoResponse {
    /// Indicates whether the database is accessible
    pub available: bool,
    /// Filesystem path to the database
    pub path: Option<String>,
    /// Number of entries in the `ups` table
    pub ups_entries: Option<i64>,
    /// Bakta database version (if available)
    pub version: Option<String>,
    /// Error message if database is not accessible
    pub error: Option<String>,
}
