//! ============================================================================
//! API request handlers
//! ============================================================================

pub mod bakta;
pub mod download;
pub mod health;
pub mod jobs;
pub mod kpi;
pub mod psos;
pub mod stats;

pub use bakta::{delete_bakta_job, get_bakta_job, ingest_bakta_results, save_bakta_job};
pub use download::download_job;
pub use health::{db_info, health_check};
pub use jobs::{
    bulk_delete_jobs, create_job, delete_job, get_job, get_sequence, list_jobs, rename_job,
    retry_job,
};
pub use kpi::get_kpi_overview;
pub use psos::{delete_psos_results, get_psos_results, save_psos_results};
pub use stats::{export_job_stats, get_job_stats};
