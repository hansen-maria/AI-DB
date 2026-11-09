//! ============================================================================
//! Handlers for Bakta job persistence
//!
//! Routes:
//!   POST   /api/job/{job_id}/bakta  → save_bakta_job
//!   GET    /api/job/{job_id}/bakta  → get_bakta_job
//!   DELETE /api/job/{job_id}/bakta  → delete_bakta_job
//!   POST   /api/job/{job_id}/bakta/ingest → ingest_bakta_results
//!
//! Reading (GET) is public for anyone who knows the job ID (share link);
//! save / delete / ingest require the job owner (cookie or API token).
//! ============================================================================

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use axum_extra::extract::CookieJar;
use std::collections::{HashMap, HashSet};

use crate::auth::authorize_owner;

use crate::models::{
    BaktaJobStateResponse, ErrorResponse, IngestCustomAnnotationsRequest,
    IngestCustomAnnotationsResponse, SaveBaktaJobRequest, SaveBaktaJobResponse,
};

use crate::state::AppState;

/// Save (upsert) Bakta job state.
/// Idempotent – called on every progress step and when the job finishes.
#[utoipa::path(
    post,
    path = "/api/job/{job_id}/bakta",
    params(("job_id" = String, Path, description = "AI-DB job ID")),
    request_body = SaveBaktaJobRequest,
    responses(
        (status = 200, description = "State saved",    body = SaveBaktaJobResponse),
        (status = 403, description = "Not the job owner", body = ErrorResponse),
        (status = 404, description = "Job not found",  body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    ),
    tag = "bakta"
)]
pub async fn save_bakta_job(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(job_id): Path<String>,
    Json(request): Json<SaveBaktaJobRequest>,
) -> Result<Json<SaveBaktaJobResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_owner(&state, &job_id, &jar, &headers)?;

    // Only count towards the "Bakta jobs started" KPI on the first save for this
    // AI-DB job – subsequent calls are progress-tick upserts of the same job.
    let is_new_bakta_job = state.load_bakta_job(&job_id).ok().flatten().is_none();

    state.upsert_bakta_job(&job_id, &request).map_err(|e| {
        tracing::error!("Failed to save bakta state for job {job_id}: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(e)),
        )
    })?;

    if is_new_bakta_job {
        state.record_bakta_job_started_kpi();
    }

    tracing::debug!(
        "Bakta state saved | job={job_id} | bakta_id={} | status={} | {}%",
        request.bakta_job_id,
        request.status,
        request.progress_percent,
    );

    Ok(Json(SaveBaktaJobResponse { saved: true }))
}

/// Load persisted Bakta job state.
/// Returns 404 when no Bakta job has been started for this AI-DB job.
#[utoipa::path(
    get,
    path = "/api/job/{job_id}/bakta",
    params(("job_id" = String, Path, description = "AI-DB job ID")),
    responses(
        (status = 200, description = "State found",     body = BaktaJobStateResponse),
        (status = 404, description = "No state found",  body = ErrorResponse),
        (status = 500, description = "Database error",  body = ErrorResponse),
    ),
    tag = "bakta"
)]
pub async fn get_bakta_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<BaktaJobStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.load_bakta_job(&job_id) {
        Ok(Some(stored)) => Ok(Json(BaktaJobStateResponse { state: stored })),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!(
                "No Bakta state found for job {job_id}"
            ))),
        )),
        Err(e) => {
            tracing::error!("Failed to load bakta state for job {job_id}: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e)),
            ))
        }
    }
}

/// Delete persisted Bakta job state. Idempotent – returns 200 even when no row existed.
#[utoipa::path(
    delete,
    path = "/api/job/{job_id}/bakta",
    params(("job_id" = String, Path, description = "AI-DB job ID")),
    responses(
        (status = 200, description = "State deleted (or never existed)"),
        (status = 403, description = "Not the job owner", body = ErrorResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    ),
    tag = "bakta"
)]
pub async fn delete_bakta_job(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(job_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    authorize_owner(&state, &job_id, &jar, &headers)?;

    state.delete_bakta_job(&job_id).map_err(|e| {
        tracing::error!("Failed to delete bakta state for job {job_id}: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(e)),
        )
    })?;

    Ok(StatusCode::OK)
}

// ── POST /api/job/:job_id/bakta/ingest ────────────────────────────────────────

/// Store Bakta annotation results of this job in the AI-DB annotations DB.
///
/// * Only hashes of this job's *unmatched* sequences and of its AI-DB
///   annotations DB matches (re-check) are accepted; the length must match and
///   all fields are validated. Everything else is counted as `rejected`.
/// * New hashes are inserted. Existing entries are never overwritten – only
///   empty fields are filled in (`updated`); entries that already contain
///   everything submitted are `unchanged`.
/// * Each accepted entry counts as a sighting of the hash by this job
///   (`seen_count`, at most once per job).
#[utoipa::path(
    post,
    path = "/api/job/{job_id}/bakta/ingest",
    params(("job_id" = String, Path, description = "AI-DB job ID")),
    request_body = IngestCustomAnnotationsRequest,
    responses(
        (status = 200, description = "Entries processed",  body = IngestCustomAnnotationsResponse),
        (status = 403, description = "Not the job owner",  body = ErrorResponse),
        (status = 404, description = "Job not found",      body = ErrorResponse),
        (status = 500, description = "Database error",     body = ErrorResponse),
    ),
    tag = "bakta"
)]
pub async fn ingest_bakta_results(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(job_id): Path<String>,
    Json(request): Json<IngestCustomAnnotationsRequest>,
) -> Result<Json<IngestCustomAnnotationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_owner(&state, &job_id, &jar, &headers)?;
    let total = request.entries.len();

    // 1. Which hashes may this job contribute? Its unmatched sequences and its
    //    AI-DB annotations DB matches (re-check). Bakta DB matches never.
    let allowed: HashMap<String, usize> = {
        let jobs = state.jobs();
        jobs.get(&job_id)
            .and_then(|job| job.sequences.as_ref())
            .map(|seqs| {
                seqs.iter()
                    .filter(|s| match s.annotation_source.as_deref() {
                        None | Some("aidb_db") => true,
                        _ => false,
                    })
                    .filter_map(|s| s.md5_hash.as_ref().map(|h| (h.to_lowercase(), s.length)))
                    .collect()
            })
            .unwrap_or_default()
    };

    // 2. Verify and validate every entry.
    let mut verified = Vec::with_capacity(total.min(allowed.len()));
    let mut seen: HashSet<String> = HashSet::new();
    let mut rejected = 0usize;
    for raw in &request.entries {
        let entry = match raw.normalized() {
            Ok(e) => e,
            Err(reason) => {
                tracing::debug!("Ingest job {job_id}: rejected entry ({reason})");
                rejected += 1;
                continue;
            }
        };
        match allowed.get(&entry.md5_hash) {
            Some(len) if *len == entry.length && seen.insert(entry.md5_hash.clone()) => {
                verified.push(entry)
            }
            _ => {
                tracing::debug!("Ingest job {job_id}: rejected entry (not part of this job)");
                rejected += 1;
            }
        }
    }

    tracing::info!(
        "Ingest job {job_id}: {} verified, {rejected} rejected of {total} entries",
        verified.len()
    );

    // 3. Insert new / fill gaps in existing entries, count sightings
    let stats = state
        .ingest_custom_annotations(&verified, &job_id)
        .map_err(|e| {
            tracing::error!("Ingest failed for job {job_id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e)),
            )
        })?;

    Ok(Json(IngestCustomAnnotationsResponse {
        ingested: stats.inserted,
        updated: stats.updated,
        unchanged: stats.unchanged,
        rejected,
        total,
    }))
}
