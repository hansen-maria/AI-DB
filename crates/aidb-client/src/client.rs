//! HTTP client for the AI-DB REST API.

use std::path::Path;
use std::time::Duration;

use aidb_types::{
    DbInfoResponse, ErrorResponse, GetJobQuery, HealthCheckResponse, JobCreateResponse, JobStatus,
    JobViewResponse, ListJobsQuery, PaginatedJobsResponse, SequenceFilterQuery,
};
use futures_util::TryStreamExt;
use reqwest::multipart::{Form, Part};
use reqwest::{Body, RequestBuilder, Url};
use serde::de::DeserializeOwned;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio_util::io::ReaderStream;

use crate::error::{Error, Result};

/// Public AI-DB instance, used when no server is configured.
pub const DEFAULT_SERVER: &str = "https://ai-db.computational.bio";

/// Default timeout for a whole request.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Largest upload the server accepts (`MAX_UPLOAD_SIZE` in the backend).
pub const MAX_UPLOAD_BYTES: u64 = 100 * 1024 * 1024;

/// Upper bound for establishing the connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Uploads of up to 100 MB may take long on slow links.
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Longest server error text passed on to the user.
const MAX_DETAIL_CHARS: usize = 300;

/// Result file formats of `GET /api/job/{job_id}/download/{format}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultFormat {
    Tsv,
    Json,
    Fasta,
    Gff3,
}

impl ResultFormat {
    pub const ALL: [ResultFormat; 4] = [Self::Tsv, Self::Json, Self::Fasta, Self::Gff3];

    /// Path segment and file extension.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tsv => "tsv",
            Self::Json => "json",
            Self::Fasta => "fasta",
            Self::Gff3 => "gff3",
        }
    }
}

impl std::fmt::Display for ResultFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether a request carries the owner token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Auth {
    /// Public endpoint: the token is never sent.
    None,
    /// The token is sent if one is configured.
    Owner,
}

/// Builder for [`Client`].
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    server: String,
    token: Option<String>,
    timeout: Duration,
    user_agent: String,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            server: DEFAULT_SERVER.to_string(),
            token: None,
            timeout: DEFAULT_TIMEOUT,
            user_agent: concat!("aidb-client/", env!("CARGO_PKG_VERSION")).to_string(),
        }
    }
}

impl ClientBuilder {
    /// Base URL of the AI-DB instance, e.g. `https://ai-db.computational.bio`.
    /// A path is kept, so instances served below a sub path work as well.
    pub fn server(mut self, url: impl Into<String>) -> Self {
        self.server = url.into();
        self
    }

    /// Owner token (UUIDv4) sent as `Authorization: Bearer` on owner routes.
    pub fn token(mut self, token: Option<String>) -> Self {
        self.token = token.filter(|t| !t.trim().is_empty());
        self
    }

    /// Timeout for a whole request (connecting is capped at 10 s). Uploads use
    /// a longer timeout of their own.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// `User-Agent` header, e.g. `aidb-cli/0.1.0`.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    pub fn build(self) -> Result<Client> {
        let base = normalize_server_url(&self.server)?;
        // No client-wide total timeout: it is set per request, so uploads can
        // take longer than ordinary API calls
        let http = reqwest::Client::builder()
            .user_agent(self.user_agent)
            .connect_timeout(self.timeout.min(CONNECT_TIMEOUT))
            .build()
            .map_err(Error::Http)?;
        Ok(Client {
            http,
            base,
            token: self.token,
            timeout: self.timeout,
        })
    }
}

/// Options for [`Client::wait_for_job`].
#[derive(Debug, Clone)]
pub struct WaitOptions {
    /// Delay before the second poll; grows by 1.5× up to `max_interval`.
    pub initial_interval: Duration,
    pub max_interval: Duration,
    /// Give up with [`Error::WaitTimeout`] after this long (`None`: never).
    pub max_wait: Option<Duration>,
    /// Consecutive temporary failures (unreachable, 429, 5xx) tolerated
    /// before the error is returned.
    pub max_retries: u32,
}

impl Default for WaitOptions {
    fn default() -> Self {
        Self {
            initial_interval: Duration::from_secs(2),
            max_interval: Duration::from_secs(15),
            max_wait: None,
            max_retries: 5,
        }
    }
}

/// Client for one AI-DB instance.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base: Url,
    token: Option<String>,
    timeout: Duration,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Normalised base URL of the server (always ends with `/`).
    pub fn server(&self) -> &Url {
        &self.base
    }

    /// `true` if an owner token is configured.
    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    /// Link to a job in the web interface.
    pub fn job_web_url(&self, job_id: &str) -> Url {
        self.segments_url(&["job", job_id])
    }

    // ── Service ──────────────────────────────────────────────────────────────

    /// `GET /api/health` – service status and API version.
    pub async fn health(&self) -> Result<HealthCheckResponse> {
        let url = self.endpoint("api/health")?;
        self.send_json(&url, self.request(self.http.get(url.clone()), Auth::None))
            .await
    }

    /// `GET /api/db/info` – Bakta DB release and AI-DB annotations DB status.
    pub async fn db_info(&self) -> Result<DbInfoResponse> {
        let url = self.endpoint("api/db/info")?;
        self.send_json(&url, self.request(self.http.get(url.clone()), Auth::None))
            .await
    }

    // ── Jobs ─────────────────────────────────────────────────────────────────

    /// `POST /api/job/` – uploads a FASTA file (plain or gzipped, streamed)
    /// as a new job owned by the configured token.
    ///
    /// `on_progress` is called with the number of bytes sent per chunk.
    /// Without a token the server creates a new, anonymous owner whose jobs
    /// cannot be managed afterwards – callers should require a token.
    pub async fn submit_fasta<F>(
        &self,
        path: &Path,
        job_name: Option<&str>,
        on_progress: F,
    ) -> Result<JobCreateResponse>
    where
        F: Fn(u64) + Send + Sync + 'static,
    {
        let file_error = |source| Error::File {
            path: path.to_path_buf(),
            source,
        };
        let file = tokio::fs::File::open(path).await.map_err(file_error)?;
        let length = file.metadata().await.map_err(file_error)?.len();

        let stream =
            ReaderStream::new(file).inspect_ok(move |chunk| on_progress(chunk.len() as u64));
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "input.fasta".to_string());
        let part = Part::stream_with_length(Body::wrap_stream(stream), length)
            .file_name(file_name)
            .mime_str("application/octet-stream")
            .map_err(Error::Http)?;

        let mut form = Form::new().part("file", part);
        if let Some(name) = job_name.filter(|n| !n.trim().is_empty()) {
            form = form.text("job_name", name.to_string());
        }

        let url = self.endpoint("api/job/")?;
        let request = self
            .request(self.http.post(url.clone()), Auth::Owner)
            .multipart(form)
            .timeout(UPLOAD_TIMEOUT);
        self.send_json(&url, request).await
    }

    /// `GET /api/jobs/` – one page of the jobs owned by the configured token
    /// (newest first). Without a token the list is empty.
    pub async fn list_jobs(&self, query: &ListJobsQuery) -> Result<PaginatedJobsResponse> {
        let url = self.endpoint("api/jobs/")?;
        let request = self
            .request(self.http.get(url.clone()), Auth::Owner)
            .query(query);
        self.send_json(&url, request).await
    }

    /// `GET /api/job/{job_id}/download/{format}` – streams the result file of
    /// a completed job into `writer` and returns the number of bytes written.
    /// Owner only.
    pub async fn download<W>(
        &self,
        job_id: &str,
        format: ResultFormat,
        filter: &SequenceFilterQuery,
        writer: &mut W,
    ) -> Result<u64>
    where
        W: AsyncWrite + Unpin + ?Sized,
    {
        let url = self.segments_url(&["api", "job", job_id, "download", format.as_str()]);
        // Result files are built on the server first; allow for large jobs
        let request = self
            .request(self.http.get(url.clone()), Auth::Owner)
            .query(filter)
            .timeout(UPLOAD_TIMEOUT);
        let response = request
            .send()
            .await
            .map_err(|e| Error::from_reqwest(url.as_str(), e))?;
        let mut response = check_status(response).await?;

        let mut written = 0u64;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| Error::from_reqwest(url.as_str(), e))?
        {
            writer.write_all(&chunk).await.map_err(Error::Write)?;
            written += chunk.len() as u64;
        }
        writer.flush().await.map_err(Error::Write)?;
        Ok(written)
    }

    /// `GET /api/job/{job_id}` with the given page / filter.
    pub async fn job(&self, job_id: &str, query: &GetJobQuery) -> Result<JobViewResponse> {
        let url = self.segments_url(&["api", "job", job_id]);
        let request = self
            .request(self.http.get(url.clone()), Auth::Owner)
            .query(query);
        self.send_json(&url, request).await
    }

    /// Status and counters of a job, without the sequence list (one entry at
    /// most, to keep polling cheap).
    pub async fn job_status(&self, job_id: &str) -> Result<JobViewResponse> {
        let query = GetJobQuery {
            per_page: Some(1),
            ..GetJobQuery::default()
        };
        self.job(job_id, &query).await
    }

    /// Number of sequences of a job matching a match-status filter
    /// (`bakta_db`, `aidb_db`, `hash_match`, `none`, …).
    pub async fn count_sequences(&self, job_id: &str, filter: &str) -> Result<usize> {
        let query = GetJobQuery {
            per_page: Some(1),
            filter: Some(filter.to_string()),
            ..GetJobQuery::default()
        };
        Ok(self.job(job_id, &query).await?.job.filtered_count)
    }

    /// Polls a job until it is `completed` or `failed` and returns its final
    /// state. `on_update` sees every polled state (for progress output).
    ///
    /// Temporary failures are retried (see [`WaitOptions::max_retries`]); a
    /// failed job is not an error here – check `status` of the result.
    pub async fn wait_for_job<F>(
        &self,
        job_id: &str,
        options: &WaitOptions,
        mut on_update: F,
    ) -> Result<JobViewResponse>
    where
        F: FnMut(&JobViewResponse),
    {
        let started = tokio::time::Instant::now();
        let mut interval = options.initial_interval;
        let mut failures = 0;

        loop {
            match self.job_status(job_id).await {
                Ok(view) => {
                    failures = 0;
                    on_update(&view);
                    if matches!(view.job.status, JobStatus::Completed | JobStatus::Failed) {
                        return Ok(view);
                    }
                }
                Err(error) if error.is_temporary() && failures < options.max_retries => {
                    failures += 1;
                }
                Err(error) => return Err(error),
            }

            if let Some(max_wait) = options.max_wait {
                let waited = started.elapsed();
                if waited >= max_wait {
                    return Err(Error::WaitTimeout {
                        job_id: job_id.to_string(),
                        waited,
                    });
                }
                // Do not sleep past the deadline
                interval = interval.min(max_wait - waited);
            }
            tokio::time::sleep(interval).await;
            interval = interval.mul_f64(1.5).min(options.max_interval);
        }
    }

    // ── Internals ────────────────────────────────────────────────────────────

    fn endpoint(&self, path: &str) -> Result<Url> {
        self.base.join(path).map_err(|e| Error::InvalidUrl {
            url: format!("{}{}", self.base, path),
            reason: e.to_string(),
        })
    }

    /// URL below the base with each segment percent-encoded, so IDs can never
    /// change the path (`../`, `?`, …).
    fn segments_url(&self, segments: &[&str]) -> Url {
        let mut url = self.base.clone();
        url.path_segments_mut()
            .expect("normalize_server_url guarantees a base URL")
            .pop_if_empty()
            .extend(segments);
        url
    }

    /// Default timeout plus the token on owner routes.
    fn request(&self, request: RequestBuilder, auth: Auth) -> RequestBuilder {
        let request = request.timeout(self.timeout);
        match (&self.token, auth) {
            (Some(token), Auth::Owner) => request.bearer_auth(token),
            _ => request,
        }
    }

    async fn send_json<T: DeserializeOwned>(
        &self,
        url: &Url,
        request: RequestBuilder,
    ) -> Result<T> {
        let response = request
            .send()
            .await
            .map_err(|e| Error::from_reqwest(url.as_str(), e))?;
        let response = check_status(response).await?;
        response
            .json::<T>()
            .await
            .map_err(|e| Error::from_reqwest(url.as_str(), e))
    }
}

/// Parses a server URL into the canonical form used by [`Client`]:
/// http(s) only, no query or fragment, path always ending with `/`.
///
/// Useful as a stable key for per-server settings (e.g. saved tokens), since
/// `https://host`, `https://host/` and `https://host/?x` map to the same value.
pub fn normalize_server_url(raw: &str) -> Result<Url> {
    let invalid = |reason: &str| Error::InvalidUrl {
        url: raw.to_string(),
        reason: reason.to_string(),
    };
    let mut url = Url::parse(raw.trim()).map_err(|e| invalid(&e.to_string()))?;
    if url.scheme() != "https" && url.scheme() != "http" {
        return Err(invalid("only http:// and https:// are supported"));
    }
    if url.cannot_be_a_base() || url.host_str().is_none() {
        return Err(invalid("missing host"));
    }
    url.set_query(None);
    url.set_fragment(None);
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

/// Turns a non-2xx response into [`Error::Api`].
async fn check_status(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    Err(api_error(status.as_u16(), &body))
}

/// Builds [`Error::Api`] from a status and the response body: the `detail`
/// of an `ErrorResponse`, otherwise the (shortened) raw body or the status.
fn api_error(status: u16, body: &str) -> Error {
    let detail = match serde_json::from_str::<ErrorResponse>(body) {
        Ok(error) if !error.detail.trim().is_empty() => error.detail,
        _ => {
            let text = body.trim();
            if text.is_empty() {
                reqwest::StatusCode::from_u16(status)
                    .ok()
                    .and_then(|s| s.canonical_reason())
                    .unwrap_or("request failed")
                    .to_string()
            } else {
                text.chars().take(MAX_DETAIL_CHARS).collect()
            }
        }
    };
    Error::Api { status, detail }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_gets_a_trailing_slash_and_loses_query() {
        let url = normalize_server_url("https://ai-db.example.org?x=1#top").unwrap();
        assert_eq!(url.as_str(), "https://ai-db.example.org/");
    }

    #[test]
    fn sub_path_deployments_are_kept() {
        let client = Client::builder()
            .server("https://example.org/ai-db")
            .build()
            .unwrap();
        assert_eq!(
            client.endpoint("api/health").unwrap().as_str(),
            "https://example.org/ai-db/api/health"
        );
    }

    #[test]
    fn unsupported_or_broken_urls_are_rejected() {
        for bad in [
            "ftp://example.org",
            "ai-db.example.org",
            "https://",
            "mailto:x@y.z",
        ] {
            assert!(
                matches!(normalize_server_url(bad), Err(Error::InvalidUrl { .. })),
                "{bad}"
            );
        }
    }

    #[test]
    fn ids_are_encoded_as_single_path_segments() {
        let client = Client::builder()
            .server("https://example.org/ai-db/")
            .build()
            .unwrap();
        assert_eq!(
            client.segments_url(&["api", "job", "abc"]).as_str(),
            "https://example.org/ai-db/api/job/abc"
        );
        assert_eq!(
            client.segments_url(&["api", "job", "../admin?x"]).as_str(),
            "https://example.org/ai-db/api/job/..%2Fadmin%3Fx"
        );
        assert_eq!(
            client.job_web_url("abc").as_str(),
            "https://example.org/ai-db/job/abc"
        );
    }

    #[test]
    fn empty_token_counts_as_no_token() {
        let client = Client::builder().token(Some("  ".into())).build().unwrap();
        assert!(!client.has_token());
    }

    #[test]
    fn api_error_prefers_the_server_detail() {
        let err = api_error(
            403,
            r#"{"detail":"Only the owner of this job may modify it"}"#,
        );
        assert_eq!(err.status(), Some(403));
        assert_eq!(
            err.to_string(),
            "Only the owner of this job may modify it (HTTP 403)"
        );
    }

    #[test]
    fn api_error_falls_back_to_body_or_reason() {
        assert_eq!(
            api_error(502, "<html>Bad Gateway</html>").to_string(),
            "<html>Bad Gateway</html> (HTTP 502)"
        );
        assert_eq!(api_error(404, "").to_string(), "Not Found (HTTP 404)");
        let long = "x".repeat(1000);
        match api_error(500, &long) {
            Error::Api { detail, .. } => assert_eq!(detail.len(), MAX_DETAIL_CHARS),
            other => panic!("unexpected {other:?}"),
        }
    }
}
