//! ============================================================================
//! aidb-client – client library for the AI-DB REST API
//!
//! Scaffold. Planned modules (see CLI concept, v0.1):
//!   * `client`   – `Client::builder()`, one method per endpoint, Bearer auth
//!   * `upload`   – streamed multipart upload of (gzipped) protein FASTA
//!   * `wait`     – job polling with backoff until completed / failed / timeout
//!   * `download` – streamed result download (tsv, json, fasta, gff3)
//!
//! All request and response types come from `aidb-types`, re-exported as
//! [`types`], so client and server cannot drift apart silently.
//! ============================================================================

pub use aidb_types as types;

pub mod compat;
