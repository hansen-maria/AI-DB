//! AI-DB REST API Backend
//!
//! Hash-Based Annotation Service for Microbial Sequencing Data
//!
//! # Module Structure
//!
//! - `models` - Data structures (Job, Sequence, Pagination, Error)
//! - `handlers` - API endpoint handlers (jobs, download, health)
//! - `services` - Business logic (FASTA parsing, annotation)
//! - `export` - Export formats (TSV, JSON, FASTA, GFF3)
//! - `state` - Application state and database connection
//! - `auth` - Owner authentication (cookie or API token), read-only sharing, admin secret
//! - `storage` - Logic to persist jobs for 30 days using SQLite

pub mod auth;
pub mod export;
pub mod handlers;
pub mod models;
pub mod services;
pub mod state;
pub mod storage;

use axum::extract::DefaultBodyLimit;
use axum::http::Method;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::openapi::path::Operation;
use utoipa::openapi::security::{
    ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityRequirement, SecurityScheme,
};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::{
    bulk_delete_jobs, create_auth_token, create_job, db_info, delete_bakta_job, delete_job,
    delete_psos_results, download_job, export_job_stats, get_auth_token, get_bakta_job, get_job,
    get_job_stats, get_kpi_overview, get_psos_results, get_sequence, health_check,
    ingest_bakta_results, list_jobs, rename_job, retry_job, save_bakta_job, save_psos_results,
};
use crate::models::{
    AuthTokenResponse, BaktaJobStateResponse, BulkDeleteRequest, BulkDeleteResponse,
    CustomAnnotationEntry, ErrorResponse, FunctionalStats, IngestCustomAnnotationsRequest,
    IngestCustomAnnotationsResponse, JobCreateResponse, JobResponse, JobStatus, JobSummary,
    PaginatedJobResponse, PaginatedJobsResponse, PaginationInfo, PsosResult, PsosResultsResponse,
    RenameJobRequest, SaveBaktaJobRequest, SaveBaktaJobResponse, SavePsosResultsRequest,
    SavePsosResultsResponse, SequenceInfo, StoredBaktaJob,
};
use crate::state::AppState;

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    info(
        title = "AI-DB REST API",
        version = "1.0.0",
        description = "Hash-Based Annotation Service for Microbial Sequencing Data\n\n\
            AI-DB accelerates microbial sequencing data analysis while preserving data \
            sovereignty through cryptographic hash-based annotations.\n\n\
            ## Features\n\n\
            - **Privacy**: Sequence data processed as MD5 hashes\n\
            - **Fast**: Hash-based annotations in seconds instead of hours\n\
            - **Comprehensive**: Access to Bakta UniRef protein annotations (~350M sequences)\n\
            - **Fallback**: LookUp in the AI-DB database\n\n\
            ## Authentication\n\n\
            ### Job owners (all routes except `/api/admin/...`)\n\n\
            Jobs belong to a random identifier (UUIDv4). Browsers receive it automatically \
            as an HTTP-only cookie. Scripts and pipelines generate their own UUIDv4 \
            (e.g. `uuidgen`) and send it with **every** request as \
            `Authorization: Bearer <token>` or `X-API-Key: <token>` (no cookies needed). \
            Keep the token secret: it is the key to your jobs. To manage jobs created \
            in the web interface from a script, copy the browser's token via \
            *Jobs → Access token* (backed by `GET /api/auth/token`).\n\n\
            Shared jobs are **read-only**: anyone who knows a job ID can view the job, its \
            sequences, stats and stored Psos/Bakta results. Downloading (incl. the stats CSV \
            export), renaming, deleting, retrying, starting or saving Psos/Bakta runs and \
            ingesting results require the owner (403 otherwise).\n\n\
            ### Admins (`/api/admin/...` only)\n\n\
            Admin routes are authorized **exclusively** by the shared secret in the \
            `X-Admin-Secret` header. Owner tokens and cookies are not accepted there, and \
            the admin secret grants no owner rights on jobs.\n\n\
            **Swagger UI:** use *Authorize* and fill in `bearer_token` **or** `api_key` \
            for job routes and `admin_secret` for admin routes; each value is only sent \
            to the routes it applies to.",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
        contact(name = "AI-DB Team", url = "https://github.com/hansen-maria/AI-DB-Web")
    ),
    tags(
        (name = "Jobs", description = "Annotation job management - create and query jobs (viewing by job ID, changes owner-only)"),
        (name = "psos", description = "Psos analysis results storage (read by anyone with the job ID, write owner-only)"),
        (name = "bakta", description = "Bakta job state persistence (read by anyone with the job ID, write owner-only)"),
        (name = "Auth", description = "Owner token of the current browser session (web interface only, same-origin)"),
        (name = "admin", description = "Admin-only endpoints (X-Admin-Secret only)"),
        (name = "Health", description = "Health check and database info")
    ),
    paths(
        handlers::jobs::get_job,
        handlers::jobs::create_job,
        handlers::jobs::list_jobs,
        handlers::jobs::delete_job,
        handlers::jobs::rename_job,
        handlers::jobs::bulk_delete_jobs,
        handlers::jobs::get_sequence,
        handlers::jobs::retry_job,
        handlers::download::download_job,
        handlers::stats::get_job_stats,
        handlers::stats::export_job_stats,
        handlers::psos::save_psos_results,
        handlers::psos::get_psos_results,
        handlers::psos::delete_psos_results,
        handlers::bakta::save_bakta_job,
        handlers::bakta::get_bakta_job,
        handlers::bakta::delete_bakta_job,
        handlers::bakta::ingest_bakta_results,
        handlers::kpi::get_kpi_overview,
        handlers::kpi::list_admin_annotations,
        handlers::kpi::review_admin_annotation,
        handlers::health::health_check,
        handlers::health::db_info,
        handlers::token::get_auth_token,
        handlers::token::create_auth_token
    ),
    components(schemas(
        JobStatus,
        SequenceInfo,
        JobResponse,
        JobCreateResponse,
        RenameJobRequest,
        BulkDeleteRequest,
        BulkDeleteResponse,
        ErrorResponse,
        PaginationInfo,
        PaginatedJobsResponse,
        JobSummary,
        PaginatedJobResponse,
        crate::handlers::jobs::JobViewResponse,
        FunctionalStats,
        PsosResult,
        PsosResultsResponse,
        SavePsosResultsRequest,
        SavePsosResultsResponse,
        StoredBaktaJob,
        SaveBaktaJobRequest,
        SaveBaktaJobResponse,
        BaktaJobStateResponse,
        CustomAnnotationEntry,
        IngestCustomAnnotationsRequest,
        IngestCustomAnnotationsResponse,
        AuthTokenResponse,
        crate::handlers::kpi::KpiMonthEntry,
        crate::handlers::kpi::KpiOverviewResponse,
        crate::handlers::kpi::AdminAnnotationEntry,
        crate::handlers::kpi::AdminAnnotationsResponse,
        crate::handlers::kpi::ReviewAnnotationRequest,
        crate::handlers::kpi::ReviewAnnotationResponse,
    ))
)]
struct ApiDoc;

// Security scheme names (must match between scheme registration and requirements)
const SCHEME_BEARER: &str = "bearer_token";
const SCHEME_API_KEY: &str = "api_key";
const SCHEME_ADMIN: &str = "admin_secret";

/// Access level of an operation, derived from its path and method.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Access {
    /// No credentials involved (health, db info).
    Public,
    /// Works anonymously, but an owner token is used if present
    /// (viewing shared jobs, creating a job, listing own jobs).
    OwnerOptional,
    /// Owner token (or cookie) required.
    OwnerRequired,
    /// `X-Admin-Secret` required, nothing else accepted.
    Admin,
}

/// Access map for the OpenAPI doc. Owner-only job routes come from
/// `auth::owner_guarded_job_id`, the same rule the middleware enforces.
fn classify(path: &str, method: &Method) -> Access {
    if path.starts_with("/api/admin/") {
        return Access::Admin;
    }
    if auth::owner_guarded_job_id(method, path).is_some() {
        return Access::OwnerRequired;
    }
    // Bulk delete only affects the requester's own jobs (checked in the handler)
    if path == "/api/jobs/" && *method == Method::DELETE {
        return Access::OwnerRequired;
    }
    if path.starts_with("/api/job/") || path.starts_with("/api/jobs/") {
        return Access::OwnerOptional;
    }
    Access::Public
}

fn requirement(scheme: &str) -> SecurityRequirement {
    SecurityRequirement::new(scheme, Vec::<String>::new())
}

/// Alternatives (OR): Bearer token or X-API-Key. The cookie is sent by the
/// browser automatically and needs no OpenAPI declaration. The admin secret is
/// never offered outside admin routes and vice versa.
fn security_for(access: Access) -> Option<Vec<SecurityRequirement>> {
    match access {
        Access::Public => None,
        Access::OwnerRequired => Some(vec![
            requirement(SCHEME_BEARER),
            requirement(SCHEME_API_KEY),
        ]),
        Access::OwnerOptional => Some(vec![
            requirement(SCHEME_BEARER),
            requirement(SCHEME_API_KEY),
            // `{}` = anonymous access is allowed as well
            SecurityRequirement::default(),
        ]),
        Access::Admin => Some(vec![requirement(SCHEME_ADMIN)]),
    }
}

/// Registers the security schemes and attaches the matching security
/// requirements to every operation, so Swagger UI actually sends the
/// configured credentials (`Authorize` alone only stores them).
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            SCHEME_BEARER,
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("UUIDv4")
                    .description(Some(
                        "Self-generated random UUIDv4 identifying the job owner. \
                         Valid for job routes only, not for /api/admin/*.",
                    ))
                    .build(),
            ),
        );
        components.add_security_scheme(
            SCHEME_API_KEY,
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "X-API-Key",
                "Alternative to the bearer token: the same owner UUIDv4 in an X-API-Key \
                 header. Valid for job routes only, not for /api/admin/*.",
            ))),
        );
        components.add_security_scheme(
            SCHEME_ADMIN,
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "X-Admin-Secret",
                "Shared admin secret (ADMIN_KPI_SECRET). The only credential accepted on \
                 /api/admin/*; grants no owner rights on jobs.",
            ))),
        );

        for (path, item) in openapi.paths.paths.iter_mut() {
            let operations: [(Method, &mut Option<Operation>); 5] = [
                (Method::GET, &mut item.get),
                (Method::POST, &mut item.post),
                (Method::PUT, &mut item.put),
                (Method::PATCH, &mut item.patch),
                (Method::DELETE, &mut item.delete),
            ];
            for (method, operation) in operations {
                if let Some(operation) = operation {
                    operation.security = security_for(classify(path, &method));
                }
            }
        }
    }
}

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
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderName::from_static(auth::API_KEY_HEADER),
            axum::http::HeaderName::from_static(auth::ADMIN_SECRET_HEADER),
        ])
        .allow_credentials(true);

    // Admin routes: authorized exclusively by X-Admin-Secret
    let admin_routes = Router::new()
        .route("/api/admin/kpis", get(get_kpi_overview))
        .route(
            "/api/admin/annotations",
            get(crate::handlers::kpi::list_admin_annotations),
        )
        .route(
            "/api/admin/annotations/{md5}/review",
            post(crate::handlers::kpi::review_admin_annotation),
        )
        .route_layer(middleware::from_fn(auth::require_admin_secret));

    // Build router
    let app = Router::new()
        // Health & info routes
        .route("/api/health", get(health_check))
        .route("/api/db/info", get(db_info))
        // Owner token of the browser session (same-origin only)
        .route(
            "/api/auth/token",
            get(get_auth_token).post(create_auth_token),
        )
        // Job management routes
        .route("/api/job/", post(create_job))
        .route(
            "/api/job/{job_id}",
            get(get_job).delete(delete_job).patch(rename_job),
        )
        .route("/api/job/{job_id}/download/{format}", get(download_job))
        .route("/api/job/{job_id}/stats", get(get_job_stats))
        .route("/api/job/{job_id}/stats/export", get(export_job_stats))
        .route("/api/job/{job_id}/retry", post(retry_job))
        .route("/api/job/{job_id}/sequence/{seq_id}", get(get_sequence))
        // Psos results routes
        .route(
            "/api/job/{job_id}/psos",
            get(get_psos_results)
                .post(save_psos_results)
                .delete(delete_psos_results),
        )
        // Bakta job state routes
        .route(
            "/api/job/{job_id}/bakta",
            get(get_bakta_job)
                .post(save_bakta_job)
                .delete(delete_bakta_job),
        )
        // Bakta → custom annotations ingest
        .route("/api/job/{job_id}/bakta/ingest", post(ingest_bakta_results))
        .route("/api/jobs/", get(list_jobs).delete(bulk_delete_jobs))
        // Shared jobs are read-only: owner check for every write/run/download
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_owner_for_writes,
        ))
        // Admin (separate auth realm, see above)
        .merge(admin_routes)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Scheme names of all alternatives of an operation's security requirement.
    fn schemes(op: &Operation) -> Vec<String> {
        let json = serde_json::to_value(op.security.clone().unwrap_or_default()).unwrap();
        json.as_array()
            .unwrap()
            .iter()
            .map(|req| {
                req.as_object()
                    .unwrap()
                    .keys()
                    .next()
                    .cloned()
                    .unwrap_or_else(|| "{}".into())
            })
            .collect()
    }

    fn op<'a>(doc: &'a utoipa::openapi::OpenApi, path: &str, method: &str) -> &'a Operation {
        let item = doc
            .paths
            .paths
            .get(path)
            .unwrap_or_else(|| panic!("path {path} missing in OpenAPI doc"));
        match method {
            "GET" => item.get.as_ref(),
            "POST" => item.post.as_ref(),
            "PATCH" => item.patch.as_ref(),
            "DELETE" => item.delete.as_ref(),
            _ => None,
        }
        .unwrap_or_else(|| panic!("{method} {path} missing in OpenAPI doc"))
    }

    #[test]
    fn admin_routes_accept_only_admin_secret() {
        let doc = ApiDoc::openapi();
        for (path, method) in [
            ("/api/admin/kpis", "GET"),
            ("/api/admin/annotations", "GET"),
            ("/api/admin/annotations/{md5}/review", "POST"),
        ] {
            assert_eq!(
                schemes(op(&doc, path, method)),
                vec![SCHEME_ADMIN],
                "{method} {path}"
            );
        }
    }

    #[test]
    fn owner_only_routes_require_owner_token_not_admin_secret() {
        let doc = ApiDoc::openapi();
        for (path, method) in [
            ("/api/job/{job_id}", "DELETE"),
            ("/api/job/{job_id}", "PATCH"),
            ("/api/job/{job_id}/download/{format}", "GET"),
            ("/api/job/{job_id}/stats/export", "GET"),
            ("/api/job/{job_id}/retry", "POST"),
            ("/api/job/{job_id}/psos", "POST"),
            ("/api/job/{job_id}/psos", "DELETE"),
            ("/api/job/{job_id}/bakta", "POST"),
            ("/api/job/{job_id}/bakta", "DELETE"),
            ("/api/job/{job_id}/bakta/ingest", "POST"),
            ("/api/jobs/", "DELETE"),
        ] {
            assert_eq!(
                schemes(op(&doc, path, method)),
                vec![SCHEME_BEARER, SCHEME_API_KEY],
                "{method} {path}"
            );
        }
    }

    #[test]
    fn read_routes_accept_optional_owner_token() {
        let doc = ApiDoc::openapi();
        for (path, method) in [
            ("/api/job/{job_id}", "GET"),
            ("/api/job/{job_id}/stats", "GET"),
            ("/api/job/", "POST"),
            ("/api/job/{job_id}/psos", "GET"),
            ("/api/job/{job_id}/bakta", "GET"),
            ("/api/jobs/", "GET"),
        ] {
            assert_eq!(
                schemes(op(&doc, path, method)),
                vec![SCHEME_BEARER, SCHEME_API_KEY, "{}"],
                "{method} {path}"
            );
        }
    }

    #[test]
    fn auth_token_is_cookie_only() {
        // Browser session only: no Bearer / API key / admin secret offered
        let doc = ApiDoc::openapi();
        assert!(op(&doc, "/api/auth/token", "GET").security.is_none());
        assert!(op(&doc, "/api/auth/token", "POST").security.is_none());
    }

    #[test]
    fn health_is_public() {
        let doc = ApiDoc::openapi();
        assert!(op(&doc, "/api/health", "GET").security.is_none());
    }
}
