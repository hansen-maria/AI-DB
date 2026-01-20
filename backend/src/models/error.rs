//! ============================================================================
//! Error types and responses
//! ============================================================================

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
