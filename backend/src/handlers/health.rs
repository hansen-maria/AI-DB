//! ============================================================================
//! Health check and database info handlers
//! ============================================================================

use crate::models::{DbInfoResponse, HealthCheckResponse};
use crate::state::AppState;
use axum::{extract::State, response::IntoResponse, Json};

/// Health check endpoint - includes database status
#[utoipa::path(
    get,
    path = "/health",
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
    path = "/db/info",
    tag = "Database",
    responses(
        (status = 200, description = "Database information", body = DbInfoResponse)
    )
)]
pub async fn db_info(State(state): State<AppState>) -> impl IntoResponse {
    let db_info = if let Some(conn) = state.open_db_connection() {
        // Get row count from ups table
        let ups_count: Result<i64, _> =
            conn.query_row("SELECT COUNT(*) FROM ups", [], |row| row.get(0));

        // Try to get version info if available
        let version: Option<String> = conn
            .query_row(
                "SELECT json_extract(info, '$.version') FROM version LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        serde_json::json!({
            "available": true,
            "path": state.bakta_db_path().map(|p| p.display().to_string()),
            "ups_entries": ups_count.ok(),
            "version": version
        })
    } else {
        serde_json::json!({
            "available": false,
            "path": state.bakta_db_path().map(|p| p.display().to_string()),
            "error": "Could not connect to database"
        })
    };

    Json(db_info)
}
