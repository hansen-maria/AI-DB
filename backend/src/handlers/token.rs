//! ============================================================================
//! Owner token of the current browser session
//!
//! The browser's owner identity lives in the HttpOnly cookie `ai_db_user`, so
//! the frontend cannot read it. These endpoints hand it to the frontend so the
//! user can copy it into the AI-DB CLI or a script (`Authorization: Bearer`);
//! web jobs and CLI jobs then belong to the same owner.
//!
//! The token is the key to the user's jobs, so:
//!   * only the cookie is read – a Bearer token is never echoed back;
//!   * only same-origin requests are answered (`auth::is_same_origin_request`);
//!   * responses carry `Cache-Control: no-store`.
//! ============================================================================

use axum::{
    http::{header::CACHE_CONTROL, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::CookieJar;

use crate::auth::{add_new_owner_cookie, is_same_origin_request, owner_secret_from_cookie};
use crate::models::{AuthTokenResponse, ErrorResponse};

const NO_STORE: [(axum::http::HeaderName, &str); 1] = [(CACHE_CONTROL, "no-store")];

fn error(status: StatusCode, detail: &str) -> Response {
    (status, NO_STORE, Json(ErrorResponse::new(detail))).into_response()
}

fn cross_origin() -> Response {
    error(
        StatusCode::FORBIDDEN,
        "The access token can only be requested by the AI-DB web interface itself",
    )
}

fn not_exportable() -> Response {
    error(
        StatusCode::CONFLICT,
        "The identity stored in this browser cannot be used as an API token",
    )
}

/// Returns the owner token of this browser session
#[utoipa::path(
    get,
    path = "/api/auth/token",
    tag = "Auth",
    description = "Returns the owner token stored in this browser's HttpOnly cookie, so it \
        can be used as `Authorization: Bearer <token>` in scripts or the AI-DB CLI. \
        Browser sessions only: same-origin requests with the owner cookie; API tokens \
        are not echoed back.",
    responses(
        (status = 200, description = "Owner token of this browser", body = AuthTokenResponse),
        (status = 403, description = "Cross-origin request", body = ErrorResponse),
        (status = 404, description = "No owner identity in this browser yet", body = ErrorResponse),
        (status = 409, description = "Stored identity is not usable as an API token", body = ErrorResponse),
    )
)]
pub async fn get_auth_token(jar: CookieJar, headers: HeaderMap) -> Response {
    if !is_same_origin_request(&headers) {
        return cross_origin();
    }
    match owner_secret_from_cookie(&jar) {
        Ok(Some(token)) => {
            (StatusCode::OK, NO_STORE, Json(AuthTokenResponse { token })).into_response()
        }
        Ok(None) => error(
            StatusCode::NOT_FOUND,
            "No access token in this browser yet – it is created with your first job",
        ),
        Err(_) => not_exportable(),
    }
}

/// Returns the owner token of this browser session, creating one if needed
#[utoipa::path(
    post,
    path = "/api/auth/token",
    tag = "Auth",
    description = "Like GET, but creates a new owner identity (and sets the HttpOnly cookie) \
        if this browser has none yet. Idempotent: an existing identity is returned unchanged \
        and never replaced.",
    responses(
        (status = 200, description = "Existing owner token of this browser", body = AuthTokenResponse),
        (status = 201, description = "New owner token created and set as cookie", body = AuthTokenResponse),
        (status = 403, description = "Cross-origin request", body = ErrorResponse),
        (status = 409, description = "Stored identity is not usable as an API token", body = ErrorResponse),
    )
)]
pub async fn create_auth_token(jar: CookieJar, headers: HeaderMap) -> Response {
    if !is_same_origin_request(&headers) {
        return cross_origin();
    }
    match owner_secret_from_cookie(&jar) {
        Ok(Some(token)) => {
            (StatusCode::OK, NO_STORE, Json(AuthTokenResponse { token })).into_response()
        }
        Ok(None) => {
            let (token, jar) = add_new_owner_cookie(jar);
            (
                StatusCode::CREATED,
                jar,
                NO_STORE,
                Json(AuthTokenResponse { token }),
            )
                .into_response()
        }
        // Never overwrite an existing identity: its jobs would become unreachable
        Err(_) => not_exportable(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::OWNER_COOKIE_NAME;
    use axum::http::{header::SET_COOKIE, HeaderValue};
    use axum_extra::extract::cookie::Cookie;

    const TOKEN: &str = "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c";

    fn same_origin() -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        h
    }

    fn same_site() -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("sec-fetch-site", HeaderValue::from_static("same-site"));
        h
    }

    fn with_cookie(value: &str) -> CookieJar {
        CookieJar::new().add(Cookie::new(OWNER_COOKIE_NAME, value.to_string()))
    }

    async fn token_of(response: Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice::<AuthTokenResponse>(&bytes)
            .unwrap()
            .token
    }

    #[tokio::test]
    async fn get_returns_the_cookie_token_uncached() {
        let response = get_auth_token(with_cookie(TOKEN), same_origin()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        assert_eq!(token_of(response).await, TOKEN);
    }

    #[tokio::test]
    async fn get_rejects_other_origins_even_on_the_same_site() {
        let response = get_auth_token(with_cookie(TOKEN), same_site()).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn get_without_cookie_is_not_found_and_creates_nothing() {
        let response = get_auth_token(CookieJar::new(), same_origin()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.headers().get(SET_COOKIE).is_none());
    }

    #[tokio::test]
    async fn get_never_echoes_a_bearer_token() {
        let mut headers = same_origin();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {TOKEN}")).unwrap(),
        );
        let response = get_auth_token(CookieJar::new(), headers).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn post_creates_a_cookie_identity_when_missing() {
        let response = create_auth_token(CookieJar::new(), same_origin()).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let set_cookie = response.headers()[SET_COOKIE].to_str().unwrap().to_string();
        let token = token_of(response).await;
        assert!(set_cookie.starts_with(&format!("{OWNER_COOKIE_NAME}={token}")));
        assert!(set_cookie.contains("HttpOnly"));
    }

    #[tokio::test]
    async fn post_keeps_an_existing_identity() {
        let response = create_auth_token(with_cookie(TOKEN), same_origin()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get(SET_COOKIE).is_none());
        assert_eq!(token_of(response).await, TOKEN);
    }

    #[tokio::test]
    async fn post_never_replaces_a_non_exportable_identity() {
        let response = create_auth_token(with_cookie("legacy-value"), same_origin()).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(response.headers().get(SET_COOKIE).is_none());
    }

    #[tokio::test]
    async fn post_rejects_cross_origin_requests() {
        let response = create_auth_token(CookieJar::new(), same_site()).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(response.headers().get(SET_COOKIE).is_none());
    }
}
