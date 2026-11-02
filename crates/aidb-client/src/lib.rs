//! ============================================================================
//! aidb-client – client library for the AI-DB REST API
//!
//! ```no_run
//! # async fn demo() -> aidb_client::Result<()> {
//! let client = aidb_client::Client::builder()
//!     .server("https://ai-db.computational.bio")
//!     .build()?;
//! let health = client.health().await?;
//! println!("{} (API v{})", health.version, health.api_version);
//! # Ok(())
//! # }
//! ```
//!
//! All request and response types come from `aidb-types`, re-exported as
//! [`types`], so client and server cannot drift apart silently.
//!
//! Covers health and DB info, job submission (streamed multipart upload with
//! progress), job listing, status, waiting with backoff and streamed result
//! download.
//! ============================================================================

pub use aidb_types as types;

mod client;
pub mod compat;
mod error;

pub use client::{
    normalize_server_url, Client, ClientBuilder, ResultFormat, WaitOptions, DEFAULT_SERVER,
    DEFAULT_TIMEOUT, MAX_UPLOAD_BYTES,
};
pub use error::{Error, Result};
