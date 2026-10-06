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
//! If ADMIN_KPI_SECRET is not set, the endpoint is disabled entirely (503)
//! rather than silently allowing unauthenticated access.
//!
//! The same secret protects the community-curation review routes:
//!   GET  /api/admin/annotations?status=candidate|confirmed|conflicted|legacy|rejected
//!   POST /api/admin/annotations/{md5}/review   {"status": "confirmed|rejected|candidate"}
//! ============================================================================

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::env;
use utoipa::ToSchema;

use crate::models::ErrorResponse;
use crate::state::AppState;
use crate::storage::KpiMonthRow;

const ADMIN_SECRET_ENV: &str = "ADMIN_KPI_SECRET";
const ADMIN_SECRET_HEADER: &str = "x-admin-secret";

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

/// Checks the X-Admin-Secret header against the ADMIN_KPI_SECRET env var.
/// Returns Err with the appropriate status/response if the check fails.
fn check_admin_secret(headers: &HeaderMap) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let configured = env::var(ADMIN_SECRET_ENV).map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(format!(
                "Admin KPI endpoint disabled: {ADMIN_SECRET_ENV} is not set on the server"
            ))),
        )
    })?;

    let provided = headers
        .get(ADMIN_SECRET_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Constant-time-ish comparison isn't critical here (this is an internal
    // admin tool, not a public auth boundary), but avoid short-circuiting on
    // length to make timing side-channels slightly harder regardless.
    let matches = provided.len() == configured.len()
        && provided
        .bytes()
        .zip(configured.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
        && !provided.is_empty();

    if matches {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(format!(
                "Missing or invalid {ADMIN_SECRET_HEADER} header"
            ))),
        ))
    }
}

/// Returns monthly KPI figures: job counts, sequence throughput, match-source
/// breakdown, Bakta/Psos usage, active users, and AI-DB annotations DB growth.
///
/// Protected by a shared secret sent in the `X-Admin-Secret` header (see
/// ADMIN_KPI_SECRET environment variable). Not a full auth system – intended
/// for internal/admin use only.
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
    headers: HeaderMap,
) -> Result<Json<KpiOverviewResponse>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_secret(&headers)?;

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
// ─────────────────────────────────────────────────────────────────────────────
// Community-curation review (admin)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AdminAnnotationsQuery {
    /// candidate | confirmed | conflicted | legacy | rejected (default: conflicted)
    pub status: Option<String>,
    /// Maximum number of rows (default 100, max 1000)
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminAnnotationEntry {
    /// MD5 of the protein sequence (hex)
    pub md5_hash: String,
    pub status: String,
    /// Number of independent contributors that submitted the stored annotation
    pub confirmations: i64,
    pub length: i64,
    pub product: Option<String>,
    pub uniref100_id: Option<String>,
    /// Number of distinct contributors that submitted anything for this hash
    pub contributors: i64,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminAnnotationsResponse {
    pub status: String,
    pub entries: Vec<AdminAnnotationEntry>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewAnnotationRequest {
    /// confirmed | rejected | candidate
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReviewAnnotationResponse {
    pub md5_hash: String,
    pub status: String,
}

const LISTABLE: [&str; 5] = ["candidate", "confirmed", "conflicted", "legacy", "rejected"];
const REVIEWABLE: [&str; 3] = ["confirmed", "rejected", "candidate"];

fn bad_request(msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::BAD_REQUEST, Json(ErrorResponse::new(msg)))
}

/// Lists curation entries of one status for review.
///
/// Protected by the same `X-Admin-Secret` as the KPI overview.
#[utoipa::path(
    get,
    path = "/api/admin/annotations",
    tag = "admin",
    params(
        ("status" = Option<String>, Query, description = "candidate | confirmed | conflicted | legacy | rejected (default conflicted)"),
        ("limit" = Option<usize>, Query, description = "Maximum rows, default 100, max 1000"),
    ),
    responses(
        (status = 200, description = "Entries with the requested status", body = AdminAnnotationsResponse),
        (status = 400, description = "Unknown status", body = ErrorResponse),
        (status = 401, description = "Missing or invalid admin secret", body = ErrorResponse),
        (status = 503, description = "Endpoint not configured", body = ErrorResponse),
    )
)]
pub async fn list_admin_annotations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<AdminAnnotationsQuery>,
) -> Result<Json<AdminAnnotationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_secret(&headers)?;
    let status = q.status.unwrap_or_else(|| "conflicted".to_string());
    if !LISTABLE.contains(&status.as_str()) {
        return Err(bad_request(format!(
            "Unknown status '{status}' (expected one of {})",
            LISTABLE.join(", ")
        )));
    }
    let limit = q.limit.unwrap_or(100).clamp(1, 1000);
    let rows = state
        .list_custom_annotations(&status, limit)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))))?;
    let entries = rows
        .into_iter()
        .map(|r| AdminAnnotationEntry {
            md5_hash: r.md5_hash,
            status: r.status,
            confirmations: r.confirmations,
            length: r.length,
            product: r.product,
            uniref100_id: r.uniref100_id,
            contributors: r.contributors,
            updated_at: r.updated_at,
        })
        .collect();
    Ok(Json(AdminAnnotationsResponse { status, entries }))
}

/// Sets the review status of one entry. `confirmed` and `rejected` lock the
/// entry against further consensus changes; `candidate` releases it again.
#[utoipa::path(
    post,
    path = "/api/admin/annotations/{md5}/review",
    tag = "admin",
    params(("md5" = String, Path, description = "MD5 of the sequence (32 hex characters)")),
    request_body = ReviewAnnotationRequest,
    responses(
        (status = 200, description = "Status updated", body = ReviewAnnotationResponse),
        (status = 400, description = "Invalid hash or status", body = ErrorResponse),
        (status = 401, description = "Missing or invalid admin secret", body = ErrorResponse),
        (status = 404, description = "Entry not found", body = ErrorResponse),
        (status = 503, description = "Endpoint not configured", body = ErrorResponse),
    )
)]
pub async fn review_admin_annotation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(md5): Path<String>,
    Json(body): Json<ReviewAnnotationRequest>,
) -> Result<Json<ReviewAnnotationResponse>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_secret(&headers)?;
    let md5 = md5.to_ascii_lowercase();
    if md5.len() != 32 || !md5.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad_request("md5 must be 32 hexadecimal characters".into()));
    }
    if !REVIEWABLE.contains(&body.status.as_str()) {
        return Err(bad_request(format!(
            "status must be one of {}",
            REVIEWABLE.join(", ")
        )));
    }
    match state.review_custom_annotation(&md5, &body.status) {
        Ok(true) => Ok(Json(ReviewAnnotationResponse {
            md5_hash: md5,
            status: body.status,
        })),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("No entry with this hash")),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e)))),
    }
}
