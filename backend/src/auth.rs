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
//! Every state-changing operation (delete, rename, retry, Psos/Bakta state,
//! ingest) and downloads require the owner identity (`authorize_owner`).
//! ============================================================================

use axum::{
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
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
    format!("{:x}", md5::compute(format!("aidb-owner:{secret}").as_bytes()))
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
        let new_id = Uuid::new_v4().to_string();
        let cookie = Cookie::build((OWNER_COOKIE_NAME, new_id.clone()))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .max_age(time::Duration::days(COOKIE_MAX_AGE_DAYS))
            .build();
        // The cookie carries the secret; only its digest becomes the owner id
        Ok((owner_digest(&new_id), jar.add(cookie)))
    }
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
/// Read-only endpoints (viewing a job, sequences, stats, Psos/Bakta state) do
/// not call this, so a shared job ID gives view access only.
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

/// Pseudonymous, stable identifier of a contributor for the shared annotations
/// DB, derived from the (already hashed) owner id with a different domain
/// separator, so contributions cannot be linked to jobs or owners.
pub fn contributor_id(owner_id: &str) -> String {
    format!("{:x}", md5::compute(format!("aidb-contributor:{owner_id}").as_bytes()))
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
        assert_ne!(owner, secret, "the secret itself must not become the owner id");
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
}
