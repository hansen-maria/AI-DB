//! ============================================================================
//! Health check and database info handlers
//! ============================================================================

use crate::models::{
    BakrepDbHealth, BaktaDbHealth, DbInfoResponse, HealthCheckResponse, API_VERSION,
};
use crate::services::annotation::{get_bakrep_db_release, get_bakta_db_release};
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

    // BakRep DB is optional: "not_installed" is a normal state, not an error.
    let bakrep_status = match state.bakrep_db_path() {
        None => "not_installed",
        Some(_) => match state.open_bakrep_db() {
            Some(_) => "connected",
            None => "error",
        },
    };

    Json(HealthCheckResponse {
        status: "healthy".to_string(),
        service: "ai-db-api".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: API_VERSION,
        bakta_db: BaktaDbHealth {
            status: db_status.to_string(),
            path: state.bakta_db_path().map(|p| p.display().to_string()),
        },
        bakrep_db: Some(BakrepDbHealth {
            status: bakrep_status.to_string(),
            path: state.bakrep_db_path().map(|p| p.display().to_string()),
        }),
    })
}

/// Database info endpoint - provides details about the Bakta database
#[utoipa::path(
    get,
    path = "/api/db/info",
    tag = "Health",
    responses(
        (status = 200, description = "Bakta DB, BakRep DB and AI-DB annotations DB information", body = DbInfoResponse)
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

    // BakRep DB: entry count comes from version.json (written by
    // scripts/bakrep/aidb_bakrep.py finalize) – a COUNT(*) over a table with
    // hundreds of millions of rows would make this endpoint slow.
    let bakrep_info = match (state.bakrep_db_path(), state.open_bakrep_db()) {
        (Some(path), Some(_)) => {
            let dir = path.parent();
            let entries = dir
                .and_then(|d| std::fs::read_to_string(d.join("version.json")).ok())
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v.get("entries").and_then(|e| e.as_i64()));
            serde_json::json!({
                "available": true,
                "ups_entries": entries,
                "release": dir.and_then(get_bakrep_db_release),
            })
        }
        _ => serde_json::json!({ "available": false }),
    };

    if let Some(obj) = db_info.as_object_mut() {
        obj.insert("aidb_db".to_string(), aidb_info);
        obj.insert("bakrep_db".to_string(), bakrep_info);
    }

    Json(db_info)
}
