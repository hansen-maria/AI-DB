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
    BaktaJobStateResponse, ErrorResponse, SaveBaktaJobRequest, SaveBaktaJobResponse,
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

    state.upsert_bakta_job(&job_id, &request).map_err(|e| {
        tracing::error!("Failed to save bakta state for job {job_id}: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(e)),
        )
    })?;

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
