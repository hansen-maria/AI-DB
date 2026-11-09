//! ============================================================================
//! Admin KPI overview handler
//!
//! Route: GET /api/admin/kpis
//!
//! Protected by a shared secret (NOT a full auth system) – suitable for an
//! internal/admin-only endpoint hit manually or from a simple internal
//! dashboard. Set the ADMIN_KPI_SECRET environment variable on the server;
//! requests must send the same value in the X-Admin-Secret header.
//!
//! The check is done centrally by `auth::require_admin_secret`, which is
//! applied as a route layer to all `/api/admin/*` routes in `main.rs`. Only
//! the admin secret authorizes there; owner tokens and cookies are ignored.
//! If ADMIN_KPI_SECRET is not set, the routes are disabled entirely (503)
//! rather than silently allowing unauthenticated access.
//!
//! The handlers below therefore assume an already authorized admin request.
//! Their security requirement (`admin_secret`) is added to the OpenAPI doc by
//! the `SecurityAddon` modifier in `main.rs`.
//! ============================================================================

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::models::ErrorResponse;
use crate::state::AppState;
use crate::storage::KpiMonthRow;

/// One month's combined KPI figures (jobs.db counters + AI-DB annotations DB growth).
#[derive(Debug, Serialize, ToSchema)]
pub struct KpiMonthEntry {
    /// "YYYY-MM"
    pub month: String,
    /// Jobs that reached a final state (Completed or Failed) this month
    pub jobs_created: i64,
    /// Of those, how many failed
    pub jobs_failed: i64,
    /// Total sequences processed across all jobs this month
    pub sequences_processed: i64,
    /// Hash matches found in the official Bakta DB
    pub hash_matches_bakta: i64,
    /// Hash matches found in the AI-DB annotations DB
    pub hash_matches_aidb: i64,
    /// Bakta annotation jobs started (V1 nucleotide + V2 protein combined)
    pub bakta_jobs_started: i64,
    /// Sequences analyzed via Psos
    pub psos_analyses: i64,
    /// Distinct owners (users) who ran at least one job this month
    pub active_owners: i64,
    /// New sequences saved into the AI-DB annotations DB this month
    /// (i.e. growth of the "gespeicherte Sequenzen" KPI)
    pub aidb_sequences_saved: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KpiOverviewResponse {
    pub months: Vec<KpiMonthEntry>,
}

/// Returns monthly KPI figures: job counts, sequence throughput, match-source
/// breakdown, Bakta/Psos usage, active users, and AI-DB annotations DB growth.
///
/// Requires the shared admin secret in the `X-Admin-Secret` header (see
/// ADMIN_KPI_SECRET environment variable). Owner tokens are not accepted.
#[utoipa::path(
    get,
    path = "/api/admin/kpis",
    tag = "admin",
    responses(
        (status = 200, description = "Monthly KPI overview", body = KpiOverviewResponse),
        (status = 401, description = "Missing or invalid admin secret", body = ErrorResponse),
        (status = 503, description = "Endpoint not configured (ADMIN_KPI_SECRET unset)", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    )
)]
pub async fn get_kpi_overview(
    State(state): State<AppState>,
) -> Result<Json<KpiOverviewResponse>, (StatusCode, Json<ErrorResponse>)> {
    let rows: Vec<KpiMonthRow> = state.get_kpi_overview().map_err(|e| {
        tracing::error!("Failed to load KPI overview: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(e)),
        )
    })?;

    let aidb_growth = state.get_aidb_growth_by_month();

    let months = rows
        .into_iter()
        .map(|r| KpiMonthEntry {
            aidb_sequences_saved: aidb_growth.get(&r.month).copied().unwrap_or(0),
            month: r.month,
            jobs_created: r.jobs_created,
            jobs_failed: r.jobs_failed,
            sequences_processed: r.sequences_processed,
            hash_matches_bakta: r.hash_matches_bakta,
            hash_matches_aidb: r.hash_matches_aidb,
            bakta_jobs_started: r.bakta_jobs_started,
            psos_analyses: r.psos_analyses,
            active_owners: r.active_owners,
        })
        .collect();

    Ok(Json(KpiOverviewResponse { months }))
}
