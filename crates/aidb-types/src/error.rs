//! ============================================================================
//! Error types and responses
//! ============================================================================

use serde::{Deserialize, Serialize};

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    /// Error description
    pub detail: String,
}

impl ErrorResponse {
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}
