//! ============================================================================
//! Handlers for Bakta job persistence
//!
//! Routes:
//!   POST   /api/job/{job_id}/bakta  → save_bakta_job
//!   GET    /api/job/{job_id}/bakta  → get_bakta_job
//!   DELETE /api/job/{job_id}/bakta  → delete_bakta_job
//! ============================================================================

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

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
        (status = 404, description = "Job not found",  body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse),
    ),
    tag = "bakta"
)]
pub async fn save_bakta_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Json(request): Json<SaveBaktaJobRequest>,
) -> Result<Json<SaveBaktaJobResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !state.jobs().contains_key(&job_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!("Job not found: {job_id}"))),
        ));
    }

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
        (status = 500, description = "Database error", body = ErrorResponse),
    ),
    tag = "bakta"
)]
pub async fn delete_bakta_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
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

/// Ingest Bakta annotation results into the custom annotations DB.
///
/// The frontend constructs one entry per unmatched sequence by matching the
/// original sequence (with its MD5 hash) against the Bakta JSON result features.
/// Sequences that Bakta couldn't annotate are still recorded (with null IDs)
/// so they are not re-submitted in future jobs.
#[utoipa::path(
    post,
    path = "/api/job/{job_id}/bakta/ingest",
    params(("job_id" = String, Path, description = "AI-DB job ID")),
    request_body = IngestCustomAnnotationsRequest,
    responses(
        (status = 200, description = "Entries ingested",   body = IngestCustomAnnotationsResponse),
        (status = 404, description = "Job not found",      body = ErrorResponse),
        (status = 500, description = "Database error",     body = ErrorResponse),
    ),
    tag = "bakta"
)]
pub async fn ingest_bakta_results(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Json(request): Json<IngestCustomAnnotationsRequest>,
) -> Result<Json<IngestCustomAnnotationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !state.jobs().contains_key(&job_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!("Job not found: {job_id}"))),
        ));
    }

    let total = request.entries.len();

    tracing::info!(
        "Ingesting {} Bakta entries for job {job_id} into AI-DB annotations DB",
        total
    );

    let (ingested, updated) = state
        .ingest_custom_annotations(&request.entries)
        .map_err(|e| {
            tracing::error!("Ingest failed for job {job_id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e)),
            )
        })?;

    Ok(Json(IngestCustomAnnotationsResponse {
        ingested,
        updated,
        total,
    }))
}
