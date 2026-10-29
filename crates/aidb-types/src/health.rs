//! ============================================================================
//! Health-Check types and responses
//! ============================================================================

use serde::{Deserialize, Serialize};

/// Service Health Check Response
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HealthCheckResponse {
    pub status: String,
    pub service: String,
    /// Server build version (`CARGO_PKG_VERSION` of the backend), e.g. "1.0.0".
    /// Empty when talking to a server that predates this field.
    #[serde(default)]
    pub version: String,
    /// Version of the HTTP API contract, see [`crate::API_VERSION`].
    /// `0` when talking to a server that predates this field.
    #[serde(default)]
    pub api_version: u32,
    pub bakta_db: BaktaDbHealth,
}

/// Bakta Database Health Check Response
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BaktaDbHealth {
    /// Database connection status
    /// Possible values: connected, error, not_found, not_configured
    pub status: String,
    /// Filesystem path to the Bakta database
    pub path: Option<String>,
}

/// Database Info Response (`GET /api/db/info`)
///
/// Documents the response shape; the handler builds it as JSON. Fields that do
/// not apply are omitted (`ups_entries`/`version`/`release` when the Bakta DB is
/// unavailable, `error` when it is available).
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DbInfoResponse {
    /// Indicates whether the Bakta database is accessible
    pub available: bool,
    /// Filesystem path to the Bakta database
    pub path: Option<String>,
    /// Number of entries in the Bakta `ups` table
    pub ups_entries: Option<i64>,
    /// Bakta database release (from `version.json` next to bakta.db, if available)
    pub version: Option<String>,
    /// Alias of `version` (same value)
    pub release: Option<String>,
    /// Error message if the Bakta database is not accessible
    pub error: Option<String>,
    /// Status of the AI-DB annotations DB
    pub aidb_db: AidbDbInfo,
}

/// AI-DB annotations DB status (part of `DbInfoResponse`)
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AidbDbInfo {
    /// Indicates whether the AI-DB annotations DB is accessible
    pub available: bool,
    /// Number of entries in the `ups` table (omitted if unavailable)
    pub ups_entries: Option<i64>,
    /// Timestamp of the most recent entry update (omitted if unavailable)
    pub last_updated: Option<String>,
}
