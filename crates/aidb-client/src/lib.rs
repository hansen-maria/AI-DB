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
//! ============================================================================

pub use aidb_types as types;

mod client;
pub mod compat;
mod error;

pub use client::{normalize_server_url, Client, ClientBuilder, DEFAULT_SERVER, DEFAULT_TIMEOUT};
pub use error::{Error, Result};
