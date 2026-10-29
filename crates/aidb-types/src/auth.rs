//! ============================================================================
//! Owner token of a browser session
//! ============================================================================

use serde::{Deserialize, Serialize};

/// Response of `GET` / `POST /api/auth/token`
///
/// The token is the browser's owner identity (the value of the HttpOnly
/// cookie). Sent as `Authorization: Bearer <token>` or `X-API-Key: <token>`,
/// it gives scripts and the AI-DB CLI access to the same jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AuthTokenResponse {
    /// Owner token (UUIDv4). Treat it like a password.
    pub token: String,
}
