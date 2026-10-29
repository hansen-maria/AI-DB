//! ============================================================================
//! Authentication and owner management
//!
//! A job owner is identified by a random UUID (v4). Two transports are accepted:
//!
//!   1. Cookie `ai_db_user` (browsers; HttpOnly, set automatically on first job
//!      submission).
//!   2. API token (scripts, workflow systems, pipelines):
//!        Authorization: Bearer <uuid-v4>
//!      or
//!        X-API-Key: <uuid-v4>
//!      API clients generate the token themselves (any random UUIDv4, e.g.
//!      `uuidgen`) and send it with every request. No cookie handling needed.
//!
//! If both are present, the token takes precedence over the cookie.
//!
//! The secret (cookie value / API token) is never stored. Everything that is
//! persisted or compared (`jobs.owner_id`, KPI owner counts, contributor ids)
//! uses its one-way digest (`owner_digest`), so a database leak or backup does
//! not expose credentials. Existing rows are migrated at startup.
//!
//! Sharing model: anyone who knows a job ID may *view* the job (read-only).
//! Every state-changing operation (delete, rename, retry, saving Psos/Bakta
//! state or starting a run, ingest), downloads and the stats CSV export require
//! the owner identity.
//! This is enforced centrally by the `require_owner_for_writes` middleware
//! (see `owner_guarded_job_id`) in addition to the handlers.
//!
//! Admin routes (`/api/admin/*`) are a separate realm: they are authorized
//! *only* by the shared secret in `X-Admin-Secret` (`require_admin_secret`).
//! Owner tokens/cookies never grant admin access, and the admin secret never
//! grants owner access (it is not read by any owner function).
//! ============================================================================

use axum::{
    extract::{Request, State},
    http::{
        header::{AUTHORIZATION, HOST, ORIGIN},
        HeaderMap, Method, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use uuid::Uuid;

use crate::models::ErrorResponse;
use crate::state::AppState;

pub const OWNER_COOKIE_NAME: &str = "ai_db_user";
const COOKIE_MAX_AGE_DAYS: i64 = 365; // 1 year

/// Alternative header for clients that cannot send `Authorization: Bearer`.
pub const API_KEY_HEADER: &str = "x-api-key";

/// Header carrying the shared admin secret (admin routes only).
pub const ADMIN_SECRET_HEADER: &str = "x-admin-secret";

/// Environment variable holding the shared admin secret.
pub const ADMIN_SECRET_ENV: &str = "ADMIN_KPI_SECRET";

/// Returned when an API token was supplied but is not a valid UUIDv4.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidToken;

/// One-way digest of an owner secret (cookie value or API token): this is the
/// owner identity that is stored in the databases and compared in handlers.
///
/// The secrets are random 122-bit UUIDv4 values, so a fast digest is sufficient
/// (no brute-force or dictionary attack applies); the digest is 32 lowercase hex
/// characters (never contains '-', unlike a raw UUID, which the one-time
/// migration relies on to recognise unmigrated rows).
pub fn owner_digest(secret: &str) -> String {
    format!(
        "{:x}",
        md5::compute(format!("aidb-owner:{secret}").as_bytes())
    )
}

/// Extracts the owner token from the request headers.
///
/// * `Ok(Some(id))` – a valid token was supplied (normalised to lowercase,
///   hyphenated form, i.e. the same format as the cookie value).
/// * `Ok(None)`     – no token header present.
/// * `Err(..)`      – a token header is present but malformed. Only UUIDv4 values
///   are accepted so trivially guessable identifiers (e.g. the nil UUID,
///   "admin") can never become an owner identity.
pub fn token_from_headers(headers: &HeaderMap) -> Result<Option<String>, InvalidToken> {
    let raw = if let Some(value) = headers.get(AUTHORIZATION) {
        let value = value.to_str().map_err(|_| InvalidToken)?.trim();
        let (scheme, token) = value.split_once(' ').ok_or(InvalidToken)?;
        if !scheme.eq_ignore_ascii_case("bearer") {
            // Other schemes (e.g. Basic) are not supported by this service.
            return Err(InvalidToken);
        }
        token.trim().to_string()
    } else if let Some(value) = headers.get(API_KEY_HEADER) {
        value.to_str().map_err(|_| InvalidToken)?.trim().to_string()
    } else {
        return Ok(None);
    };

    match Uuid::parse_str(&raw) {
        Ok(uuid) if uuid.get_version_num() == 4 => Ok(Some(uuid.to_string())),
        _ => Err(InvalidToken),
    }
}

/// Returns the owner_id (digest) of the requester, if any (read-only; never creates one).
///
/// Token header first, then cookie. A malformed token yields `None`
/// (= anonymous) and never falls back to the cookie.
pub fn owner_from_request(jar: &CookieJar, headers: &HeaderMap) -> Option<String> {
    match token_from_headers(headers) {
        Ok(Some(token)) => Some(owner_digest(&token)),
        Ok(None) => jar.get(OWNER_COOKIE_NAME).map(|c| owner_digest(c.value())),
        Err(InvalidToken) => None,
    }
}

/// Returns the owner_id (digest) of the requester or creates a new one.
///
/// * Valid API token  → used as owner; **no cookie is set**.
/// * Malformed token  → `Err(InvalidToken)` (caller should answer 401).
/// * Cookie present   → cookie value is used.
/// * Nothing present  → a new owner is created and set as cookie (browser flow).
pub fn get_or_create_owner(
    jar: CookieJar,
    headers: &HeaderMap,
) -> Result<(String, CookieJar), InvalidToken> {
    if let Some(token) = token_from_headers(headers)? {
        return Ok((owner_digest(&token), jar));
    }

    if let Some(cookie) = jar.get(OWNER_COOKIE_NAME) {
        Ok((owner_digest(cookie.value()), jar))
    } else {
        // The cookie carries the secret; only its digest becomes the owner id
        let (new_id, jar) = add_new_owner_cookie(jar);
        Ok((owner_digest(&new_id), jar))
    }
}

/// Creates a new owner secret (UUIDv4) and adds it to the jar as the
/// HttpOnly owner cookie. Returns the secret and the updated jar.
pub fn add_new_owner_cookie(jar: CookieJar) -> (String, CookieJar) {
    let new_id = Uuid::new_v4().to_string();
    let cookie = Cookie::build((OWNER_COOKIE_NAME, new_id.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::days(COOKIE_MAX_AGE_DAYS))
        .build();
    (new_id, jar.add(cookie))
}

/// The owner secret stored in the browser cookie, if it can be used as an API
/// token for the same owner.
///
/// The owner digest is computed from the raw cookie value, while API tokens
/// are normalised to the lowercase, hyphenated UUID form before hashing. The
/// cookie is therefore only exportable if it already is a UUIDv4 in exactly
/// that form – which is always the case for cookies created by this server.
///
/// * `Ok(Some(token))` – cookie present and usable as an API token
/// * `Ok(None)`        – no owner cookie in this browser
/// * `Err(..)`         – cookie present but not a canonical UUIDv4, so as an
///   API token it would identify a different owner
pub fn owner_secret_from_cookie(jar: &CookieJar) -> Result<Option<String>, InvalidToken> {
    let Some(cookie) = jar.get(OWNER_COOKIE_NAME) else {
        return Ok(None);
    };
    let value = cookie.value();
    match Uuid::parse_str(value) {
        Ok(uuid) if uuid.get_version_num() == 4 && uuid.to_string() == value => {
            Ok(Some(value.to_string()))
        }
        _ => Err(InvalidToken),
    }
}

/// `true` if the browser marks the request as coming from the same origin
/// (the AI-DB frontend itself), `false` for any cross-origin request.
///
/// Endpoints that return the owner secret must only answer same-origin
/// requests: CORS mirrors any origin with credentials, and the `Lax` cookie is
/// also sent by pages of the same *site* (other `*.computational.bio`
/// subdomains), so CORS and SameSite alone would let such a page read it.
///
/// * `Sec-Fetch-Site` (all current browsers): only `same-origin`, or `none`
///   (the user opened the URL directly), is accepted.
/// * Older browsers without it: a cross-origin request always carries an
///   `Origin` header, whose host must then match the `Host` header. Without
///   either header the request is not a cross-origin browser request.
pub fn is_same_origin_request(headers: &HeaderMap) -> bool {
    if let Some(site) = headers.get("sec-fetch-site") {
        return match site.to_str() {
            Ok(site) => {
                site.eq_ignore_ascii_case("same-origin") || site.eq_ignore_ascii_case("none")
            }
            Err(_) => false,
        };
    }

    let Some(origin) = headers.get(ORIGIN) else {
        return true;
    };
    let (Ok(origin), Some(Ok(host))) = (origin.to_str(), headers.get(HOST).map(|h| h.to_str()))
    else {
        return false;
    };
    // "https://ai-db.example.org:8443" -> "ai-db.example.org"
    let origin_host = origin
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("");
    !origin_host.is_empty() && hostname(origin_host).eq_ignore_ascii_case(hostname(host))
}

/// Host without port ("example.org:8080" -> "example.org", "[::1]:80" -> "[::1]").
fn hostname(host: &str) -> &str {
    if host.starts_with('[') {
        return host.split_inclusive(']').next().unwrap_or(host);
    }
    host.split(':').next().unwrap_or(host)
}

/// Validates if the given owner_id matches the job's owner
pub fn validate_owner(job_owner: Option<&String>, cookie_owner: Option<&String>) -> bool {
    match (job_owner, cookie_owner) {
        (Some(job_owner), Some(cookie_owner)) => job_owner == cookie_owner,
        _ => false,
    }
}

/// Guard for state-changing endpoints: the job must exist and the requester
/// must be its owner (API token or cookie). Returns the owner id.
///
/// * unknown job            → 404
/// * no / foreign identity  → 403 (also for legacy jobs without an owner)
///
/// Read-only endpoints (viewing a job, sequences, stats, reading Psos/Bakta
/// state) do not call this, so a shared job ID gives view access only.
pub fn authorize_owner(
    state: &AppState,
    job_id: &str,
    jar: &CookieJar,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let requester = owner_from_request(jar, headers);
    let jobs = state.jobs();
    match jobs.get(job_id) {
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!("Job not found: {job_id}"))),
        )),
        Some(job) if validate_owner(job.owner_id.as_ref(), requester.as_ref()) => {
            Ok(requester.clone().unwrap_or_default())
        }
        Some(_) => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "Only the owner of this job may modify it (shared jobs are read-only)",
            )),
        )),
    }
}

/// Single source of truth for which job requests are owner-only.
///
/// Returns the job ID if `method` + `path` on `/api/job/{job_id}/...` requires
/// the owner, `None` otherwise. Owner-only are:
///
/// * every state-changing method (POST/PUT/PATCH/DELETE) on a job and its
///   sub-resources – delete, rename, retry, saving/deleting Psos and Bakta
///   state (i.e. starting a run for the job), Bakta ingest;
/// * downloads (`GET .../download/{format}`) and the stats CSV export
///   (`GET .../stats/export`).
///
/// Job creation (`POST /api/job/`) has no job ID and is not guarded. Works on
/// concrete paths and on OpenAPI path templates (`{job_id}`), so the OpenAPI
/// security requirements are derived from the same rules.
pub fn owner_guarded_job_id<'a>(method: &Method, path: &'a str) -> Option<&'a str> {
    let rest = path.strip_prefix("/api/job/")?;
    let (job_id, sub) = rest.split_once('/').unwrap_or((rest, ""));
    if job_id.is_empty() {
        return None;
    }
    let guarded = match *method {
        Method::GET | Method::HEAD => {
            sub == "download" || sub.starts_with("download/") || sub == "stats/export"
        }
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE => true,
        _ => false,
    };
    guarded.then_some(job_id)
}

/// Middleware: rejects non-owners on every owner-only job route
/// (see `owner_guarded_job_id`) before the handler runs, so shared jobs are
/// strictly read-only even if a handler forgets its own check.
pub async fn require_owner_for_writes(
    State(state): State<AppState>,
    jar: CookieJar,
    req: Request,
    next: Next,
) -> Response {
    if let Some(job_id) = owner_guarded_job_id(req.method(), req.uri().path()) {
        if let Err(rejection) = authorize_owner(&state, job_id, &jar, req.headers()) {
            return rejection.into_response();
        }
    }
    next.run(req).await
}

/// Middleware for `/api/admin/*`: only the shared secret in `X-Admin-Secret`
/// authorizes. Owner tokens (`Authorization`, `X-API-Key`) and cookies are
/// ignored here, so they can never grant admin access.
///
/// * secret not configured → 503 (admin routes disabled)
/// * missing / wrong secret → 401
pub async fn require_admin_secret(req: Request, next: Next) -> Response {
    let Some(expected) = std::env::var(ADMIN_SECRET_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty())
    else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(format!(
                "Admin routes are disabled ({ADMIN_SECRET_ENV} not configured)"
            ))),
        )
            .into_response();
    };

    if !admin_secret_matches(req.headers(), expected.trim()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("Missing or invalid X-Admin-Secret")),
        )
            .into_response();
    }
    next.run(req).await
}

/// Checks the `X-Admin-Secret` header (and only that header) against the
/// expected secret in constant time.
fn admin_secret_matches(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(ADMIN_SECRET_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|provided| constant_time_eq(provided.trim().as_bytes(), expected.as_bytes()))
        .unwrap_or(false)
}

/// Constant-time comparison (only the length may leak).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Pseudonymous, stable identifier of a contributor for the shared annotations
/// DB, derived from the (already hashed) owner id with a different domain
/// separator, so contributions cannot be linked to jobs or owners.
pub fn contributor_id(owner_id: &str) -> String {
    format!(
        "{:x}",
        md5::compute(format!("aidb-contributor:{owner_id}").as_bytes())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    const TOKEN: &str = "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c";

    fn headers(name: &'static str, value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(name, HeaderValue::from_str(value).unwrap());
        h
    }

    #[test]
    fn same_origin_by_sec_fetch_site() {
        assert!(is_same_origin_request(&headers(
            "sec-fetch-site",
            "same-origin"
        )));
        assert!(is_same_origin_request(&headers("sec-fetch-site", "none")));
        for value in ["same-site", "cross-site", "garbage"] {
            assert!(
                !is_same_origin_request(&headers("sec-fetch-site", value)),
                "{value}"
            );
        }
    }

    #[test]
    fn sec_fetch_site_wins_over_matching_origin() {
        let mut h = headers("sec-fetch-site", "same-site");
        h.insert(
            "origin",
            HeaderValue::from_static("https://ai-db.example.org"),
        );
        h.insert("host", HeaderValue::from_static("ai-db.example.org"));
        assert!(!is_same_origin_request(&h));
    }

    #[test]
    fn same_origin_by_origin_and_host_without_fetch_metadata() {
        let mut same = headers("origin", "https://ai-db.example.org");
        same.insert("host", HeaderValue::from_static("ai-db.example.org"));
        assert!(is_same_origin_request(&same));

        // Vite dev proxy: frontend on :8080, Host rewritten to the backend port
        let mut dev = headers("origin", "http://localhost:8080");
        dev.insert("host", HeaderValue::from_static("localhost:8000"));
        assert!(is_same_origin_request(&dev));

        let mut sibling = headers("origin", "https://psos.example.org");
        sibling.insert("host", HeaderValue::from_static("ai-db.example.org"));
        assert!(!is_same_origin_request(&sibling));

        let mut null_origin = headers("origin", "null");
        null_origin.insert("host", HeaderValue::from_static("ai-db.example.org"));
        assert!(!is_same_origin_request(&null_origin));

        // Origin present but no Host header: cannot verify -> reject
        assert!(!is_same_origin_request(&headers(
            "origin",
            "https://ai-db.example.org"
        )));
    }

    #[test]
    fn requests_without_origin_information_are_not_cross_origin() {
        assert!(is_same_origin_request(&HeaderMap::new()));
    }

    #[test]
    fn owner_secret_from_cookie_accepts_only_canonical_uuid_v4() {
        let jar = CookieJar::new();
        assert_eq!(owner_secret_from_cookie(&jar), Ok(None));

        let jar = CookieJar::new().add(Cookie::new(OWNER_COOKIE_NAME, TOKEN));
        assert_eq!(owner_secret_from_cookie(&jar), Ok(Some(TOKEN.to_string())));

        // As an API token it would be normalised and hash to another owner
        let jar = CookieJar::new().add(Cookie::new(OWNER_COOKIE_NAME, TOKEN.to_uppercase()));
        assert_eq!(owner_secret_from_cookie(&jar), Err(InvalidToken));

        let jar = CookieJar::new().add(Cookie::new(OWNER_COOKIE_NAME, "not-a-uuid"));
        assert_eq!(owner_secret_from_cookie(&jar), Err(InvalidToken));
    }

    #[test]
    fn new_owner_cookie_is_http_only_and_matches_the_secret() {
        let (secret, jar) = add_new_owner_cookie(CookieJar::new());
        let cookie = jar.get(OWNER_COOKIE_NAME).expect("cookie set");
        assert_eq!(cookie.value(), secret);
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(owner_secret_from_cookie(&jar), Ok(Some(secret)));
    }

    #[test]
    fn bearer_token_is_accepted_case_insensitive_scheme() {
        let h = headers("authorization", &format!("bearer {TOKEN}"));
        assert_eq!(token_from_headers(&h), Ok(Some(TOKEN.to_string())));
    }

    #[test]
    fn api_key_header_is_accepted_and_normalised() {
        let h = headers("x-api-key", &TOKEN.to_uppercase());
        assert_eq!(token_from_headers(&h), Ok(Some(TOKEN.to_string())));
    }

    #[test]
    fn no_header_means_no_token() {
        assert_eq!(token_from_headers(&HeaderMap::new()), Ok(None));
    }

    #[test]
    fn malformed_tokens_are_rejected() {
        for bad in [
            "Bearer admin",
            "Bearer 00000000-0000-0000-0000-000000000000", // nil UUID
            "Bearer 3f2b8c1e-5d4a-1e7b-9a6c-1d2e3f4a5b6c", // not version 4
            "Basic dXNlcjpwYXNz",
            "Bearer",
        ] {
            let h = headers("authorization", bad);
            assert_eq!(token_from_headers(&h), Err(InvalidToken), "{bad}");
        }
    }

    #[test]
    fn token_takes_precedence_over_cookie() {
        let jar = CookieJar::new().add(Cookie::new(OWNER_COOKIE_NAME, "cookie-owner"));
        let h = headers("authorization", &format!("Bearer {TOKEN}"));
        assert_eq!(owner_from_request(&jar, &h), Some(owner_digest(TOKEN)));
    }

    #[test]
    fn cookie_is_used_without_token() {
        let jar = CookieJar::new().add(Cookie::new(OWNER_COOKIE_NAME, "cookie-owner"));
        assert_eq!(
            owner_from_request(&jar, &HeaderMap::new()),
            Some(owner_digest("cookie-owner"))
        );
    }

    #[test]
    fn malformed_token_never_falls_back_to_cookie() {
        let jar = CookieJar::new().add(Cookie::new(OWNER_COOKIE_NAME, "cookie-owner"));
        let h = headers("authorization", "Bearer admin");
        assert_eq!(owner_from_request(&jar, &h), None);
    }

    #[test]
    fn admin_secret_does_not_grant_owner_identity() {
        let h = headers("x-admin-secret", TOKEN);
        assert_eq!(owner_from_request(&CookieJar::new(), &h), None);
    }

    #[test]
    fn token_flow_does_not_set_a_cookie() {
        let h = headers("x-api-key", TOKEN);
        let (owner, jar) = get_or_create_owner(CookieJar::new(), &h).unwrap();
        assert_eq!(owner, owner_digest(TOKEN));
        assert!(jar.get(OWNER_COOKIE_NAME).is_none());
    }

    #[test]
    fn browser_flow_still_creates_cookie() {
        let (owner, jar) = get_or_create_owner(CookieJar::new(), &HeaderMap::new()).unwrap();
        let secret = jar.get(OWNER_COOKIE_NAME).unwrap().value().to_string();
        assert_eq!(owner, owner_digest(&secret));
        assert_ne!(
            owner, secret,
            "the secret itself must not become the owner id"
        );
    }

    #[test]
    fn owner_digest_is_stable_hex_without_hyphens() {
        let a = owner_digest(TOKEN);
        assert_eq!(a, owner_digest(TOKEN));
        assert_ne!(a, owner_digest("other"));
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(!a.contains('-') && !a.contains(TOKEN));
    }

    #[test]
    fn contributor_id_is_stable_and_hides_the_token() {
        let a = contributor_id(TOKEN);
        assert_eq!(a, contributor_id(TOKEN));
        assert_ne!(a, contributor_id("someone-else"));
        assert!(!a.contains(TOKEN));
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn invalid_token_on_create_is_an_error() {
        let h = headers("authorization", "Bearer admin");
        assert!(get_or_create_owner(CookieJar::new(), &h).is_err());
    }

    #[test]
    fn writes_and_runs_on_jobs_are_owner_only() {
        for (method, path) in [
            (Method::DELETE, "/api/job/abc"),
            (Method::PATCH, "/api/job/abc"),
            (Method::POST, "/api/job/abc/retry"),
            (Method::POST, "/api/job/abc/psos"),
            (Method::DELETE, "/api/job/abc/psos"),
            (Method::POST, "/api/job/abc/bakta"),
            (Method::DELETE, "/api/job/abc/bakta"),
            (Method::POST, "/api/job/abc/bakta/ingest"),
            (Method::GET, "/api/job/abc/download/tsv"),
            (Method::GET, "/api/job/abc/stats/export"),
        ] {
            assert_eq!(
                owner_guarded_job_id(&method, path),
                Some("abc"),
                "{method} {path}"
            );
        }
    }

    #[test]
    fn reads_and_creation_are_not_owner_guarded() {
        for (method, path) in [
            (Method::POST, "/api/job/"),
            (Method::GET, "/api/job/abc"),
            (Method::GET, "/api/job/abc/stats"),
            (Method::GET, "/api/job/abc/psos"),
            (Method::GET, "/api/job/abc/bakta"),
            (Method::GET, "/api/job/abc/sequence/1"),
            (Method::OPTIONS, "/api/job/abc"),
            (Method::DELETE, "/api/jobs/"),
            (Method::GET, "/api/admin/kpis"),
        ] {
            assert_eq!(owner_guarded_job_id(&method, path), None, "{method} {path}");
        }
    }

    #[test]
    fn guard_works_on_openapi_templates() {
        assert_eq!(
            owner_guarded_job_id(&Method::GET, "/api/job/{job_id}/download/{format}"),
            Some("{job_id}")
        );
    }

    #[test]
    fn only_admin_header_matches_admin_secret() {
        let secret = "s3cr3t-admin";
        assert!(admin_secret_matches(
            &headers("x-admin-secret", secret),
            secret
        ));
        assert!(!admin_secret_matches(
            &headers("x-admin-secret", "wrong"),
            secret
        ));
        assert!(!admin_secret_matches(&headers("x-api-key", secret), secret));
        assert!(!admin_secret_matches(
            &headers("authorization", &format!("Bearer {secret}")),
            secret
        ));
        assert!(!admin_secret_matches(&HeaderMap::new(), secret));
    }
}
