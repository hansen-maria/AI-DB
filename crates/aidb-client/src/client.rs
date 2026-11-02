//! HTTP client for the AI-DB REST API.

use std::time::Duration;

use aidb_types::{DbInfoResponse, ErrorResponse, HealthCheckResponse};
use reqwest::Url;
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};

/// Public AI-DB instance, used when no server is configured.
pub const DEFAULT_SERVER: &str = "https://ai-db.computational.bio";

/// Default timeout for a whole request.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Upper bound for establishing the connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Longest server error text passed on to the user.
const MAX_DETAIL_CHARS: usize = 300;

/// Whether a request carries the owner token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Auth {
    /// Public endpoint: the token is never sent.
    None,
    /// The token is sent if one is configured.
    #[allow(dead_code)] // first used by the job endpoints (next step)
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

    /// Timeout for a whole request (connecting is capped at 10 s).
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
        let base = normalize_base_url(&self.server)?;
        let http = reqwest::Client::builder()
            .user_agent(self.user_agent)
            .timeout(self.timeout)
            .connect_timeout(self.timeout.min(CONNECT_TIMEOUT))
            .build()
            .map_err(Error::Http)?;
        Ok(Client {
            http,
            base,
            token: self.token,
        })
    }
}

/// Client for one AI-DB instance.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base: Url,
    token: Option<String>,
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

    /// `GET /api/health` – service status and API version.
    pub async fn health(&self) -> Result<HealthCheckResponse> {
        self.get_json("api/health", Auth::None).await
    }

    /// `GET /api/db/info` – Bakta DB release and AI-DB annotations DB status.
    pub async fn db_info(&self) -> Result<DbInfoResponse> {
        self.get_json("api/db/info", Auth::None).await
    }

    // ── Internals ────────────────────────────────────────────────────────────

    fn endpoint(&self, path: &str) -> Result<Url> {
        self.base.join(path).map_err(|e| Error::InvalidUrl {
            url: format!("{}{}", self.base, path),
            reason: e.to_string(),
        })
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str, auth: Auth) -> Result<T> {
        let url = self.endpoint(path)?;
        let mut request = self.http.get(url.clone());
        if auth == Auth::Owner {
            if let Some(token) = &self.token {
                request = request.bearer_auth(token);
            }
        }
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

/// Parses the server URL and makes it usable as a base for relative joins.
fn normalize_base_url(raw: &str) -> Result<Url> {
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
        let url = normalize_base_url("https://ai-db.example.org?x=1#top").unwrap();
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
                matches!(normalize_base_url(bad), Err(Error::InvalidUrl { .. })),
                "{bad}"
            );
        }
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
