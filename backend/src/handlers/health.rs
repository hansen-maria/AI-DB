//! ============================================================================
//! Health check and database info handlers
//! ============================================================================

use crate::models::{DbInfoResponse, HealthCheckResponse};
use crate::services::annotation::get_bakta_db_release;
use crate::state::AppState;
use axum::{extract::State, response::IntoResponse, Json};

/// Health check endpoint - includes database status
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service health status", body = HealthCheckResponse)
    )
)]
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_status = if let Some(path) = state.bakta_db_path() {
        if path.exists() {
            // Try to open connection to verify database is accessible
            match state.open_db_connection() {
                Some(_) => "connected",
                None => "error",
            }
        } else {
            "not_found"
        }
    } else {
        "not_configured"
    };

    Json(serde_json::json!({
        "status": "healthy",
        "service": "ai-db-api",
        "bakta_db": {
            "status": db_status,
            "path": state.bakta_db_path().map(|p| p.display().to_string())
        }
    }))
}

/// Database info endpoint - provides details about the Bakta database
#[utoipa::path(
    get,
    path = "/api/db/info",
    tag = "Database",
    responses(
        (status = 200, description = "Database information", body = DbInfoResponse)
    )
)]
pub async fn db_info(State(state): State<AppState>) -> impl IntoResponse {
    // Kept the original flat shape (available/path/ups_entries/version) for backward
    // compatibility with existing consumers, plus a "release" alias and a new
    // nested "aidb_db" block for the AI-DB annotations DB's freshness info.
    //
    // Note: the release/version comes from `version.json` on disk next to bakta.db,
    // NOT from a SQL table inside bakta.db (Bakta doesn't store it there).
    let release = state
        .bakta_db_path()
        .and_then(|p| p.parent())
        .and_then(get_bakta_db_release);

    let mut db_info = if let Some(conn) = state.open_db_connection() {
        // Get row count from ups table
        let ups_count: Result<i64, _> =
            conn.query_row("SELECT COUNT(*) FROM ups", [], |row| row.get(0));

        serde_json::json!({
            "available": true,
            "path": state.bakta_db_path().map(|p| p.display().to_string()),
            "ups_entries": ups_count.ok(),
            "version": release,
            "release": release
        })
    } else {
        serde_json::json!({
            "available": false,
            "path": state.bakta_db_path().map(|p| p.display().to_string()),
            "error": "Could not connect to database"
        })
    };

    // AI-DB annotations DB: report entry count and the most recent ingest timestamp,
    // so the user can see both sources' "freshness" from a single endpoint.
    let aidb_info = if let Some(conn) = state.open_custom_annotations_db() {
        let ups_count: Result<i64, _> =
            conn.query_row("SELECT COUNT(*) FROM ups", [], |row| row.get(0));
        let last_updated: Option<String> = conn
            .query_row("SELECT MAX(updated_at) FROM ups", [], |row| row.get(0))
            .ok()
            .flatten();

        serde_json::json!({
            "available": true,
            "ups_entries": ups_count.ok(),
            "last_updated": last_updated
        })
    } else {
        serde_json::json!({ "available": false })
    };

    if let Some(obj) = db_info.as_object_mut() {
        obj.insert("aidb_db".to_string(), aidb_info);
    }

    Json(db_info)
}
