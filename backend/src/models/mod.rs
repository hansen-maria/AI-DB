//! ============================================================================
//! Data models for the AI-DB API
//!
//! Wire types (request/response bodies, query parameters) live in the shared
//! `aidb-types` crate so API clients use exactly the same definitions; they
//! are re-exported here so handlers keep importing from `crate::models`.
//! Server-internal types (hash lookup, sequence filters) stay in this module.
//! ============================================================================

pub mod sequence;

pub use aidb_types::*;
pub use sequence::{AdvancedSequenceFilter, HashLookupResult, SequenceFilter};
