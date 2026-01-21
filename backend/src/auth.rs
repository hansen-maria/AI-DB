//! ============================================================================
//! Authentication and owner management via cookies
//! ============================================================================

use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use uuid::Uuid;

pub const OWNER_COOKIE_NAME: &str = "ai_db_user";
const COOKIE_MAX_AGE_DAYS: i64 = 365; // 1 year

/// Returns the owner_id from the cookie jar, or creates a new one if it doesn't exist
pub fn get_or_create_owner(jar: CookieJar) -> (String, CookieJar) {
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

/// Validates if the given owner_id matches the job's owner
pub fn validate_owner(job_owner: Option<&String>, cookie_owner: Option<&String>) -> bool {
    match (job_owner, cookie_owner) {
        (Some(job_owner), Some(cookie_owner)) => job_owner == cookie_owner,
        _ => false,
    }
}
