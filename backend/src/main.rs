//! AI-DB REST API Backend
//! Hash-Based Annotation Service for Microbial Sequencing Data

use axum::extract::DefaultBodyLimit;
use axum::http::{header, Method};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use md5::{Digest, Md5};
use parking_lot::RwLock;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufRead, BufReader, Write},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};
use tempfile::{Builder as TempFileBuilder, NamedTempFile};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

// ============================================================================
// Data Models
// ============================================================================

/// Status of an annotation job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Information about the sequence
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SequenceInfo {
    /// Sequence identifier
    pub id: String,
    /// MD5-Hash
    pub md5_hash: String,
    /// Length in bp / aa
    pub length: usize,
    /// The actual sequence (amino acids or nucleotides)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    /// Annotation description (if found)
    pub annotation: Option<String>,
    /// Source of annotation
    pub annotation_source: Option<String>,
    /// UniParc ID (if found)
    pub uniparc_id: Option<String>,
    /// NCBI NRP ID (if found)
    pub ncbi_nrp_id: Option<String>,
    /// UniRef100 ID (if found)
    pub uniref100_id: Option<String>,
}

/// Job details and results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobResponse {
    /// Unique job ID (UUID)
    pub job_id: String,
    /// Current status of the job
    pub status: JobStatus,
    /// Timestamp of creation
    pub created_at: DateTime<Utc>,
    /// Last updated at
    pub updated_at: DateTime<Utc>,
    /// Name of the uploaded file
    pub filename: Option<String>,
    /// Number of sequences
    pub sequence_count: usize,
    /// Number of processed sequences
    pub processed_count: usize,
    /// Number of hash matches
    pub hash_matches: usize,
    /// Number of alignment matches
    pub alignment_matches: usize,
    /// Details about the sequences
    pub sequences: Option<Vec<SequenceInfo>>,
    /// Error messages (if an error occurred)
    pub error_message: Option<String>,
    /// Owner ID (from cookie, not serialized to client)
    #[serde(skip_serializing)]
    pub owner_id: Option<String>,
}

/// Response after job creation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobCreateResponse {
    /// Unique job ID (UUID)
    pub job_id: String,
    /// Initial status (pending)
    pub status: JobStatus,
    /// Confirmation message
    pub message: String,
    /// Number of found sequences
    pub sequence_count: usize,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Error description
    pub detail: String,
}

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationInfo {
    /// Current page (1-indexed)
    pub page: usize,
    /// Items per page
    pub per_page: usize,
    /// Total number of items
    pub total_items: usize,
    /// Total number of pages
    pub total_pages: usize,
    /// Has next page
    pub has_next: bool,
    /// Has previous page
    pub has_prev: bool,
}

impl PaginationInfo {
    pub fn new(page: usize, per_page: usize, total_items: usize) -> Self {
        let total_pages = (total_items + per_page - 1) / per_page; // Ceiling division
        Self {
            page,
            per_page,
            total_items,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

/// Paginated response for job list
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedJobsResponse {
    /// List of jobs (without sequences for performance)
    pub jobs: Vec<JobSummary>,
    /// Pagination information
    pub pagination: PaginationInfo,
}

/// Job summary (without sequences for list view)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobSummary {
    /// Unique job ID
    pub job_id: String,
    /// Current job status
    pub status: JobStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Uploaded filename
    pub filename: Option<String>,
    /// Total sequence count
    pub sequence_count: usize,
    /// Processed sequence count
    pub processed_count: usize,
    /// Hash match count
    pub hash_matches: usize,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

impl From<&JobResponse> for JobSummary {
    fn from(job: &JobResponse) -> Self {
        Self {
            job_id: job.job_id.clone(),
            status: job.status.clone(),
            created_at: job.created_at,
            updated_at: job.updated_at,
            filename: job.filename.clone(),
            sequence_count: job.sequence_count,
            processed_count: job.processed_count,
            hash_matches: job.hash_matches,
            error_message: job.error_message.clone(),
        }
    }
}

/// Paginated job details response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedJobResponse {
    /// Job ID
    pub job_id: String,
    /// Current job status
    pub status: JobStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Uploaded filename
    pub filename: Option<String>,
    /// Total sequence count (unfiltered)
    pub sequence_count: usize,
    /// Processed sequence count
    pub processed_count: usize,
    /// Hash match count
    pub hash_matches: usize,
    /// Alignment match count
    pub alignment_matches: usize,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Paginated sequences (filtered)
    pub sequences: Vec<SequenceInfo>,
    /// Pagination information for sequences
    pub pagination: PaginationInfo,
    /// Current filter applied ("all", "hash_match", "alignment", "none")
    pub filter: String,
    /// Number of sequences matching the current filter
    pub filtered_count: usize,
}

/// Query parameters for job list
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListJobsQuery {
    /// Page number (1-indexed, default: 1)
    pub page: Option<usize>,
    /// Items per page (default: 20, max: 100)
    pub per_page: Option<usize>,
}

/// Query parameters for job details
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetJobQuery {
    /// Page number for sequences (1-indexed, default: 1)
    pub page: Option<usize>,
    /// Sequences per page (default: 20, max: 100)
    pub per_page: Option<usize>,
    /// Filter by annotation source: "all", "hash_match", "alignment", "none" (default: "all")
    pub filter: Option<String>,
}

/// Filter type for sequences
#[derive(Debug, Clone, PartialEq, ToSchema)]
pub enum SequenceFilter {
    All,
    HashMatch,
    Alignment,
    NoMatch,
}

impl SequenceFilter {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "hash_match" | "hash" => SequenceFilter::HashMatch,
            "alignment" => SequenceFilter::Alignment,
            "none" | "no_match" => SequenceFilter::NoMatch,
            _ => SequenceFilter::All,
        }
    }

    fn matches(&self, seq: &SequenceInfo) -> bool {
        match self {
            SequenceFilter::All => true,
            SequenceFilter::HashMatch => seq.annotation.as_deref() == Some("hash_match"),
            SequenceFilter::Alignment => seq.annotation.as_deref() == Some("alignment"),
            SequenceFilter::NoMatch => seq.annotation.is_none(),
        }
    }
}

/// Bakta Hash Lookup Result
#[derive(Debug, Clone, ToSchema)]
pub struct HashLookupResult {
    pub found: bool,
    pub db_length: Option<i64>,
    pub uniparc_id: Option<String>,
    pub ncbi_nrp_id: Option<String>,
    pub uniref100_id: Option<String>,
}

/// Service Health Check Response
#[derive(Serialize, ToSchema)]
pub struct HealthCheckResponse {
    pub status: String,
    pub service: String,
    pub bakta_db: BaktaDbHealth,
}

/// Bakta Database Health Check Response
#[derive(Serialize, ToSchema)]
pub struct BaktaDbHealth {
    /// Database connection status
    /// Possible values: connected, error, not_found, not_configured
    pub status: String,

    /// Filesystem path to the Bakta database
    pub path: Option<String>,
}

/// Database Info Response
#[derive(Serialize, ToSchema)]
pub struct DbInfoResponse {
    /// Indicates whether the database is accessible
    pub available: bool,

    /// Filesystem path to the database
    pub path: Option<String>,

    /// Number of entries in the `ups` table
    pub ups_entries: Option<i64>,

    /// Bakta database version (if available)
    pub version: Option<String>,

    /// Error message if database is not accessible
    pub error: Option<String>,
}

// ============================================================================
// Application State
// ============================================================================

#[derive(Clone)]
pub struct AppState {
    jobs: Arc<RwLock<HashMap<String, JobResponse>>>,
    bakta_db_path: Option<PathBuf>,
}

impl AppState {
    pub fn new() -> Self {
        // Get database path from environment variable
        let bakta_db_path = env::var("BAKTA_DB")
            .ok()
            .map(|p| PathBuf::from(p).join("bakta.db"))
            .or_else(|| {
                // Fallback paths
                let fallback_paths = [
                    PathBuf::from("/bakta-db/bakta.db"),
                    PathBuf::from("/opt/bakta-db/bakta.db"),
                    PathBuf::from("/mnt/bakta-db/db/bakta.db"),
                ];
                fallback_paths.into_iter().find(|p| p.exists())
            });

        if let Some(ref path) = bakta_db_path {
            if path.exists() {
                tracing::info!("Bakta database found at: {:?}", path);
            } else {
                tracing::warn!(
                    "Bakta database path configured but file not found: {:?}",
                    path
                );
            }
        } else {
            tracing::warn!("No Bakta database configured. Set BAKTA_DB environment variable.");
            tracing::warn!("Hash lookups will return no matches.");
        }

        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            bakta_db_path,
        }
    }

    /// Opens a read-only connection to the Bakta database
    fn open_db_connection(&self) -> Option<Connection> {
        self.bakta_db_path.as_ref().and_then(|path| {
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| {
                    tracing::error!("Failed to open Bakta database: {}", e);
                    e
                })
                .ok()
        })
    }
}

// ============================================================================
// Constants
// ============================================================================

const OWNER_COOKIE_NAME: &str = "ai_db_user";
const COOKIE_MAX_AGE_DAYS: i64 = 365; // 1 year

// ============================================================================
// Helper Functions
// ============================================================================

/// Generates or retrieves an owner identifier stored in a cookie.
///
/// This function checks if a cookie matching the owner's identifier exists
/// within the provided `CookieJar`. If it exists, the function retrieves
/// the value of the cookie and returns it along with the unchanged `CookieJar`.
/// If the owner's cookie does not exist, a new unique identifier is generated,
/// stored in a cookie, and added to the `CookieJar`.
///
/// # Parameters
/// - `jar`: A `CookieJar` containing cookies for the client request. It is
///   used to check for an existing owner cookie or to store a new one.
///
/// # Returns
/// A tuple containing:
/// - `String`: The owner identifier, either retrieved from an existing cookie
///   or newly generated.
/// - `CookieJar`: The updated `CookieJar` that contains the new owner cookie
///   if one was created.
///
/// # Behavior
/// - If the cookie with the name `OWNER_COOKIE_NAME` exists, its value is
///   extracted and returned.
/// - If the cookie does not exist:
///   - A new identifier is generated using `Uuid::new_v4`.
///   - The new identifier is stored in a cookie with the following properties:
///     - Path set to the root directory (`"/"`).
///     - HTTP-only flag is set (`http_only(true)`).
///     - SameSite protection is set to `Lax`.
///     - Maximum age of the cookie is defined by `COOKIE_MAX_AGE_DAYS`.
///   - The new cookie is added to the `CookieJar`.
///
/// # Example
/// ```rust
/// let jar = CookieJar::new();
/// let (owner_id, updated_jar) = get_or_create_owner(jar);
/// println!("Owner ID: {}", owner_id);
/// ```
fn get_or_create_owner(jar: CookieJar) -> (String, CookieJar) {
    if let Some(cookie) = jar.get(OWNER_COOKIE_NAME) {
        (cookie.value().to_string(), jar)
    } else {
        let new_id = Uuid::new_v4().to_string();
        let cookie = Cookie::build((OWNER_COOKIE_NAME, new_id.clone()))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .max_age(time::Duration::days(COOKIE_MAX_AGE_DAYS))
            .build();
        (new_id, jar.add(cookie))
    }
}

// ============================================================================
// FASTA Parsing (Streaming)
// ============================================================================

/// Batch size for processing sequences
const BATCH_SIZE: usize = 1000;

/// Maximum sequence length to prevent memory issues
const MAX_SEQUENCE_LENGTH: usize = 5_000_000; // 5 MB

/// Streaming FASTA parser that yields sequences one at a time
struct FastaIterator<R: BufRead> {
    reader: R,
    current_header: Option<String>,
    current_sequence: String,
    line_buffer: String,
    finished: bool,
}

impl<R: BufRead> FastaIterator<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            current_header: None,
            current_sequence: String::new(),
            line_buffer: String::new(),
            finished: false,
        }
    }
}

impl<R: BufRead> Iterator for FastaIterator<R> {
    type Item = (String, String);

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            self.line_buffer.clear();
            match self.reader.read_line(&mut self.line_buffer) {
                Ok(0) => {
                    // EOF reached
                    self.finished = true;
                    if let Some(header) = self.current_header.take() {
                        if !self.current_sequence.is_empty() {
                            let seq = std::mem::take(&mut self.current_sequence);
                            return Some((header, seq));
                        }
                    }
                    return None;
                }
                Ok(_) => {
                    let line = self.line_buffer.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if line.starts_with('>') {
                        // New sequence header
                        let new_header = line[1..]
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .to_string();

                        if let Some(header) = self.current_header.take() {
                            if !self.current_sequence.is_empty() {
                                let seq = std::mem::take(&mut self.current_sequence);
                                self.current_header = Some(new_header);
                                return Some((header, seq));
                            }
                        }
                        self.current_header = Some(new_header);
                        self.current_sequence.clear();
                    } else {
                        // Sequence line - append (with length limit)
                        if self.current_sequence.len() < MAX_SEQUENCE_LENGTH {
                            self.current_sequence.push_str(&line.to_uppercase());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading FASTA: {}", e);
                    self.finished = true;
                    return None;
                }
            }
        }
    }
}

/// Computes the MD5 hash of an input string and returns both its hexadecimal
/// representation and raw byte components.
///
/// # Arguments
/// - `sequence`: A string slice (`&str`) representing the input data to hash.
///
/// # Returns
/// - A tuple containing:
///   1. `String`: The computed MD5 hash represented as a hexadecimal string.
///   2. `Vec<u8>`: The computed MD5 hash as a vector of raw bytes.
///
/// # Example
/// ```
/// use md5::{Md5, Digest};
/// use hex;
///
/// let input = "example";
/// let (hash_hex, hash_bytes) = compute_md5(input);
///
/// assert_eq!(hash_hex, "1a79a4d60de6718e8e5b326e338ae533");
/// assert_eq!(hash_bytes, vec![0x1a, 0x79, 0xa4, 0xd6, 0x0d, 0xe6, 0x71, 0x8e, 0x8e, 0x5b, 0x32, 0x6e, 0x33, 0x8a, 0xe5, 0x33]);
/// ```
fn compute_md5(sequence: &str) -> (String, Vec<u8>) {
    let mut hasher = Md5::new();
    hasher.update(sequence.as_bytes());
    let hash_bytes = hasher.finalize();
    let hash_hex = hex::encode(&hash_bytes);
    (hash_hex, hash_bytes.to_vec())
}

/// Looks up a hash in the `ups` table of the database using the provided connection.
///
/// This function queries the database for a hash (stored as a BLOB) and retrieves its associated
/// metadata, such as sequence length, `uniparc_id`, `ncbi_nrp_id`, and `uniref100_id`. If the hash
/// is not found or the query fails, appropriate fallback values are returned.
///
/// # Arguments
///
/// * `conn` - A reference to a `rusqlite::Connection` object representing the database connection.
/// * `hash_bytes` - A byte slice representing the hash value to look up.
/// * `seq_length` - The length of the input sequence corresponding to the hash. This is used as an
///   optional sanity check to ensure the retrieved length matches.
///
/// # Returns
///
/// A `HashLookupResult` struct containing:
/// * `found` - A boolean indicating if the hash was successfully found in the database.
/// * `db_length` - The length of the sequence stored in the database (if present).
/// * `uniparc_id` - The associated UniParc ID (if present).
/// * `ncbi_nrp_id` - The associated NCBI NRP ID (if present).
/// * `uniref100_id` - The associated UniRef100 ID (if present).
///
/// # Behavior
///
/// * If the hash is found and the query completes successfully:
///   - The `found` field is set to `true`.
///   - Other fields are populated with the retrieved values from the database.
///   - If the retrieved length differs from `seq_length`, a debug log is emitted but the result is still returned.
/// * If the hash is not found:
///   - The `found` field is set to `false`.
///   - All other fields are set to `None`.
/// * If a database query error occurs:
///   - An error is logged.
///   - The `found` field is set to `false`.
///   - All other fields are set to `None`.
///
/// # Example
///
/// ```rust
/// use rusqlite::{Connection, OpenFlags};
///
/// let conn = Connection::open_in_memory().expect("Failed to create in-memory SQLite database");
/// // Assume the `ups` table is set up in the database.
///
/// let hash_bytes = b"example_hash";
/// let seq_length = 100;
///
/// let result = lookup_hash_in_bakta(&conn, hash_bytes, seq_length);
///
/// if result.found {
///     println!("Hash found with UniParc ID: {:?}", result.uniparc_id);
/// } else {
///     println!("Hash not found in the database.");
/// }
/// ```
///
/// # Errors
///
/// * Returns an error message in debug logs if the database query fails.
///
/// # Notes
///
/// This function assumes the existence of an `ups` table with the following schema:
/// ```sql
/// CREATE TABLE ups (
///     hash BLOB PRIMARY KEY,
///     length INTEGER,
///     uniparc_id TEXT,
///     ncbi_nrp_id TEXT,
///     uniref100_id TEXT
/// );
/// ```
/// Ensure this table is correctly set up before using the function.
fn lookup_hash_in_bakta(
    conn: &Connection,
    hash_bytes: &[u8],
    seq_length: usize,
) -> HashLookupResult {
    // Query the ups table - hash is stored as BLOB
    let query = "SELECT length, uniparc_id, ncbi_nrp_id, uniref100_id FROM ups WHERE hash = ?";

    match conn.query_row(query, [hash_bytes], |row| {
        Ok(HashLookupResult {
            found: true,
            db_length: row.get(0).ok(),
            uniparc_id: row.get(1).ok(),
            ncbi_nrp_id: row.get(2).ok(),
            uniref100_id: row.get(3).ok(),
        })
    }) {
        Ok(result) => {
            // Verify length matches (optional sanity check)
            if let Some(db_len) = result.db_length {
                if db_len as usize != seq_length {
                    tracing::debug!(
                        "Hash found but length mismatch: DB={}, Query={}",
                        db_len,
                        seq_length
                    );
                }
            }
            result
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            // No match found
            HashLookupResult {
                found: false,
                db_length: None,
                uniparc_id: None,
                ncbi_nrp_id: None,
                uniref100_id: None,
            }
        }
        Err(e) => {
            tracing::error!("Database query error: {}", e);
            HashLookupResult {
                found: false,
                db_length: None,
                uniparc_id: None,
                ncbi_nrp_id: None,
                uniref100_id: None,
            }
        }
    }
}

/// Formats an annotation string based on the given `HashLookupResult`.
///
/// This function takes a reference to a `HashLookupResult` and generates a formatted
/// annotation string containing identifiers if they are available. If the `HashLookupResult`
/// indicates that no match was found (`found` is `false`), the function returns `None`.
/// Otherwise, it builds a string containing relevant IDs (e.g., UniRef100, UniParc, NCBI)
/// and combines them using a " | " delimiter.
///
/// # Parameters
/// - `result`: A reference to a `HashLookupResult` structure containing the lookup result.
///
/// # Returns
/// - `Some(String)` if the `HashLookupResult` indicates a match (`found` is `true`) with
///   one or more available IDs.
/// - `None` if `found` is `false`.
///
/// # Behavior
/// - If no specific IDs are found but the `HashLookupResult` indicates a match, the function
///   returns a generic string: `"Known protein (hash match)"`.
/// - The formatting for each ID included in the annotation:
///   * `"UniRef100:<id>"` for the UniRef100 ID
///   * `"UniParc:<id>"` for the UniParc ID
///   * `"NCBI:<id>"` for the NCBI ID
///
/// # Examples
/// ```
/// let result = HashLookupResult {
///     found: true,
///     uniref100_id: Some("P12345".to_string()),
///     uniparc_id: Some("UPI00001".to_string()),
///     ncbi_nrp_id: None,
/// };
///
/// let annotation = format_annotation(&result);
/// assert_eq!(annotation, Some("UniRef100:P12345 | UniParc:UPI00001".to_string()));
/// ```
///
/// ```
/// let result = HashLookupResult {
///     found: false,
///     uniref100_id: None,
///     uniparc_id: None,
///     ncbi_nrp_id: None,
/// };
///
/// let annotation = format_annotation(&result);
/// assert_eq!(annotation, None);
/// ```
///
/// ```
/// let result = HashLookupResult {
///     found: true,
///     uniref100_id: None,
///     uniparc_id: None,
///     ncbi_nrp_id: None,
/// };
///
/// let annotation = format_annotation(&result);
/// assert_eq!(annotation, Some("Known protein (hash match)".to_string()));
/// ```
fn format_annotation(result: &HashLookupResult) -> Option<String> {
    if !result.found {
        return None;
    }

    // Build annotation from available IDs
    let mut parts = Vec::new();

    if let Some(ref id) = result.uniref100_id {
        parts.push(format!("UniRef100:{}", id));
    }
    if let Some(ref id) = result.uniparc_id {
        parts.push(format!("UniParc:{}", id));
    }
    if let Some(ref id) = result.ncbi_nrp_id {
        parts.push(format!("NCBI:{}", id));
    }

    if parts.is_empty() {
        Some("Known protein (hash match)".to_string())
    } else {
        Some(parts.join(" | "))
    }
}

/// Processes a job, reading sequences from a file, optionally decompressing gzip files, performing MD5 hashing, and optionally looking up sequences in a database.
///
/// # Parameters
/// - `state`: A shared application state (`AppState`) containing job metadata and configurations.
/// - `job_id`: A string slice representing the unique identifier of the job.
/// - `file_path`: A `Path` reference specifying the location of the file containing sequences to process.
/// - `is_gzip`: A boolean indicating whether the file is in gzip-compressed format.
///
/// # Description
/// 1. The job status is set to `Processing`, and the job's metadata (such as timestamp) in `AppState` is updated accordingly.
/// 2. If a database connection can be established based on the shared application state, it logs a message indicating database support; otherwise, it proceeds without database lookup.
/// 3. The input file is opened for reading. If file reading fails, the job is marked as `Failed`, and an error message is logged and stored.
/// 4. The file is processed for sequences using a streaming approach. If `is_gzip` is true, the file is decompressed using `GzDecoder`. Sequences are iterated through using a `FastaIterator`, avoiding preallocation to handle larger files.
/// 5. For each sequence:
///    - The MD5 hash is computed, and sequence length is determined.
///    - If a database connection is available, a lookup is performed in the database using the hash value. Annotations are added to the result if a match is found.
///    - Results for sequences are stored (up to `MAX_RESULTS`) to avoid memory overflows.
/// 6. During processing:
///    - Progress is updated every `BATCH_SIZE` sequences by modifying the job metadata stored in `AppState`.
/// 7. After processing all sequences:
///    - Memory for unused space in the results vector is released.
///    - Final job statuses are set: either `Completed` if any sequences are processed, or `Failed` if none are successfully processed.
///    - Additional warnings may be added, such as indication of truncated results if the sequence count exceeds `MAX_RESULTS`, or no valid sequences found.
/// 8. Logs detailing the job's completion status, the number of sequences processed, and hash matches are created.
///
/// # Constants
/// - `MAX_RESULTS`: Maximum number of results (`1_000_000`) that can be stored to prevent excessive memory usage.
/// - `BATCH_SIZE`: Used to determine how often to report progress during the process.
///
/// # Behavior
/// The process is robust, handling errors related to:
/// - File access (e.g., file not found, read errors).
/// - Null or missing database connections.
///
/// # Thread Safety
/// The method uses a write lock (`AppState.jobs.write`) to safely update shared job states across threads.
///
/// # Error Handling
/// - If the file cannot be read, the job is marked `Failed` with a corresponding error message.
/// - If the sequence count exceeds `MAX_RESULTS`, warnings about truncated results are added to the job metadata.
///
/// # Examples
/// ```rust
/// // Example usage:
/// let state = AppState::new();
/// let job_id = "job123";
/// let file_path = Path::new("/path/to/file.fasta");
/// let is_gzip = false;
///
/// process_job_from_file(&state, job_id, file_path, is_gzip);
/// ```
fn process_job_from_file(
    state: &AppState,
    job_id: &str,
    file_path: &std::path::Path,
    is_gzip: bool,
) {
    // Set status to processing
    {
        let mut jobs = state.jobs.write();
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Processing;
            job.updated_at = Utc::now();
        }
    }

    // Try to open database connection
    let db_conn = state.open_db_connection();
    let db_available = db_conn.is_some();

    if db_available {
        tracing::info!("Processing job {} with Bakta database lookup", job_id);
    } else {
        tracing::warn!("Processing job {} without database", job_id);
    }

    // Open file for streaming
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to open temp file for job {}: {}", job_id, e);
            let mut jobs = state.jobs.write();
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = JobStatus::Failed;
                job.error_message = Some(format!("Failed to read uploaded file: {}", e));
                job.updated_at = Utc::now();
            }
            return;
        }
    };

    // Create streaming reader (with gzip support)
    let reader: Box<dyn BufRead + Send> = if is_gzip {
        Box::new(BufReader::with_capacity(64 * 1024, GzDecoder::new(file)))
    } else {
        Box::new(BufReader::with_capacity(64 * 1024, file))
    };

    let fasta_iter = FastaIterator::new(reader);

    // Process without pre-allocation (we don't know the count)
    let mut sequence_infos = Vec::new();
    let mut hash_matches = 0;
    let alignment_matches = 0;
    let mut processed_count = 0;
    let mut batch_count = 0;

    // Maximum results to store (prevent OOM)
    const MAX_RESULTS: usize = 1_000_000;

    // Process sequences one at a time (streaming)
    for (header, seq) in fasta_iter {
        let (hash_hex, hash_bytes) = compute_md5(&seq);
        let seq_length = seq.len();

        // Perform database lookup if available
        let lookup_result = if let Some(ref conn) = db_conn {
            lookup_hash_in_bakta(conn, &hash_bytes, seq_length)
        } else {
            HashLookupResult {
                found: false,
                db_length: None,
                uniparc_id: None,
                ncbi_nrp_id: None,
                uniref100_id: None,
            }
        };

        let (annotation, annotation_source) = if lookup_result.found {
            hash_matches += 1;
            (
                format_annotation(&lookup_result),
                Some("hash_match".to_string()),
            )
        } else {
            (None, None)
        };

        // Only store results if we haven't hit the limit
        if sequence_infos.len() < MAX_RESULTS {
            sequence_infos.push(SequenceInfo {
                id: header,
                md5_hash: hash_hex,
                length: seq_length,
                sequence: Some(seq),
                annotation,
                annotation_source,
                uniparc_id: lookup_result.uniparc_id,
                ncbi_nrp_id: lookup_result.ncbi_nrp_id,
                uniref100_id: lookup_result.uniref100_id,
            });
        }

        processed_count += 1;
        batch_count += 1;

        // Update progress every BATCH_SIZE sequences
        if batch_count >= BATCH_SIZE {
            batch_count = 0;
            {
                let mut jobs = state.jobs.write();
                if let Some(job) = jobs.get_mut(job_id) {
                    job.sequence_count = processed_count; // Update count as we go
                    job.processed_count = processed_count;
                    job.hash_matches = hash_matches;
                    job.updated_at = Utc::now();
                }
            }
            tracing::debug!(
                "Job {} progress: {} sequences processed",
                job_id,
                processed_count
            );
        }
    }

    // Shrink to fit to release unused memory
    sequence_infos.shrink_to_fit();

    // Final update with results
    {
        let mut jobs = state.jobs.write();
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = if processed_count > 0 {
                JobStatus::Completed
            } else {
                JobStatus::Failed
            };
            job.updated_at = Utc::now();
            job.sequence_count = processed_count;
            job.processed_count = processed_count;
            job.hash_matches = hash_matches;
            job.alignment_matches = alignment_matches;

            // Add warning if results were truncated
            if processed_count > MAX_RESULTS {
                job.error_message = Some(format!(
                    "Results truncated: showing first {} of {} sequences",
                    MAX_RESULTS, processed_count
                ));
            } else if processed_count == 0 {
                job.error_message = Some("No valid sequences found in input.".to_string());
            }

            job.sequences = Some(sequence_infos);
        }
    }

    tracing::info!(
        "Job {} completed: {} sequences processed, {} hash matches",
        job_id,
        processed_count,
        hash_matches
    );
}

// ============================================================================
// API Handlers
// ============================================================================

/// Default items per page
const DEFAULT_PER_PAGE: usize = 20;
/// Maximum items per page
const MAX_PER_PAGE: usize = 100;

/// Retrieves information about a specific job by its unique `job_id`.
///
/// # Endpoint
/// `GET /api/job/{job_id}`
///
/// This endpoint provides detailed information about a job, including filtered and paginated sequences.
///
/// # Path Parameters
/// - `job_id` (*String*, required): A unique job identifier (UUID).
///
/// # Query Parameters
/// - `page` (*Option<usize>*, optional): Specifies the sequence page to retrieve. Defaults to `1` (indexed from 1).
/// - `per_page` (*Option<usize>*, optional): The number of sequences to return per page. Defaults to `20` and cannot exceed `100`.
/// - `filter` (*Option<String>*, optional): Specifies the type of sequences to include in the response. Options include:
///   - `all`: Include all sequences.
///   - `hash_match`: Include only sequences with hash matches.
///   - `alignment`: Include sequences with alignment matches.
///   - `none`: Include sequences without matches. Defaults to `all`.
///
/// # Responses
///
/// ## Success (200)
/// Returns job details and filtered/paginated sequences.
///
/// Example Response Body:
/// ```json
/// {
///   "job_id": "string",
///   "status": "string",
///   "created_at": "ISO8601_timestamp",
///   "updated_at": "ISO8601_timestamp",
///   "filename": "string",
///   "sequence_count": 100,
///   "processed_count": 80,
///   "hash_matches": 50,
///   "alignment_matches": 30,
///   "error_message": null,
///   "sequences": [
///     {
///       "sequence_id": "string",
///       "hash": "string",
///       "alignment_score": 95.0,
///       "metadata": {}
///     }
///   ],
///   "pagination": {
///     "page": 1,
///     "per_page": 20,
///     "total_pages": 5,
///     "total_items": 100
///   },
///   "filter": "all",
///   "filtered_count": 100
/// }
/// ```
///
/// ## Error (404)
/// If the requested job does not exist, a `404 Not Found` response is returned with an error message.
///
/// Example Error Response:
/// ```json
/// {
///   "detail": "Job with ID 'job_id' not found"
/// }
/// ```
///
/// # Notes
/// - Filters are applied to the sequences within the job.
/// - Results are paginated after filtering is performed.
/// - If no sequences match the filter, an empty list is returned.
///
/// # Implementation Details
/// - Default values are used for missing query parameters.
/// - Pagination is calculated based on the filtered sequence count.
/// - The response includes metadata about pagination, job status, and any applicable filtering.
///
#[utoipa::path(
    get,
    path = "/api/job/{job_id}",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)"),
        ("page" = Option<usize>, Query, description = "Sequence page (indexed from 1, default: 1)"),
        ("per_page" = Option<usize>, Query, description = "Sequences per page (default: 20, max: 100)"),
        ("filter" = Option<String>, Query, description = "Filter: all, hash_match, alignment, none")
    ),
    responses(
        (status = 200, description = "Job found", body = JobResponse),
        (status = 404, description = "Job not found", body = ErrorResponse)
    )
)]
async fn get_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Query(query): Query<GetJobQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .min(MAX_PER_PAGE)
        .max(1);
    let filter = query
        .filter
        .as_deref()
        .map(SequenceFilter::from_str)
        .unwrap_or(SequenceFilter::All);
    let filter_str = match &filter {
        SequenceFilter::All => "all",
        SequenceFilter::HashMatch => "hash_match",
        SequenceFilter::Alignment => "alignment",
        SequenceFilter::NoMatch => "none",
    }
    .to_string();

    let jobs = state.jobs.read();

    match jobs.get(&job_id) {
        Some(job) => {
            // Apply filter to sequences
            let filtered_sequences: Vec<&SequenceInfo> = job
                .sequences
                .as_ref()
                .map(|seqs| {
                    seqs.iter()
                        .filter(|s| match filter {
                            SequenceFilter::All => true,
                            SequenceFilter::HashMatch => {
                                s.annotation_source.as_deref() == Some("hash_match")
                            }
                            SequenceFilter::Alignment => {
                                s.annotation_source.as_deref() == Some("alignment")
                            }
                            SequenceFilter::NoMatch => s.annotation_source.is_none(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let filtered_count = filtered_sequences.len();

            // Calculate pagination based on filtered results
            let pagination = PaginationInfo::new(page, per_page, filtered_count);

            // Get paginated sequences from filtered results
            let paginated_sequences: Vec<SequenceInfo> = {
                let start = (page - 1) * per_page;
                let end = (start + per_page).min(filtered_count);
                if start < filtered_count {
                    filtered_sequences[start..end]
                        .iter()
                        .map(|s| (*s).clone())
                        .collect()
                } else {
                    Vec::new()
                }
            };

            let response = PaginatedJobResponse {
                job_id: job.job_id.clone(),
                status: job.status.clone(),
                created_at: job.created_at,
                updated_at: job.updated_at,
                filename: job.filename.clone(),
                sequence_count: job.sequence_count,
                processed_count: job.processed_count,
                hash_matches: job.hash_matches,
                alignment_matches: job.alignment_matches,
                error_message: job.error_message.clone(),
                sequences: paginated_sequences,
                pagination,
                filter: filter_str,
                filtered_count,
            };

            (StatusCode::OK, Json(response)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                detail: format!("Job with ID '{}' not found", job_id),
            }),
        )
            .into_response(),
    }
}

/// Maximum upload size (unlimited when using temp files but set to a reasonable limit)
const MAX_UPLOAD_SIZE: usize = 100 * 1024 * 1024; // 100 MB

/// Get temp directory from environment or use default
fn get_temp_dir() -> PathBuf {
    env::var("AI_DB_TEMP_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// # Create Job Endpoint
///
/// This async function handles the creation of a "job" by processing multipart form submissions. It takes in
/// a FASTA file (as a file upload or raw FASTA content) and prepares it for further processing. The function
/// validates the input, streams the file directly to a temporary location on disk, and starts processing the job
/// in the background while responding synchronously to the client.
///
/// ## Endpoint
/// - **Method:** POST
/// - **Path:** `/api/job/`
/// - **Tag:** Jobs
///
/// ## Request Body
/// Multipart form data:
/// - **file**: Optionally upload a FASTA file as a file input.
/// - **fasta_content**: Paste FASTA content directly as an alternative to file upload.
/// - **job_name**: (Optional) Provide a name for the job.
///
/// - **Content-Type:** `multipart/form-data`
/// - **FASTA Data Requirements:** Input can either be a standard FASTA file or raw FASTA content.
/// - **Size Limit:** The file size is capped at `MAX_UPLOAD_SIZE` with appropriate error handling triggered for larger files.
///
/// ## Responses
/// - **201 Created:** Job successfully created. Returns a JSON body of type `JobCreateResponse`.
/// - **400 Bad Request:**
///   - Invalid input, e.g., no file/content received.
///   - File exceeds the maximum allowed size. Returns a JSON body of type `ErrorResponse`.
/// - **500 Internal Server Error:** An error occurred on the server while handling the request or writing the temp file. Returns an `ErrorResponse`.
///
/// ## Workflow
/// 1. **Owner ID Management:**
///    - Extracts or creates an owner ID from cookies.
///
/// 2. **Multipart Form Handling:**
///    - Handles file uploads or directly provided content via multipart streams.
///    - Writes file chunks directly to a temporary file on disk.
///    - Validates the gzip format by examining magic bytes in the first data chunk.
///
/// 3. **Validation:**
///    - Ensures that a valid file or content was uploaded.
///    - Verifies file size constraints against `MAX_UPLOAD_SIZE`.
///
/// 4. **Temporary File Management:**
///    - Uses a temporary directory configured via the `get_temp_dir` function.
///    - Streams multipart form data directly into the temporary file for optimized memory usage.
///    - Automatically cleans up temporary files after successful or failed processing.
///
/// 5. **Job Creation:**
///    - Creates a job in the system with a unique UUID, setting its initial state to `Pending`.
///    - Updates the job state in the application's in-memory `jobs` storage for further reference.
///
/// 6. **Background Processing:**
///    - Schedules the background job processing using `tokio::task::spawn_blocking`.
///    - Processes the uploaded data, handles any errors, and removes temporary files upon completion.
///
/// ## Example Response
/// ```json
/// {
///   "job_id": "uuid-v4-string",
///   "status": "Pending",
///   "created_at": "2023-03-25T18:35:12.345Z",
///   "updated_at": "2023-03-25T18:35:12.345Z",
///   "filename": "user_uploaded_file.fasta",
///   "sequence_count": 0,
///   "processed_count": 0,
///   "hash_matches": 0,
///   "alignment_matches": 0,
///   "sequences": null,
///   "error_message": null,
///   "owner_id": "owner-id-stored-in-cookie"
/// }
/// ```
///
/// ## Error Responses
/// - **400 Bad Request:**
/// ```json
/// {
///   "detail": "File too large. Maximum size is 100 MB."
/// }
/// ```
/// - **500 Internal Server Error:**
/// ```json
/// {
///   "detail": "Failed to create temporary file for upload."
/// }
/// ```
///
/// ## Notes
/// - Gzip files are detected by checking the first two bytes (gzip magic numbers 0x1F and 0x8B).
/// - Background job execution delays include a short sleep to ensure synchronization before processing begins.
///
/// ## Parameters
/// - `State(state): State<AppState>`: Shared application state containing context for processing.
/// - `jar: CookieJar`: Cookie management for identifying ownership of the job.
/// - `multipart: Multipart`: Handles multipart body uploads for files and form fields.
///
/// ## Internal Functions Used
/// - `get_or_create_owner(jar)`: Retrieves or generates an owner ID from the provided cookies.
/// - `get_temp_dir()`: Returns the path to the temporary directory for job processing.
/// - `process_job_from_file`: Handles the sequence processing logic for the uploaded data.
///
/// ## Important Constants
/// - `MAX_UPLOAD_SIZE`: Maximum uploaded file size allowed (in bytes).
///
/// ## Usage
/// Call this endpoint by submitting a valid multipart form containing a FASTA file or content to initiate a job.
#[utoipa::path(
    post,
    path = "/api/job/",
    tag = "Jobs",
    request_body(content_type = "multipart/form-data", description = "FASTA file or FASTA content"),
    responses(
        (status = 201, description = "Job created", body = JobCreateResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse)
    )
)]
async fn create_job(
    State(state): State<AppState>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Get or create owner ID from cookie
    let (owner_id, jar) = get_or_create_owner(jar);

    let mut temp_file: Option<NamedTempFile> = None;
    let mut filename: Option<String> = None;
    let mut job_name: Option<String> = None;
    let mut is_gzip_data = false;
    let mut total_bytes = 0usize;

    // Get temp directory
    let temp_dir = get_temp_dir();

    // Process multipart form data - stream directly to temp file
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();
        let file_name = field.file_name().map(|s| s.to_string());

        match field_name.as_str() {
            "file" | "fasta_content" => {
                if field_name == "file" {
                    filename = file_name;
                }

                // Create temp file in configured directory
                let tf = match TempFileBuilder::new()
                    .prefix("ai-db-upload-")
                    .suffix(".fasta")
                    .tempfile_in(&temp_dir)
                {
                    Ok(tf) => tf,
                    Err(e) => {
                        tracing::error!("Failed to create temp file in {:?}: {}", temp_dir, e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            jar,
                            Json(ErrorResponse {
                                detail: "Failed to create temporary file for upload.".to_string(),
                            }),
                        )
                            .into_response();
                    }
                };

                let mut file = tf;
                let mut first_chunk = true;

                // Stream chunks directly to temp file
                let mut stream = field;
                loop {
                    match stream.chunk().await {
                        Ok(Some(chunk)) => {
                            // Check for gzip magic bytes in first chunk
                            if first_chunk && chunk.len() >= 2 {
                                is_gzip_data = chunk[0] == 0x1f && chunk[1] == 0x8b;
                                first_chunk = false;
                            }

                            // Check size limit
                            if total_bytes + chunk.len() > MAX_UPLOAD_SIZE {
                                return (
                                    StatusCode::BAD_REQUEST,
                                    jar,
                                    Json(ErrorResponse {
                                        detail: format!(
                                            "File too large. Maximum size is {} MB.",
                                            MAX_UPLOAD_SIZE / (1024 * 1024)
                                        ),
                                    }),
                                )
                                    .into_response();
                            }

                            // Write chunk to temp file
                            if let Err(e) = file.write_all(&chunk) {
                                tracing::error!("Failed to write to temp file: {}", e);
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    jar,
                                    Json(ErrorResponse {
                                        detail: "Failed to save uploaded data.".to_string(),
                                    }),
                                )
                                    .into_response();
                            }

                            total_bytes += chunk.len();
                        }
                        Ok(None) => break,
                        Err(e) => {
                            return (
                                StatusCode::BAD_REQUEST,
                                jar,
                                Json(ErrorResponse {
                                    detail: format!("Error reading upload: {}", e),
                                }),
                            )
                                .into_response();
                        }
                    }
                }

                // Flush and save the temp file
                if total_bytes > 0 {
                    if let Err(e) = file.flush() {
                        tracing::warn!("Failed to flush temp file: {}", e);
                    }
                    temp_file = Some(file);
                }
            }
            "job_name" => {
                if let Ok(data) = field.bytes().await {
                    if let Ok(name) = String::from_utf8(data.to_vec()) {
                        job_name = Some(name);
                    }
                }
            }
            _ => {
                // Consume and discard unknown fields
                let _ = field.bytes().await;
            }
        }
    }

    // Validate input
    let tf = match temp_file {
        Some(tf) => tf,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                jar,
                Json(ErrorResponse {
                    detail: "No input received. Please upload a FASTA file or paste FASTA content."
                        .to_string(),
                }),
            )
                .into_response()
        }
    };

    tracing::info!(
        "Received upload: {} bytes, gzip: {}",
        total_bytes,
        is_gzip_data
    );

    // Create job (sequence_count will be determined during processing)
    let job_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let job = JobResponse {
        job_id: job_id.clone(),
        status: JobStatus::Pending,
        created_at: now,
        updated_at: now,
        filename: filename
            .or(job_name.clone())
            .or(Some("direct_input".to_string())),
        sequence_count: 0,
        processed_count: 0,
        hash_matches: 0,
        alignment_matches: 0,
        sequences: None,
        error_message: None,
        owner_id: Some(owner_id.clone()),
    };

    // Store job
    {
        let mut jobs = state.jobs.write();
        jobs.insert(job_id.clone(), job);
    }

    // Keep temp file path for background processing
    let temp_path = tf.into_temp_path();

    // Process job in background
    let state_clone = state.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        // Small delay to ensure job is stored
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Use blocking task for CPU-intensive work
        let state_for_blocking = state_clone.clone();
        let job_id_for_blocking = job_id_clone.clone();
        let path_clone = temp_path.to_path_buf();

        let result = tokio::task::spawn_blocking(move || {
            process_job_from_file(
                &state_for_blocking,
                &job_id_for_blocking,
                &path_clone,
                is_gzip_data,
            );
        })
        .await;

        // Clean up temp file
        if let Err(e) = temp_path.close() {
            tracing::warn!("Failed to clean up temp file: {}", e);
        }

        if let Err(e) = result {
            tracing::error!("Job processing panicked: {}", e);
        }
    });

    // Return response with cookie jar
    (
        StatusCode::CREATED,
        jar,
        Json(JobCreateResponse {
            job_id,
            status: JobStatus::Pending,
            message: "Job successfully created. Processing started.".to_string(),
            sequence_count: 0, // Unknown until processing starts
        }),
    )
        .into_response()
}

/// Fetches a paginated list of jobs for the authenticated owner.
///
/// # Endpoint
/// GET `/api/jobs/`
///
/// # Query Parameters
/// - `page` (optional): The page number of the results, 1-indexed. Defaults to `1`.
/// - `per_page` (optional): The number of jobs to return per page. Defaults to `20`, with a maximum of `100`.
///
/// # Behavior
/// - Only jobs that belong to the authenticated owner are returned, determined by the `OWNER_COOKIE_NAME` cookie.
/// - Jobs are sorted by their `created_at` field in descending order (newest first).
/// - Pagination is applied to the filtered and sorted job list based on the provided or default `page` and `per_page` values.
///
/// # Response
/// - **Status 200**: Returns a JSON object containing the paginated list of jobs and pagination information.
///   - Body: `Vec<JobResponse>`
///     ```json
///     {
///       "jobs": [
///         {
///           "id": "string",
///           "name": "string",
///           "created_at": "datetime",
///           ...
///         }
///       ],
///       "pagination": {
///         "page": 1,
///         "per_page": 20,
///         "total_items": 100,
///         "total_pages": 5
///       }
///     }
///     ```
///
/// # Parameters
/// - `State<AppState>`: Application-wide shared state that contains references to the job repository.
/// - `jar: CookieJar`: Allows fetching cookies, which are used to determine the authenticated owner's ID.
/// - `Query<ListJobsQuery>`: Captures optional query parameters (`page`, `per_page`) for pagination settings.
///
/// # Internal Logic
/// - Defaults are applied for `page` and `per_page` if not provided.
/// - Jobs are filtered to include only those belonging to the authenticated owner (matched by cookie).
/// - The filtered jobs are converted into summaries and sorted by creation time.
/// - Pagination is calculated based on the `page` and `per_page` settings and applied to the sorted job list.
/// - The result, along with pagination metadata, is returned as a JSON response.
///
/// # Notes
/// - If the `OWNER_COOKIE_NAME` does not exist or is invalid, no jobs are returned.
/// - If the requested page exceeds the total number of pages, an empty list of jobs is returned.
///
/// # Example
/// ```http
/// GET /api/jobs/?page=2&per_page=10
///
/// HTTP/1.1 200 OK
/// Content-Type: application/json
///
/// {
///   "jobs": [
///     {
///       "id": "abc123",
///       "name": "Job A",
///       "created_at": "2023-10-01T12:34:56Z"
///     },
///     {
///       "id": "def456",
///       "name": "Job B",
///       "created_at": "2023-09-25T14:20:30Z"
///     }
///   ],
///   "pagination": {
///     "page": 2,
///     "per_page": 10,
///     "total_items": 100,
///     "total_pages": 10
///   }
/// }
/// ```
#[utoipa::path(
    get,
    path = "/api/jobs/",
    tag = "Jobs",
    params(
        ("page" = Option<usize>, Query, description = "Page (1-indexed, Default: 1)"),
        ("per_page" = Option<usize>, Query, description = "Jobs per page (Default: 20, Max: 100)")
    ),
    responses(
        (status = 200, description = "List of jobs", body = Vec<JobResponse>)
    )
)]
async fn list_jobs(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<ListJobsQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .min(MAX_PER_PAGE)
        .max(1);

    // Get owner ID from cookie
    let owner_id = jar.get(OWNER_COOKIE_NAME).map(|c| c.value().to_string());

    let jobs = state.jobs.read();

    // Filter jobs by owner_id and collect as summaries (without sequences)
    let mut job_list: Vec<JobSummary> = jobs
        .values()
        .filter(|job| {
            // Only show jobs that belong to this owner
            match (&job.owner_id, &owner_id) {
                (Some(job_owner), Some(cookie_owner)) => job_owner == cookie_owner,
                _ => false,
            }
        })
        .map(JobSummary::from)
        .collect();

    // Sort by created_at descending
    job_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Calculate pagination
    let total_items = job_list.len();
    let pagination = PaginationInfo::new(page, per_page, total_items);

    // Apply pagination
    let start = (page - 1) * per_page;
    let paginated_jobs: Vec<JobSummary> = if start < job_list.len() {
        let end = (start + per_page).min(job_list.len());
        job_list[start..end].to_vec()
    } else {
        Vec::new()
    };

    Json(PaginatedJobsResponse {
        jobs: paginated_jobs,
        pagination,
    })
}

/// Deletes a job with the specified ID.
///
/// # Endpoint
/// **DELETE** `/api/job/{job_id}`
///
/// # Description
/// This endpoint deletes a job identified by its unique ID (UUID).
/// If the job exists, it will be removed from the system.
/// If the job doesn't exist, a `404 Not Found` response will be returned.
///
/// # Parameters
/// - `job_id` (path parameter): A `String` representing the unique ID of the job to be deleted.
///   The `job_id` is expected to be in the UUID format.
/// - `jar`: Cookie jar containing the session cookie.
///
/// # Responses
/// - **204 No Content:** The job was successfully deleted.
/// - **403 Forbidden:** The job cannot be deleted by the user with the `jar` session cookie.
/// - **404 Not Found:** The job with the specified `job_id` was not found.
///   The response includes an `ErrorResponse` with details.
///
/// # Example
/// ## Request
/// ```http
/// DELETE /api/job/123e4567-e89b-12d3-a456-426614174000 HTTP/1.1
/// Host: example.com
/// ```
///
/// ## Success Response
/// ```http
/// HTTP/1.1 204 No Content
/// ```
///
/// ## Error Response
/// ```http
/// HTTP/1.1 404 Not Found
/// Content-Type: application/json
///
/// {
///   "detail": "Job with ID ‘123e4567-e89b-12d3-a456-426614174000’ not found"
/// }
/// ```
///
/// # Notes
/// - The `AppState` is used to access a shared, thread-safe state which contains the jobs.
/// - The jobs are stored in a `RwLock` to enable concurrent access.
/// - The `ErrorResponse` struct must be implemented to contain the `detail` field for error messages.
#[utoipa::path(
    delete,
    path = "/api/job/{job_id}",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)")
    ),
    responses(
        (status = 204, description = "Job deleted"),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Job not found", body = ErrorResponse)
    )
)]
async fn delete_job(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let owner_id = jar.get(OWNER_COOKIE_NAME).map(|c| c.value().to_string());

    let mut jobs = state.jobs.write();

    // First check if job exists and belongs to owner
    if let Some(job) = jobs.get(&job_id) {
        // Check ownership
        let is_owner = match (&job.owner_id, &owner_id) {
            (Some(job_owner), Some(cookie_owner)) => job_owner == cookie_owner,
            _ => false,
        };

        if !is_owner {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    detail: "No permission to delete this job".to_string(),
                }),
            )
                .into_response();
        }
    }

    match jobs.remove(&job_id) {
        Some(_) => StatusCode::NO_CONTENT.into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                detail: format!("Job mit ID '{}' nicht gefunden", job_id),
            }),
        )
            .into_response(),
    }
}

// ============================================================================
// Download Handlers
// ============================================================================

/// Download format options
#[derive(Debug, Clone, Copy)]
pub enum DownloadFormat {
    Tsv,
    Json,
    Fasta,
    Gff3,
}

impl DownloadFormat {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tsv" => Some(DownloadFormat::Tsv),
            "json" => Some(DownloadFormat::Json),
            "fasta" => Some(DownloadFormat::Fasta),
            "gff3" => Some(DownloadFormat::Gff3),
            _ => None,
        }
    }

    fn content_type(&self) -> &'static str {
        match self {
            DownloadFormat::Tsv => "text/tab-separated-values",
            DownloadFormat::Json => "application/json",
            DownloadFormat::Fasta => "text/x-fasta",
            DownloadFormat::Gff3 => "text/x-gff3",
        }
    }

    fn file_extension(&self) -> &'static str {
        match self {
            DownloadFormat::Tsv => "tsv",
            DownloadFormat::Json => "json",
            DownloadFormat::Fasta => "fasta",
            DownloadFormat::Gff3 => "gff3",
        }
    }
}

/// JSON export structure with full metadata
#[derive(Serialize)]
struct JsonExport {
    job_id: String,
    filename: Option<String>,
    created_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
    statistics: JsonExportStats,
    sequences: Vec<JsonExportSequence>,
}

#[derive(Serialize)]
struct JsonExportStats {
    total_sequences: usize,
    hash_matches: usize,
    alignment_matches: usize,
    no_matches: usize,
}

#[derive(Serialize)]
struct JsonExportSequence {
    id: String,
    length: usize,
    md5_hash: String,
    sequence: Option<String>,
    annotation_source: Option<String>,
    annotation: Option<String>,
    database_ids: JsonExportDbIds,
    database_urls: JsonExportDbUrls,
}

#[derive(Serialize)]
struct JsonExportDbIds {
    uniparc: Option<String>,
    ncbi_nrp: Option<String>,
    uniref100: Option<String>,
}

#[derive(Serialize)]
struct JsonExportDbUrls {
    uniparc: Option<String>,
    ncbi_nrp: Option<String>,
    uniref100: Option<String>,
}

/// Generate TSV output
fn generate_tsv(job: &JobResponse) -> String {
    let mut output = String::new();

    // Header line
    output.push_str("# AI-DB Annotation Results\n");
    output.push_str(&format!("# Job ID: {}\n", job.job_id));
    output.push_str(&format!(
        "# Filename: {}\n",
        job.filename.as_deref().unwrap_or("N/A")
    ));
    output.push_str(&format!("# Created: {}\n", job.created_at));
    output.push_str(&format!("# Total Sequences: {}\n", job.sequence_count));
    output.push_str(&format!("# Hash Matches: {}\n", job.hash_matches));
    output.push_str(&format!("# Alignment Matches: {}\n", job.alignment_matches));
    output.push_str(&format!(
        "# No Matches: {}\n",
        job.sequence_count - job.hash_matches - job.alignment_matches
    ));
    output.push_str("#\n");

    // Column headers
    output.push_str("sequence_id\tlength\tmd5_hash\tannotation_source\tannotation\tuniparc_id\tncbi_nrp_id\tuniref100_id\tuniparc_url\tncbi_url\tuniref100_url\n");

    // Data rows
    if let Some(ref sequences) = job.sequences {
        for seq in sequences {
            let source = seq.annotation_source.as_deref().unwrap_or("none");
            let annotation = seq.annotation.as_deref().unwrap_or("");
            let uniparc = seq.uniparc_id.as_deref().unwrap_or("");
            let ncbi = seq.ncbi_nrp_id.as_deref().unwrap_or("");
            let uniref = seq.uniref100_id.as_deref().unwrap_or("");

            // Generate URLs
            let uniparc_url = seq
                .uniparc_id
                .as_ref()
                .map(|id| format!("https://www.uniprot.org/uniparc/{}", id))
                .unwrap_or_default();
            let ncbi_url = seq
                .ncbi_nrp_id
                .as_ref()
                .map(|id| format!("https://www.ncbi.nlm.nih.gov/protein/{}", id))
                .unwrap_or_default();
            let uniref_url = seq
                .uniref100_id
                .as_ref()
                .map(|id| format!("https://www.uniprot.org/uniref/{}", id))
                .unwrap_or_default();

            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                seq.id,
                seq.length,
                seq.md5_hash,
                source,
                annotation,
                uniparc,
                ncbi,
                uniref,
                uniparc_url,
                ncbi_url,
                uniref_url
            ));
        }
    }

    output
}

/// Generate JSON output
fn generate_json(job: &JobResponse) -> String {
    let sequences: Vec<JsonExportSequence> = job
        .sequences
        .as_ref()
        .map(|seqs| {
            seqs.iter()
                .map(|seq| JsonExportSequence {
                    id: seq.id.clone(),
                    length: seq.length,
                    md5_hash: seq.md5_hash.clone(),
                    sequence: seq.sequence.clone(),
                    annotation_source: seq.annotation_source.clone(),
                    annotation: seq.annotation.clone(),
                    database_ids: JsonExportDbIds {
                        uniparc: seq.uniparc_id.clone(),
                        ncbi_nrp: seq.ncbi_nrp_id.clone(),
                        uniref100: seq.uniref100_id.clone(),
                    },
                    database_urls: JsonExportDbUrls {
                        uniparc: seq
                            .uniparc_id
                            .as_ref()
                            .map(|id| format!("https://www.uniprot.org/uniparc/{}", id)),
                        ncbi_nrp: seq
                            .ncbi_nrp_id
                            .as_ref()
                            .map(|id| format!("https://www.ncbi.nlm.nih.gov/protein/{}", id)),
                        uniref100: seq
                            .uniref100_id
                            .as_ref()
                            .map(|id| format!("https://www.uniprot.org/uniref/{}", id)),
                    },
                })
                .collect()
        })
        .unwrap_or_default();

    let export = JsonExport {
        job_id: job.job_id.clone(),
        filename: job.filename.clone(),
        created_at: job.created_at,
        completed_at: job.updated_at,
        statistics: JsonExportStats {
            total_sequences: job.sequence_count,
            hash_matches: job.hash_matches,
            alignment_matches: job.alignment_matches,
            no_matches: job.sequence_count - job.hash_matches - job.alignment_matches,
        },
        sequences,
    };

    serde_json::to_string_pretty(&export).unwrap_or_default()
}

/// Generate annotated FASTA output
fn generate_fasta(job: &JobResponse) -> String {
    let mut output = String::new();

    if let Some(ref sequences) = job.sequences {
        for seq in sequences {
            // Build header with annotations
            let mut header_parts = vec![seq.id.clone()];

            if let Some(ref source) = seq.annotation_source {
                header_parts.push(format!("source={}", source));
            }

            if let Some(ref annotation) = seq.annotation {
                // Escape any special characters in annotation
                let clean_annotation = annotation.replace("|", "_").replace("\n", " ");
                header_parts.push(format!("annotation={}", clean_annotation));
            }

            if let Some(ref uniparc) = seq.uniparc_id {
                header_parts.push(format!("UniParc={}", uniparc));
            }

            if let Some(ref uniref) = seq.uniref100_id {
                header_parts.push(format!("UniRef100={}", uniref));
            }

            if let Some(ref ncbi) = seq.ncbi_nrp_id {
                header_parts.push(format!("NCBI_NRP={}", ncbi));
            }

            header_parts.push(format!("length={}", seq.length));
            header_parts.push(format!("md5={}", seq.md5_hash));

            output.push_str(&format!(">{}\n", header_parts.join(" | ")));

            // Write sequence (wrapped at 60 characters)
            if let Some(ref sequence) = seq.sequence {
                for chunk in sequence.as_bytes().chunks(60) {
                    output.push_str(&String::from_utf8_lossy(chunk));
                    output.push('\n');
                }
            } else {
                output.push_str("# Sequence not available\n");
            }
        }
    }

    output
}

/// Generate GFF3 output following the GFF3 specification
/// For protein annotations, each sequence is treated as a region with annotation features
fn generate_gff3(job: &JobResponse) -> String {
    let mut output = String::new();

    // GFF3 header (required)
    output.push_str("##gff-version 3\n");

    // Metadata as comments
    output.push_str(&format!("#!annotation-source AI-DB v1.0\n"));
    output.push_str(&format!("#!job-id {}\n", job.job_id));
    if let Some(ref filename) = job.filename {
        output.push_str(&format!("#!original-file {}\n", filename));
    }
    output.push_str(&format!("#!date {}\n", job.created_at.format("%Y-%m-%d")));

    if let Some(ref sequences) = job.sequences {
        // First pass: declare all sequence regions
        for seq in sequences {
            // Sanitize sequence ID for GFF3 (no whitespace, tabs)
            let safe_seqid = sanitize_gff3_seqid(&seq.id);
            output.push_str(&format!(
                "##sequence-region {} 1 {}\n",
                safe_seqid, seq.length
            ));
        }

        // Separator between header and features
        output.push_str("###\n");

        // Second pass: output features
        for (idx, seq) in sequences.iter().enumerate() {
            let safe_seqid = sanitize_gff3_seqid(&seq.id);

            // Determine feature type based on annotation source (SOFA terms)
            // - polypeptide (SO:0000104): A sequence of amino acids
            // - protein_match (SO:0000349): A match to a protein sequence
            let (feature_type, score) = match seq.annotation_source.as_deref() {
                Some("hash_match") => ("protein_match", "."),
                Some("alignment") => ("protein_match", "."),
                _ => ("polypeptide", "."),
            };

            // Build attributes following GFF3 attribute conventions
            let mut attributes = Vec::new();

            // ID is required for features that may be referenced
            attributes.push(format!("ID=seq_{:06}", idx + 1));

            // Name attribute for display
            let display_name = sanitize_gff3_attribute(&seq.id);
            attributes.push(format!("Name={}", display_name));

            // Add MD5 hash as custom attribute
            attributes.push(format!("md5={}", seq.md5_hash));

            // Add annotation note if present
            if let Some(ref annotation) = seq.annotation {
                let encoded = encode_gff3_attribute(annotation);
                attributes.push(format!("Note={}", encoded));
            }

            // Add database cross-references (Dbxref format: DB:ID)
            let mut dbxrefs = Vec::new();
            if let Some(ref uniparc) = seq.uniparc_id {
                dbxrefs.push(format!("UniParc:{}", uniparc));
            }
            if let Some(ref uniref) = seq.uniref100_id {
                dbxrefs.push(format!("UniRef100:{}", uniref));
            }
            if let Some(ref ncbi) = seq.ncbi_nrp_id {
                dbxrefs.push(format!("NCBI_NRP:{}", ncbi));
            }
            if !dbxrefs.is_empty() {
                attributes.push(format!("Dbxref={}", dbxrefs.join(",")));
            }

            // Add ontology term for annotation source
            if let Some(ref source) = seq.annotation_source {
                attributes.push(format!("source_type={}", source));
            }

            // GFF3 columns (tab-separated):
            // seqid, source, type, start, end, score, strand, phase, attributes
            // For proteins: strand is '.', phase is '.' (only relevant for CDS)
            output.push_str(&format!(
                "{}\tai-db\t{}\t1\t{}\t{}\t.\t.\t{}\n",
                safe_seqid,
                feature_type,
                seq.length,
                score,
                attributes.join(";")
            ));
        }
    }

    output
}

/// Sanitize sequence ID for use as GFF3 seqid (column 1)
/// seqid cannot contain whitespace, semicolons, equals signs, or percent signs (unencoded)
fn sanitize_gff3_seqid(id: &str) -> String {
    id.chars()
        .map(|c| match c {
            ' ' | '\t' | '\n' | '\r' => '_',
            ';' | '=' | '%' | '&' | ',' => '_',
            _ => c,
        })
        .collect()
}

/// Sanitize a value for use in GFF3 attributes (not URL-encoded, just cleaned)
fn sanitize_gff3_attribute(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            ';' | '=' | '&' | ',' | '\t' | '\n' | '\r' => '_',
            _ => c,
        })
        .collect()
}

/// URL-encode special characters in GFF3 attribute values
/// Required for: tab, newline, carriage return, semicolons, equals, percent, ampersand, comma
fn encode_gff3_attribute(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '%' => encoded.push_str("%25"),
            ';' => encoded.push_str("%3B"),
            '=' => encoded.push_str("%3D"),
            '&' => encoded.push_str("%26"),
            ',' => encoded.push_str("%2C"),
            '\t' => encoded.push_str("%09"),
            '\n' => encoded.push_str("%0A"),
            '\r' => encoded.push_str("%0D"),
            _ => encoded.push(c),
        }
    }
    encoded
}

/// Handles the `/api/job/{job_id}/download/{format}` endpoint for file downloads.
///
/// This endpoint allows users to download the results of a completed job in a specified format.
/// The download is restricted to the owner of the job and supports multiple formats such as `tsv`,
/// `json`, `fasta`, and `gff3`.
///
/// # Endpoint
/// - `GET /api/job/{job_id}/download/{format}`
///
/// # Path Parameters
/// - `job_id` (String): The unique identifier (UUID) of the job.
/// - `format` (String): The desired file format for download. Supported formats: `tsv`, `json`, `fasta`, `gff3`.
///
/// # Response
///
/// - **200 OK**: Returns the requested file in the specified format.
///     - Content-Type: `application/octet-stream`
///     - Content-Disposition: `attachment; filename="<generated_filename>"`
///
/// - **400 Bad Request**: Returns an error if the format is invalid or if the job is not yet completed.
///     - Example error response:
///       ```json
///       {
///           "detail": "Invalid format 'xyz'. Supported formats: tsv, json, fasta, gff3"
///       }
///       ```
///
/// - **403 Forbidden**: Returns an error if the user is not authorized to download the requested job.
///     - Example error response:
///       ```json
///       {
///           "detail": "Not authorized to download this job"
///       }
///       ```
///
/// - **404 Not Found**: Returns an error if the job is not found.
///     - Example error response:
///       ```json
///       {
///           "detail": "Job with ID '1234' not found"
///       }
///       ```
///
/// # Authorization
/// The endpoint checks if the user is the owner of the job by validating the `OWNER_COOKIE_NAME`
/// stored in the cookies. Only the job owner is authorized to download the results.
///
/// # Preconditions
/// - The job must have a status of `Completed` to be eligible for download.
///
/// # File Naming and Format Details
/// - The filename is dynamically generated based on the job's metadata and requested format.
/// - Supported file extensions and MIME types:
///     - `tsv`: `text/tab-separated-values`
///     - `json`: `application/json`
///     - `fasta`: `text/x-fasta`
///     - `gff3`: `text/x-gff3`
///
/// # Errors
/// - If the `format` parameter is invalid, a `400 Bad Request` response is returned.
/// - If the job is not owned by the requesting user, a `403 Forbidden` response is returned.
/// - If the job does not exist, a `404 Not Found` response is returned.
///
/// # Implementation Details
/// - The function retrieves the job data from the application state.
/// - It ensures that the user has appropriate permissions to access the job.
/// - The requested file is generated dynamically based on the job's content and the chosen format.
///
/// # Example
///
/// ```http
/// GET /api/job/123e4567-e89b-12d3-a456-426614174000/download/tsv HTTP/1.1
/// Host: example.com
/// Cookie: owner_cookie=<owner_id>
/// ```
///
/// Response (200 OK):
/// ```
/// HTTP/1.1 200 OK
/// Content-Type: application/octet-stream
/// Content-Disposition: attachment; filename="results_annotations.tsv"
///
/// <file content>
/// ```
#[utoipa::path(
    get,
    path = "/api/job/{job_id}/download/{format}",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Job ID (UUID)"),
        ("format" = String, Path, description = "Download format: tsv, json, fasta, gff3")
    ),
    responses(
        (status = 200, description = "File download", content_type = "application/octet-stream"),
        (status = 400, description = "Invalid format"),
        (status = 404, description = "Job not found")
    )
)]
async fn download_job(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((job_id, format_str)): Path<(String, String)>,
) -> impl IntoResponse {
    // Parse format
    let format = match DownloadFormat::from_str(&format_str) {
        Some(f) => f,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    detail: format!(
                        "Invalid format '{}'. Supported formats: tsv, json, fasta, gff3",
                        format_str
                    ),
                }),
            )
                .into_response();
        }
    };

    let owner_id = jar.get(OWNER_COOKIE_NAME).map(|c| c.value().to_string());
    let jobs = state.jobs.read();

    match jobs.get(&job_id) {
        Some(job) => {
            // Check ownership
            let is_owner = match (&job.owner_id, &owner_id) {
                (Some(job_owner), Some(cookie_owner)) => job_owner == cookie_owner,
                _ => false,
            };

            if !is_owner {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        detail: "Not authorized to download this job".to_string(),
                    }),
                )
                    .into_response();
            }

            // Check job is completed
            if job.status != JobStatus::Completed {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        detail: "Job is not yet completed".to_string(),
                    }),
                )
                    .into_response();
            }

            // Generate content based on format
            let content = match format {
                DownloadFormat::Tsv => generate_tsv(job),
                DownloadFormat::Json => generate_json(job),
                DownloadFormat::Fasta => generate_fasta(job),
                DownloadFormat::Gff3 => generate_gff3(job),
            };

            // Generate filename
            let base_name = job
                .filename
                .as_ref()
                .map(|f| {
                    f.trim_end_matches(".gz")
                        .trim_end_matches(".fasta")
                        .trim_end_matches(".fa")
                })
                .unwrap_or("results");
            let filename = format!("{}_annotations.{}", base_name, format.file_extension());

            // Return file response
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, format.content_type()),
                    (
                        header::CONTENT_DISPOSITION,
                        &format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                content,
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                detail: format!("Job with ID '{}' not found", job_id),
            }),
        )
            .into_response(),
    }
}

/// Health check endpoint handler.
///
/// This asynchronous function is responsible for handling requests to the health check endpoint.
/// It returns a JSON response containing the current status of the service and the state of the `bakta_db` database.
///
/// # Parameters
/// - `State(state): State<AppState>`: The shared application state passed into the handler. This contains
///   application-specific configurations and functionality.
///
/// # Returns
/// An implementation of `IntoResponse`, typically a JSON object with the following structure:
/// ```json
/// {
///     "status": "healthy",
///     "service": "ai-db-api",
///     "bakta_db": {
///         "status": <db_status>,
///         "path": <database path or null>
///     }
/// }
/// ```
///
/// - **`status`**: Fixed value "healthy", indicating the service is operational.
/// - **`service`**: Fixed value "ai-db-api", identifying the service name.
/// - **`bakta_db.status`**: Indicates the state of the `bakta_db` database:
///     - `"connected"`: Database exists and connection is successful.
///     - `"error"`: Database exists but connection could not be established.
///     - `"not_found"`: Database path is defined in the state but the file does not exist.
///     - `"not_configured"`: Database path is not configured in the application state.
/// - **`bakta_db.path`**: The file system path to the `bakta_db` database, or `null` if not configured.
///
/// # Example Response:
/// If the database is configured correctly and accessible:
/// ```json
/// {
///     "status": "healthy",
///     "service": "ai-db-api",
///     "bakta_db": {
///         "status": "connected",
///         "path": "/path/to/bakta_db.sqlite"
///     }
/// }
/// ```
///
/// If the database is neither configured nor found:
/// ```json
/// {
///     "status": "healthy",
///     "service": "ai-db-api",
///     "bakta_db": {
///         "status": "not_configured",
///         "path": null
///     }
/// }
/// ```
///
/// # Notes:
/// - This function attempts to establish a database connection for verification only;
///   the actual connection result is used to determine the `status`.
/// - The `.exists()` method is used to check if the path to the database is valid.
/// - Errors during database connection are not propagated, but instead reflected in the response as `"error"`.
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service health status", body = HealthCheckResponse)
    )
)]
async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_status = if let Some(ref path) = state.bakta_db_path {
        if path.exists() {
            // Try to open connection to verify database is accessible
            match state.open_db_connection() {
                Some(_) => "connected",
                None => "error",
            }
        } else {
            "not_found"
        }
    } else {
        "not_configured"
    };

    Json(serde_json::json!({
        "status": "healthy",
        "service": "ai-db-api",
        "bakta_db": {
            "status": db_status,
            "path": state.bakta_db_path.as_ref().map(|p| p.display().to_string())
        }
    }))
}

/// Asynchronous function to fetch database-related information.
///
/// This function attempts to open a database connection via the application's shared state.
/// If the connection is successful, it retrieves:
/// - The total row count from the `ups` table.
/// - The database version information (if available) from the `version` table.
///
/// If the connection fails, an appropriate error response is returned.
///
/// # Parameters
/// - `State(state): State<AppState>`: The shared application state that contains the database path
///   and functionality to establish a database connection.
///
/// # Returns
/// This function returns a JSON response containing the following keys:
/// - `available` (Boolean): Indicates whether the database is accessible.
/// - `path` (Optional<String>): The filesystem path to the database (if set).
/// - `ups_entries` (Optional<i64>): The total number of entries in the `ups` table (if the count query succeeds).
/// - `version` (Optional<String>): The database version extracted from the `version` table.
/// - `error` (Optional<String>): Contains an error message only when the database connection fails.
///
/// ## Example of Successful Response
/// ```json
/// {
///     "available": true,
///     "path": "/path/to/database.db",
///     "ups_entries": 42,
///     "version": "1.0.0"
/// }
/// ```
///
/// ## Example of Failed Response
/// ```json
/// {
///     "available": false,
///     "path": "/path/to/database.db",
///     "error": "Could not connect to database"
/// }
/// ```
///
/// # Errors
/// - If the database connection cannot be established, the response will indicate `"available": false`
///   and include an `error` field with a descriptive message.
#[utoipa::path(
    get,
    path = "/db/info",
    tag = "Database",
    responses(
        (status = 200, description = "Database information", body = DbInfoResponse)
    )
)]
async fn db_info(State(state): State<AppState>) -> impl IntoResponse {
    let db_info = if let Some(conn) = state.open_db_connection() {
        // Get row count from ups table
        let ups_count: Result<i64, _> =
            conn.query_row("SELECT COUNT(*) FROM ups", [], |row| row.get(0));

        // Try to get version info if available
        let version: Option<String> = conn
            .query_row(
                "SELECT json_extract(info, '$.version') FROM version LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        serde_json::json!({
            "available": true,
            "path": state.bakta_db_path.as_ref().map(|p| p.display().to_string()),
            "ups_entries": ups_count.ok(),
            "version": version
        })
    } else {
        serde_json::json!({
            "available": false,
            "path": state.bakta_db_path.as_ref().map(|p| p.display().to_string()),
            "error": "Could not connect to database"
        })
    };

    Json(db_info)
}

// ============================================================================
// OpenAPI Documentation
// ============================================================================

#[derive(OpenApi)]
#[openapi(
    info(
        title = "AI-DB REST API",
        version = "1.0.0",
        description = "Hash-Based Annotation Service for Microbial Sequencing Data\n\n\
            AI-DB accelerates the analysis of microbial sequencing data while \
            maintaining data sovereignty through cryptographic hash-based annotations.\n\n\
            ## Features\n\n\
            - **Data protection**: Sequence data is processed as MD5 hashes\n\
            - **Fast**: Hash-based annotations in seconds instead of hours\n\
            - **Comprehensive**: Access to UniRef protein annotations\n\
            - **Fallback**: Diamond alignment for new sequences",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
        contact(name = "AI-DB Team", url = "https://github.com/hansen-maria/AI-DB-Web")
    ),
    tags(
        (name = "Jobs", description = "Annotation Job Management - Creating and Querying Jobs")
    ),
    paths(health_check, db_info, create_job, get_job, list_jobs, download_job, delete_job),
    components(schemas(
        JobStatus,
        SequenceInfo,
        JobResponse,
        JobCreateResponse,
        ErrorResponse,
        PaginationInfo,
        PaginatedJobsResponse,
        JobSummary,
        PaginatedJobResponse,
        ListJobsQuery,
        GetJobQuery,
        SequenceFilter,
        HashLookupResult,
        HealthCheckResponse,
        BaktaDbHealth,
        DbInfoResponse,
    ))
)]
struct ApiDoc;

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let state = AppState::new();

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::mirror_request())
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::AUTHORIZATION])
        .allow_credentials(true);

    // Build router
    let app = Router::new()
        // API routes
        .route("/api/health", get(health_check))
        .route("/api/db/info", get(db_info))
        .route("/api/job/", post(create_job))
        .route("/api/job/{job_id}", get(get_job).delete(delete_job))
        .route("/api/job/{job_id}/download/{format}", get(download_job))
        .route("/api/jobs/", get(list_jobs))
        // Swagger UI
        .merge(SwaggerUi::new("/api/docs/").url("/api/openapi.json", ApiDoc::openapi()))
        // Middleware
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100 MB Limit
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // State
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("Starting AI-DB API server on http://{}", addr);
    tracing::info!("Swagger UI available at http://{}/api/docs/", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
