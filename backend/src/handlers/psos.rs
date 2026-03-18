//! ============================================================================
//! Psos results handlers
//! ============================================================================

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::models::{
    ErrorResponse, PsosResultsResponse, SavePsosResultsRequest, SavePsosResultsResponse,
};
use crate::state::AppState;

/// Save Psos results for a job
///
/// Stores Psos analysis results in the database, associated with a job.
/// Results persist as long as the job exists (30 days).
#[utoipa::path(
    post,
    path = "/api/job/{job_id}/psos",
    params(
        ("job_id" = String, Path, description = "Job ID")
    ),
    request_body = SavePsosResultsRequest,
    responses(
        (status = 200, description = "Results saved successfully", body = SavePsosResultsResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "psos"
)]
pub async fn save_psos_results(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Json(request): Json<SavePsosResultsRequest>,
) -> Result<Json<SavePsosResultsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify job exists
    if !state.jobs().contains_key(&job_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!("Job not found: {}", job_id))),
        ));
    }

    // Save results
    let total_count = state
        .save_psos_results(&job_id, &request.results)
        .map_err(|e| {
            tracing::error!("Failed to save psos results: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e)),
            )
        })?;

    tracing::info!(
        "Saved {} psos results for job {}, total: {}",
        request.results.len(),
        job_id,
        total_count
    );

    Ok(Json(SavePsosResultsResponse {
        saved_count: request.results.len(),
        total_count,
    }))
}

/// Get Psos results for a job
///
/// Retrieves all stored Psos analysis results for a job.
#[utoipa::path(
    get,
    path = "/api/job/{job_id}/psos",
    params(
        ("job_id" = String, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Psos results", body = PsosResultsResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "psos"
)]
pub async fn get_psos_results(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<PsosResultsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify job exists
    if !state.jobs().contains_key(&job_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!("Job not found: {}", job_id))),
        ));
    }

    // Load results
    let results = state.load_psos_results(&job_id).map_err(|e| {
        tracing::error!("Failed to load psos results: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(e)),
        )
    })?;

    Ok(Json(PsosResultsResponse {
        job_id,
        total_count: results.len(),
        results,
    }))
}

/// Delete all Psos results for a job
///
/// Removes all stored Psos analysis results for a job.
#[utoipa::path(
    delete,
    path = "/api/job/{job_id}/psos",
    params(
        ("job_id" = String, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Results deleted"),
        (status = 404, description = "Job not found", body = ErrorResponse)
    ),
    tag = "psos"
)]
pub async fn delete_psos_results(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Verify job exists
    if !state.jobs().contains_key(&job_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!("Job not found: {}", job_id))),
        ));
    }

    // Delete results
    if let Some(conn) = state.open_jobs_db() {
        if let Err(e) = crate::storage::delete_psos_results(&conn, &job_id) {
            tracing::error!("Failed to delete psos results: {}", e);
        }
    }

    tracing::info!("Deleted psos results for job {}", job_id);
    Ok(StatusCode::OK)
}
