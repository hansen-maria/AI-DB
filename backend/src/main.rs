//! AI-DB REST API Backend
//! Hash-Based Annotation Service for Microbial Sequencing Data

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use md5::{Digest, Md5};
use parking_lot::RwLock;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, net::SocketAddr, path::PathBuf, sync::Arc};
use std::io::Read;
use axum::http::{header, Method};
use tower_http::cors::{Any, CorsLayer};
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
    /// Found annotations
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

/// Query parameters for job lists
#[derive(Debug, Deserialize)]
pub struct ListJobsQuery {
    limit: Option<usize>,
}

/// Bakta Hash Lookup Result
#[derive(Debug, Clone)]
pub struct HashLookupResult {
    pub found: bool,
    pub db_length: Option<i64>,
    pub uniparc_id: Option<String>,
    pub ncbi_nrp_id: Option<String>,
    pub uniref100_id: Option<String>,
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
                tracing::warn!("Bakta database path configured but file not found: {:?}", path);
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
// FASTA Parsing
// ============================================================================

/// Checks if data is gzip compressed (magic bytes: 0x1f 0x8b)
fn is_gzip(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
}

/// Decompresses gzip data, returns original data if not compressed
fn decompress_if_gzip(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    if is_gzip(data) {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    } else {
        Ok(data.to_vec())
    }
}

/// Parses a FASTA-formatted string and returns a vector of tuples, where each tuple contains a
/// sequence identifier (header) and its corresponding sequence.
///
/// FASTA format is a widely used text-based format for representing nucleotide or protein sequences.
/// Each sequence in the FASTA file starts with a line beginning with the `>` character, followed by
/// an identifier for the sequence (header). Subsequent lines contain the sequence data, while empty
/// lines are ignored.
///
/// # Arguments
///
/// * `content` - A string slice representing the FASTA-formatted input.
///
/// # Returns
///
/// A `Vec` of tuples, where:
/// - The first element of the tuple is a `String` containing the sequence identifier (header).
/// - The second element of the tuple is a `String` containing the nucleotide or protein sequence in uppercase.
///
/// # Behavior
///
/// - Sequence headers are extracted from lines starting with `>` and are trimmed to the first word (separated by whitespace).
/// - Sequence data accumulates until a new header is encountered or the content ends.
/// - All sequences are converted to uppercase.
/// - Empty lines are ignored.
/// - If duplicate headers are present, only the most recent sequence associated with that header is stored.
///
/// # Example
///
/// ```
/// let fasta_data = ">seq1
/// ATGCGT
/// AACGT
/// >seq2
/// GGATA
/// ";
/// let result = parse_fasta(fasta_data);
/// assert_eq!(
///     result,
///     vec![
///         ("seq1".to_string(), "ATGCGTAACGT".to_string()),
///         ("seq2".to_string(), "GGATA".to_string())
///     ]
/// );
/// ```
///
/// # Edge Cases
///
/// - If the input string is empty, the function returns an empty vector.
/// - If a header is present but no sequence data appears below it, the header is ignored.
/// - If sequence data exists without a preceding header, it will not be included in the output.
///
/// # Notes
///
/// This function assumes valid FASTA input and does not perform extensive validation.
fn parse_fasta(content: &str) -> Vec<(String, String)> {
    let mut sequences = Vec::new();
    let mut current_header: Option<String> = None;
    let mut current_sequence = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('>') {
            if let Some(header) = current_header.take() {
                if !current_sequence.is_empty() {
                    sequences.push((header, current_sequence.clone()));
                }
            }
            current_header = Some(
                line[1..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string(),
            );
            current_sequence.clear();
        } else {
            current_sequence.push_str(&line.to_uppercase());
        }
    }

    if let Some(header) = current_header {
        if !current_sequence.is_empty() {
            sequences.push((header, current_sequence));
        }
    }

    sequences
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
fn lookup_hash_in_bakta(conn: &Connection, hash_bytes: &[u8], seq_length: usize) -> HashLookupResult {
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
        Ok(mut result) => {
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

/// Processes a job by analyzing and annotating sequences, optionally utilizing
/// a database for sequence hash lookup.
///
/// # Parameters
/// - `state`: A reference to the shared application state (`AppState`) containing job tracking
///   information and database connection management.
/// - `job_id`: A string slice representing the unique identifier of the job to process.
/// - `sequences`: A vector of tuples containing sequence headers and sequence strings for processing.
///
/// # Description
/// This function performs the following steps:
/// 1. Updates the job status to `Processing` in the application state and records the current time.
/// 2. Attempts to create a database connection to enable sequence hash lookups in the Bakta database.
/// 3. Iterates over the provided sequences:
///    - Computes the MD5 hash of each sequence.
///    - If a database connection is available, performs a hash lookup to retrieve metadata
///      and annotations for the sequence.
///    - Records the sequence information, annotation, and any metadata retrieved (such as UniProt
///      identifiers) in a `SequenceInfo` structure.
///    - Tracks the number of hash matches and alignment matches.
/// 4. Updates the job's status to `Completed` in the application state, along with the processed
///    results, including sequence count, hash match count, alignment match count, and the annotated
///    sequences.
/// 5. Logs the processing results, including the number of sequences and hash matches.
///
/// # Behavior
/// - If the database connection is unavailable, the function proceeds with processing but skips the
///   hash lookup step, logging a warning that no matches are possible.
/// - Sequence annotations are sourced from either database matches (`hash_match`) or left empty if
///   no match is found.
///
/// # Notes
/// - The `state` parameter provides thread-safe access to shared state using an internal read-write
///   lock, ensuring consistent updates to job details.
/// - The final results, including job status and sequence information, are stored in the application
///   state to ensure that downstream systems can retrieve the processed data.
///
/// # Example
/// ```rust
/// let state = AppState::new(); // Initialize application state
/// let job_id = "job_12345";
/// let sequences = vec![
///     ("seq1".to_string(), "ATGC".to_string()),
///     ("seq2".to_string(), "GGTA".to_string()),
/// ];
///
/// process_job(&state, job_id, sequences);
/// ```
///
/// # See Also
/// - `AppState`: Manages shared application state and database connections.
/// - `compute_md5`: Computes the MD5 hash for a given sequence string.
/// - `lookup_hash_in_bakta`: Executes a database query to match sequences using their MD5 hashes.
/// - `SequenceInfo`: Stores details of a processed sequence, including annotations and metadata.
fn process_job(state: &AppState, job_id: &str, sequences: Vec<(String, String)>) {
    // Set status to processing
    {
        let mut jobs = state.jobs.write();
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Processing;
            job.updated_at = Utc::now();
        }
    }

    // Try to open a database connection
    let db_conn = state.open_db_connection();
    let db_available = db_conn.is_some();

    if db_available {
        tracing::info!("Processing job {} with Bakta database lookup", job_id);
    } else {
        tracing::warn!("Processing job {} without database (no matches possible)", job_id);
    }

    let mut sequence_infos = Vec::new();
    let mut hash_matches = 0;
    let mut alignment_matches = 0;

    for (header, seq) in &sequences {
        let (hash_hex, hash_bytes) = compute_md5(seq);
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
            (format_annotation(&lookup_result), Some("hash_match".to_string()))
        } else {
            (None, None)
        };

        sequence_infos.push(SequenceInfo {
            id: header.clone(),
            md5_hash: hash_hex,
            length: seq_length,
            annotation,
            annotation_source,
            uniparc_id: lookup_result.uniparc_id,
            ncbi_nrp_id: lookup_result.ncbi_nrp_id,
            uniref100_id: lookup_result.uniref100_id,
        });
    }

    // Update job with results
    {
        let mut jobs = state.jobs.write();
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Completed;
            job.updated_at = Utc::now();
            job.processed_count = sequences.len();
            job.hash_matches = hash_matches;
            job.alignment_matches = alignment_matches;
            job.sequences = Some(sequence_infos);
        }
    }

    tracing::info!(
        "Job {} completed: {} sequences, {} hash matches",
        job_id,
        sequences.len(),
        hash_matches
    );
}

// ============================================================================
// API Handlers
// ============================================================================

/// API Handler and Documentation for the `get_job` function.
///
/// This function defines a REST API endpoint to fetch details of a job based on the provided job ID.
/// It is accessible via a `GET` request to the `/api/job/{job_id}` route.
///
/// # Attributes
///
/// - `#[utoipa::path]`:
///   - **HTTP Method**: `GET`
///   - **Path**: `/api/job/{job_id}`
///   - **Tag**: `Jobs`
///   - **Parameters**:
///     - `job_id` (Path - String): A unique identifier for the job (UUID).
///   - **Responses**:
///     - `200 OK`: Returned when the job is found. The body contains a JSON serialized `JobResponse`.
///     - `404 NOT FOUND`: Returned when the job is not found. The body contains an `ErrorResponse` with a descriptive error message.
///
/// # Parameters
///
/// - `State(state)`: Shared application state (`AppState`) passed to the handler. This includes a read lock on the `jobs` store, which holds all job data.
/// - `Path(job_id)`: The unique ID of the job retrieved from the request path.
///
/// # Returns
///
/// This function returns a response that implements the `IntoResponse` trait. The response can be one of the following:
/// - **Status 200 OK**: If the job is found, it includes a JSON representation of the job in the response body.
/// - **Status 404 NOT FOUND**: If the job is not found, it includes an error message in the response body.
///
/// # Examples
///
/// ## Request
/// ```http
/// GET /api/job/123e4567-e89b-12d3-a456-426614174000 HTTP/1.1
/// Host: example.com
/// ```
///
/// ## Response (200 OK)
/// ```json
/// {
///   "id": "123e4567-e89b-12d3-a456-426614174000",
///   "name": "Job Name",
///   "status": "In Progress"
/// }
/// ```
///
/// ## Response (404 NOT FOUND)
/// ```json
/// {
///   "detail": "Job with ID '123e4567-e89b-12d3-a456-426614174000' not found"
/// }
/// ```
///
/// # Implementation Details
///
/// - The function uses a read-lock (`state.jobs.read()`) to access the `jobs` store from `AppState`.
/// - It attempts to fetch the job corresponding to the given `job_id` from the `jobs` map.
/// - If the job exists, its data is serialized to JSON and returned with a `200 OK` status.
/// - If the job is not found, an `ErrorResponse` with a descriptive error message is created and returned with a `404 NOT FOUND` status.
#[utoipa::path(
    get,
    path = "/api/job/{job_id}",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)")
    ),
    responses(
        (status = 200, description = "Job found", body = JobResponse),
        (status = 404, description = "Job not found", body = ErrorResponse)
    )
)]
async fn get_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let jobs = state.jobs.read();

    match jobs.get(&job_id) {
        Some(job) => (StatusCode::OK, Json(serde_json::to_value(job).unwrap())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                detail: format!("Job with ID '{}' not found", job_id),
            }),
        )
            .into_response(),
    }
}

/// Handles the creation of a new bioinformatics job based on user-provided FASTA input.
///
/// ## Endpoint
/// - **Method**: POST
/// - **Path**: `/api/job/`
/// - **Tag**: `Jobs`
///
/// ## Request
/// This endpoint supports `multipart/form-data` content type for uploading either a FASTA file
/// or raw FASTA content. Additionally, an optional job name can be provided.
/// - **Fields**:
///   - `file` (optional): A FASTA file. Gzipped files are supported and will be decompressed.
///   - `fasta_content` (optional): Raw FASTA content in the request body. Gzipped content is also supported.
///   - `job_name` (optional): A name for the job to be created.
///
/// ## Responses
/// - **201 Created**: Returns upon successful job creation.
///   - **Body**: [`JobCreateResponse`]
///   - Details:
///     - `job_id`: Unique identifier of the created job.
///     - `status`: Initial status of the job (always "Pending").
///     - `message`: A success message indicating the job has started processing.
///     - `sequence_count`: The number of valid sequences present in the provided FASTA content.
/// - **400 Bad Request**: Returned in one of the following cases:
///   - Input is invalid or missing (e.g., no FASTA data provided).
///   - An error occurred while processing the request (e.g., malformed data).
///   - **Body**: [`ErrorResponse`]
///   - Details:
///     - `detail`: Human-readable error explanation.
///
/// ## Behavior
/// - Extracts an owner ID from the cookie or generates a new one if not present.
/// - Reads the request's multipart content to handle either a file upload or raw FASTA content.
/// - Attempts to decompress Gzipped files and content as needed.
/// - Parses the FASTA data to extract valid sequences.
/// - Creates a job entry with an initial status of `Pending`.
/// - Stores the job in the application's state and triggers asynchronous background processing of the job.
///
/// ## Error Handling
/// - If no valid FASTA content is found in the request, returns a `400 Bad Request` with an error message.
/// - Any errors during the multipart data read or decompression are logged and result in a failure response.
///
/// ## Background Processing
/// - After job creation, processing of FASTA sequences occurs asynchronously using a separate task.
/// - The processing involves computation tasks, alignment, and hashing, which update the job's status.
///
/// ## Security
/// - Owner IDs are managed within cookies to associate jobs with specific users.
/// - No sensitive data is exposed in the request or response bodies.
///
/// ## Example
///
/// ### Request (file upload):
/// ```http
/// POST /api/job/
/// Content-Type: multipart/form-data
///
/// --boundary
/// Content-Disposition: form-data; name="file"; filename="example.fasta"
/// Content-Type: application/octet-stream
///
/// >ACTG...
/// --boundary--
/// ```
///
/// ### Request (raw FASTA content):
/// ```http
/// POST /api/job/
/// Content-Type: multipart/form-data
///
/// --boundary
/// Content-Disposition: form-data; name="fasta_content"
///
/// >sequence1
/// ATCG
/// >sequence2
/// TAGC
/// --boundary--
/// ```
///
/// ### Response (201 Created):
/// ```json
/// {
///   "job_id": "123e4567-e89b-12d3-a456-426614174000",
///   "status": "Pending",
///   "message": "Job successfully created. Processing started.",
///   "sequence_count": 2
/// }
/// ```
///
/// ### Response (400 Bad Request):
/// ```json
/// {
///   "detail": "No input received. Please upload FASTA file or send FASTA content."
/// }
/// ```
///
/// ## Dependencies
/// - This function relies on the following components:
///   - `AppState`: Shared application state for storing jobs.
///   - `CookieJar`: Used for managing owner IDs.
///   - `Multipart`: For handling multipart form data.
///   - `parse_fasta`: To parse and extract sequences from FASTA content.
///   - `decompress_if_gzip`: To support Gzipped file or content uploads.
///   - `process_job`: Background task handler for processing the job asynchronously.
///
/// ## Notes
/// - The job creation process ensures idempotent and asynchronous handling to avoid blocking the request/response lifecycle.
/// - Errors related to malformed input or file handling are logged with appropriate warnings for debugging.
///
/// ## Parameters
/// - `State(state)`: Extracted application-wide state (`AppState`) used for storing and accessing jobs.
/// - `jar`: A `CookieJar` for handling owner tracking and managing session-scoped cookies.
/// - `multipart`: A `Multipart` instance to process and read multipart request data.
///
/// ## Returns
/// - A type implementing `IntoResponse`, containing either success or error HTTP responses (201 or 400).
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

    let mut fasta_content: Option<String> = None;
    let mut filename: Option<String> = None;
    let mut job_name: Option<String> = None;

    // Process multipart form data
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();
        let file_name = field.file_name().map(|s| s.to_string());

        match field.bytes().await {
            Ok(data) => match field_name.as_str() {
                "file" => {
                    filename = file_name;
                    // Try to decompress if gzip, otherwise use raw data
                    let raw_data = match decompress_if_gzip(&data) {
                        Ok(decompressed) => decompressed,
                        Err(e) => {
                            tracing::warn!("Gzip decompression failed: {}, using raw data", e);
                            data.to_vec()
                        }
                    };
                    if let Ok(content) = String::from_utf8(raw_data) {
                        if !content.is_empty() {
                            fasta_content = Some(content);
                        }
                    }
                }
                "fasta_content" => {
                    // Also support gzip for direct content
                    let raw_data = match decompress_if_gzip(&data) {
                        Ok(decompressed) => decompressed,
                        Err(_) => data.to_vec()
                    };
                    if let Ok(content) = String::from_utf8(raw_data) {
                        if !content.is_empty() {
                            fasta_content = Some(content);
                        }
                    }
                }
                "job_name" => {
                    if let Ok(name) = String::from_utf8(data.to_vec()) {
                        job_name = Some(name);
                    }
                }
                _ => {}
            },
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    jar,
                    Json(ErrorResponse {
                        detail: format!("Fehler beim Lesen der Daten: {}", e),
                    }),
                ).into_response()
            }
        }
    }

    // Validate input
    let content = match fasta_content {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                jar,
                Json(ErrorResponse {
                    detail: "No input received. Please upload FASTA file or send FASTA content.".to_string(),
                }),
            ).into_response()
        }
    };

    // Parse FASTA
    let sequences = parse_fasta(&content);

    if sequences.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            jar,
            Json(ErrorResponse {
                detail: "No valid sequences found in the input.".to_string(),
            }),
        ).into_response();
    }

    // Create job
    let job_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let sequence_count = sequences.len();

    let job = JobResponse {
        job_id: job_id.clone(),
        status: JobStatus::Pending,
        created_at: now,
        updated_at: now,
        filename: filename
            .or(job_name.clone())
            .or(Some("direct_input".to_string())),
        sequence_count,
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

    // Process job in background
    let state_clone = state.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        // Small delay to ensure job is stored
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        process_job(&state_clone, &job_id_clone, sequences);
    });

    // Return response with cookie jar
    (
        StatusCode::CREATED,
        jar,
        Json(JobCreateResponse {
            job_id,
            status: JobStatus::Pending,
            message: "Job successfully created. Processing started.".to_string(),
            sequence_count,
        }),
    ).into_response()
}

/// `list_jobs` is an HTTP GET endpoint to retrieve a list of jobs created by the user.
///
/// This endpoint is part of the "Jobs" API and provides a paginated list of jobs created by the user
/// with an optional limit parameter to specify the maximum number of jobs to return.
/// By default, the limit is set to 100 if not provided in the query parameters.
///
/// # Endpoint
/// `/api/jobs/`
///
/// # Parameters
///
/// - `limit` (optional): `Option<usize>`
///     - Query parameter that specifies the maximum number of job entries to return.
///     - Default value: 100.
///
/// # Responses
///
/// - **200 OK**: Returns a JSON array of jobs (`Vec<JobResponse>`), sorted by their
///   `created_at` property in descending order.
///
/// # Arguments
///
/// - `State<AppState>`: Application state containing shared resources, such as the in-memory jobs store.
///     - Used to access the current list of jobs.
/// - `CookieJar`: Cookie jar containing the session cookie.
/// - `Query<ListJobsQuery>`: Query parameters passed to the endpoint, including the optional `limit`.
///
/// # Returns
///
/// A JSON response containing a list of jobs (`JobResponse`) limited to the specified number
/// and sorted by the most recently created jobs.
///
/// # Implementation Details
/// 1. Extracts the `limit` parameter from the query parameters, defaulting to 100 if not specified.
/// 2. Reads the current list of jobs from the application's state.
/// 3. Converts the jobs into a vector (`Vec<JobResponse>`) and sorts them in descending order by their `created_at` timestamp.
/// 4. Truncates the list of jobs to the specified `limit`.
/// 5. Returns the resulting list as a JSON response.
///
/// # Examples
///
/// **Request:**
/// ```http
/// GET /api/jobs/?limit=10
/// ```
///
/// **Response:**
/// ```json
/// [
///   {
///     "id": "job1",
///     "name": "Example Job",
///     "created_at": "2023-10-01T12:00:00Z"
///   },
///   ...
/// ]
/// ```
///
/// **Request with Default Limit:**
/// ```http
/// GET /api/jobs/
/// ```
///
/// **Response:**
/// Same as above, but returns up to 100 jobs by default.
///
/// Note: The function ensures thread-safe read access to the shared job list using the `RwLock`.
#[utoipa::path(
    get,
    path = "/api/jobs/",
    tag = "Jobs",
    params(
        ("limit" = Option<usize>, Query, description = "Maximum number of jobs")
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
    let limit = query.limit.unwrap_or(100);

    // Get owner ID from cookie
    let owner_id = jar.get(OWNER_COOKIE_NAME).map(|c| c.value().to_string());

    let jobs = state.jobs.read();

    // Filter jobs by owner_id
    let mut job_list: Vec<JobResponse> = jobs
        .values()
        .filter(|job| {
            // Only show jobs that belong to this owner
            match (&job.owner_id, &owner_id) {
                (Some(job_owner), Some(cookie_owner)) => job_owner == cookie_owner,
                _ => false, // Don't show jobs without owner or if no cookie
            }
        })
        .cloned()
        .collect();

    // Sort by created_at descending
    job_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Apply limit
    job_list.truncate(limit);

    Json(job_list)
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
            ).into_response();
        }
    }

    match jobs.remove(&job_id) {
        Some(_) => StatusCode::NO_CONTENT.into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                detail: format!("Job with ID ‘{}’ not found", job_id),
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
async fn db_info(State(state): State<AppState>) -> impl IntoResponse {
    let db_info = if let Some(conn) = state.open_db_connection() {
        // Get row count from ups table
        let ups_count: Result<i64, _> = conn.query_row(
            "SELECT COUNT(*) FROM ups",
            [],
            |row| row.get(0)
        );

        // Try to get version info if available
        let version: Option<String> = conn.query_row(
            "SELECT json_extract(info, '$.version') FROM version LIMIT 1",
            [],
            |row| row.get(0)
        ).ok();

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
        license(name = "MIT"),
        contact(name = "AI-DB Team", url = "https://github.com/hansen-maria/AI-DB-Web")
    ),
    tags(
        (name = "Jobs", description = "Annotation Job Management - Creating and Querying Jobs")
    ),
    paths(get_job, create_job, list_jobs, delete_job),
    components(schemas(
        JobStatus,
        SequenceInfo,
        JobResponse,
        JobCreateResponse,
        ErrorResponse
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
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let state = AppState::new();

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::mirror_request())
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::OPTIONS
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::AUTHORIZATION
        ])
        .allow_credentials(true);

    // Build router
    let app = Router::new()
        // API routes
        .route("/api/health", get(health_check))
        .route("/api/db/info", get(db_info))
        .route("/api/job/", post(create_job))
        .route("/api/job/{job_id}", get(get_job).delete(delete_job))
        .route("/api/jobs/", get(list_jobs))
        // Swagger UI
        .merge(SwaggerUi::new("/api/docs/").url("/api/openapi.json", ApiDoc::openapi()))
        // Middleware
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