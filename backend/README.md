# AI-DB Backend

Rust/Axum backend for the AI-DB Hash-Based Annotation Service.

## Directory Structure

```
backend/src/
├── main.rs              # Entry point, router setup 
├── auth.rs              # Owner identification (cookie / API token), owner digest, owner & admin guards
├── state.rs             # AppState, DB connection, job management
├── storage.rs           # SQLite persistence (30-day retention)
│
├── models/              # Re-exports the wire types from crates/aidb-types
│   ├── mod.rs
│   └── sequence.rs      # Server-internal: HashLookupResult, SequenceFilter, AdvancedSequenceFilter
│
├── handlers/            # API endpoints 
│   ├── mod.rs           
│   ├── bakta.rs         # save/get/delete_bakta_job, ingest_bakta_results (verified, curated)
│   ├── download.rs      # download_job
│   ├── health.rs        # health_check, db_info
│   ├── jobs.rs          # get_job, create_job, list_jobs, delete_job, rename/retry/bulk delete
│   ├── kpi.rs           # Admin: KPI overview (X-Admin-Secret)
│   ├── psos.rs          # save_psos_results, get_psos_results, delete_psos_results
│   └── stats.rs         # get_job_stats, export_job_stats (functional analysis, CSV)
│
├── services/            # Business logic 
│   ├── mod.rs           
│   ├── fasta.rs         # FastaIterator, compute_md5
│   └── annotation.rs    # process_job_from_file, lookup_hash_in_bakta
│
└── export/              # Download formats 
    ├── mod.rs           
    ├── format.rs        # DownloadFormat enum
    ├── tsv.rs           # generate_tsv
    ├── json.rs          # generate_json, JsonExport structs
    ├── fasta.rs         # generate_fasta
    └── gff3.rs          # generate_gff3, sanitize helpers
```

## Module Responsibilities

| Module         | Purpose                                              |
|----------------|------------------------------------------------------|
| **main.rs**    | Application entry point, router configuration        |
| **auth.rs**    | Owner identification, digest, owner/admin middleware |
| **state.rs**   | Shared state, database connection, job cache         |
| **storage.rs** | SQLite persistence, 30-day retention, cleanup        |
| **models/**    | All data structures and types                        |
| **handlers/**  | HTTP request/response handling                       |
| **services/**  | Core business logic (FASTA parsing, DB lookup)       |
| **export/**    | Output format generation                             |

## Authentication and Sharing

A job belongs to a random secret (UUIDv4), supplied in one of two ways:

- **Browser**: HTTP-only cookie `ai_db_user` (SameSite=Lax, 1 year), set on first job submission.
- **API / pipelines**: your own UUIDv4 in `Authorization: Bearer <token>` or `X-API-Key: <token>`
  on every request; no cookie is set. A malformed token gives `401` on job creation and is treated
  as anonymous elsewhere. If token and cookie are both sent, the token wins.
- **Browser → API**: *Jobs → Access token* in the web interface shows the cookie's secret
  (`GET /api/auth/token`, or `POST` to create one before the first job). Used as API token it
  identifies the same owner, so web jobs can be managed from scripts and the AI-DB CLI. The
  endpoint reads only the cookie (never echoes a Bearer token), answers only same-origin requests
  (`auth::is_same_origin_request`: CORS mirrors origins and the Lax cookie also reaches sibling
  subdomains) and sends `Cache-Control: no-store`.

**The secret is never stored.** `auth::owner_digest` computes `md5("aidb-owner:" + secret)`
(32 hex characters); this digest is the `owner_id` in `jobs.db` and in the KPI counters. At startup,
`storage::migrate_owner_ids_to_digest` / `migrate_kpi_owner_ids_to_digest` convert rows that still hold a raw
UUID (idempotent, in a transaction).

Anyone who knows a job ID can **view** it read-only: job, sequences, stats and stored Psos/Bakta
results (`GET /api/job/{id}` returns `is_owner`). Owner-only are every state-changing request on a job
(`POST`/`PATCH`/`DELETE`: rename, delete, retry, saving/deleting Psos and Bakta state, ingest), the
result download and the stats CSV export. Listing (`GET /api/jobs/`) and bulk delete only ever act on
the caller's own jobs.

Enforcement is central: `auth::owner_guarded_job_id` defines which job routes are owner-only, and the
`auth::require_owner_for_writes` middleware rejects non-owners (`403`, unknown job `404`) before any
handler runs. The handlers check again (`auth::authorize_owner` / `auth::validate_owner`). The same
rule drives the OpenAPI security requirements, so Swagger UI sends the token exactly where it applies.

**Admin routes** (`/api/admin/*`) are a separate realm guarded by `auth::require_admin_secret`: only the
shared secret in `X-Admin-Secret` (`ADMIN_KPI_SECRET`) authorizes there (`401` otherwise, `503` if the
secret is not configured). Owner tokens and cookies are ignored on admin routes, and the admin secret
grants no owner rights on jobs.

## Job Persistence

Jobs are persisted to SQLite and survive container restarts:

- **Location**: `/data/jobs.db` (configurable via `AI_DB_JOBS_PATH`)
- **Retention**: 30 days (automatic cleanup on startup)
- **Storage**: Jobs serialized as JSON in SQLite

### Database Schema

```sql
CREATE TABLE IF NOT EXISTS jobs (
    job_id TEXT PRIMARY KEY,
    owner_id TEXT,              -- one-way digest of the owner secret, never the secret itself
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    filename TEXT,
    sequence_count INTEGER NOT NULL DEFAULT 0,
    processed_count INTEGER NOT NULL DEFAULT 0,
    hash_matches INTEGER NOT NULL DEFAULT 0,
    alignment_matches INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    sequences TEXT
);
```

### Persistence Flow

1. On startup: Load all jobs from SQLite into memory cache
2. On job create/update: Write to both memory and SQLite
3. On startup: Cleanup jobs older than 30 days

## API Endpoints

Access: **public** = no credentials; **anyone** = anyone with the job ID (owner token optional);
**owner** = owner cookie or API token; **admin** = `X-Admin-Secret` only.

| Method   | Endpoint                              | Handler                                  | Access                                |
|----------|---------------------------------------|------------------------------------------|---------------------------------------|
| `JOBS`   |                                       |                                          |                                       |
| `POST`   | `/api/job/`                           | `handlers::jobs::create_job`             | anyone (creates owner cookie if none) |
| `GET`    | `/api/job/{id}`                       | `handlers::jobs::get_job`                | anyone                                |
| `PATCH`  | `/api/job/{id}`                       | `handlers::jobs::rename_job`             | owner                                 |
| `DELETE` | `/api/job/{id}`                       | `handlers::jobs::delete_job`             | owner                                 |
| `GET`    | `/api/job/{id}/download/{format}`     | `handlers::download::download_job`       | owner                                 |
| `GET`    | `/api/job/{id}/stats`                 | `handlers::stats::get_job_stats`         | anyone                                |
| `GET`    | `/api/job/{id}/stats/export`          | `handlers::stats::export_job_stats`      | owner                                 |
| `POST`   | `/api/job/{id}/retry`                 | `handlers::jobs::retry_job`              | owner                                 |
| `GET`    | `/api/job/{id}/sequence/{seq_id}`     | `handlers::jobs::get_sequence`           | anyone                                |
| `GET`    | `/api/jobs/`                          | `handlers::jobs::list_jobs`              | own jobs only                         |
| `DELETE` | `/api/jobs/`                          | `handlers::jobs::bulk_delete_jobs`       | own jobs only (`207` if partial)      |
| `PSOS`   |                                       |                                          |                                       |
| `GET`    | `/api/job/{id}/psos`                  | `handlers::psos::get_psos_results`       | anyone                                |
| `POST`   | `/api/job/{id}/psos`                  | `handlers::psos::save_psos_results`      | owner                                 |
| `DELETE` | `/api/job/{id}/psos`                  | `handlers::psos::delete_psos_results`    | owner                                 |
| `BAKTA`  |                                       |                                          |                                       |
| `GET`    | `/api/job/{id}/bakta`                 | `handlers::bakta::get_bakta_job`         | anyone                                |
| `POST`   | `/api/job/{id}/bakta`                 | `handlers::bakta::save_bakta_job`        | owner                                 |
| `DELETE` | `/api/job/{id}/bakta`                 | `handlers::bakta::delete_bakta_job`      | owner                                 |
| `POST`   | `/api/job/{id}/bakta/ingest`          | `handlers::bakta::ingest_bakta_results`  | owner                                 |
| `ADMIN`  |                                       |                                          |                                       |
| `GET`    | `/api/admin/kpis`                     | `handlers::kpi::get_kpi_overview`        | admin                                 |
| `HEALTH` |                                       |                                          |                                       |
| `GET`    | `/api/health`                         | `handlers::health::health_check`         | public                                |
| `GET`    | `/api/db/info`                        | `handlers::health::db_info`              | public                                |
| `AUTH`   |                                       |                                          |                                       |
| `GET`    | `/api/auth/token`                     | `handlers::token::get_auth_token`        | browser cookie, same-origin only      |
| `POST`   | `/api/auth/token`                     | `handlers::token::create_auth_token`     | browser cookie, same-origin only      |

## Functional Analysis

The `/api/job/{id}/stats` endpoint computes statistics from the job's stored annotations
(no additional database query); `/api/job/{id}/stats/export` returns the same as CSV (owner only):

- **Top Genes**: Most frequent gene names
- **Top Products**: Most frequent product descriptions
- **COG Categories**: Clusters of Orthologous Groups distribution
- **EC Classes**: Enzyme Commission classification
- **GO Terms**: Gene Ontology terms (the ontology namespace is not stored, so all terms are
  reported under `molecular_function`)

### Database Lookup Chain

```
sequence.aa_hash (MD5)
    → Hash lookup (per sequence during job processing):
      1. MD5(seq) → hash in Bakta DB?        → Annotation, else Step 2
      2. MD5(seq) → hash in AI-DB?           → Annotation, else Step 3
      3. No match
```

`annotation_source` is `bakta_db` or `aidb_db`; matched sequences additionally carry `seen_count`
(see [Sightings counter](#sightings-counter)).

## AI-DB Annotations DB Ingest

`POST /api/job/{id}/bakta/ingest` (owner only):

1. **Verification**: hashes must belong to the job's unmatched sequences or to its `aidb_db` matches
   (re-check); length must match; all fields are
   validated. Hashes from the Bakta DB are never accepted.
2. **Insert or complete**: new hashes are inserted into `ups` (+ `ips`/`psc`). Existing values are never
   overwritten; only NULL fields are filled in (`COALESCE(existing, new)`), `updated_at` changes only then.
   Response: `ingested` (new), `updated` (completed), `unchanged`, `rejected`, `total`.
3. **Sightings**: every accepted entry is counted in `hash_sightings` (see below).

### Sightings counter

`hash_sightings(hash, seen_count, first_seen_at, last_seen_at, last_job_id)` in `custom_annotations.db`
counts in how many jobs AI-DB has seen a hash: lookup hits in the Bakta DB or the AI-DB annotations DB
(`services::annotation::apply_sightings`, after a job or retry) and ingests. `last_job_id` limits this to
once per job, so duplicates within a job, repeated ingests or retries do not inflate it. Unmatched hashes are
not recorded unless ingested. The value after the job is stored per sequence as `seen_count`.

The table is created lazily (`storage::ensure_annotations_schema`) and seeded once from existing `ups` rows
(number of distinct jobs in the former `annotation_submissions` table, at least 1). Columns and tables of the
former curation model (`ups.status`, `ups.confirmations`, `annotation_submissions`, …) are left in place but no
longer used.

## Advanced Filtering

The `AdvancedSequenceFilter` supports (query parameters of `GET /api/job/{id}` and the download):

| Filter        | Type   | Description                                                                                         |
|---------------|--------|-----------------------------------------------------------------------------------------------------|
| `filter`      | String | Match status: `all` (default), `bakta_db`, `aidb_db`, `hash_match` (either DB), `alignment`, `none` |
| `search`      | String | Case-insensitive search in ID/gene/product                                                          |
| `min_length`  | usize  | Minimum sequence length                                                                             |
| `max_length`  | usize  | Maximum sequence length                                                                             |
| `cog`         | String | COG category letter (A-Z)                                                                           |
| `ec_class`    | String | EC class prefix (1-7)                                                                               |
| `has_gene`    | bool   | Only sequences with gene annotation                                                                 |
| `has_product` | bool   | Only sequences with product annotation                                                              |

## Environment Variables

| Variable                        | Description                                                           | Default                            |
|---------------------------------|-----------------------------------------------------------------------|------------------------------------|
| `RUST_LOG`                      | Log level (trace, debug, info, warn, error)                           | `info`                             |
| `BAKTA_DB`                      | Path to Bakta database directory                                      | `/bakta-db`                        |
| `AI_DB_TEMP_DIR`                | Directory for temporary upload files                                  | `/tmp`                             |
| `AI_DB_JOBS_PATH`               | Path to SQLite jobs database                                          | `/data/jobs.db`                    |
| `AI_DB_CUSTOM_ANNOTATIONS_PATH` | Path to custom AI-DB database                                         | `/custom-db/custom_annotations.db` |
| `ADMIN_KPI_SECRET`              | Secret for `X-Admin-Secret` (admin routes return 503 if unset/empty)  | unset                              |

## Usage

```bash
# Build
cargo build --release

# Run (listens on 0.0.0.0:8000)
cargo run

# Run with logging
RUST_LOG=debug cargo run

# With custom paths
AI_DB_TEMP_DIR=/mnt/ai-db-tmp \
AI_DB_JOBS_PATH=/data/jobs.db \
AI_DB_CUSTOM_ANNOTATIONS_PATH=/custom-db/custom_annotations.db \
BAKTA_DB=/bakta-db \
cargo run
```

## Docker

The Dockerfile creates the `/data` directory with appropriate permissions:

```dockerfile
# Create data directory for job persistence
RUN mkdir -p /data && chown appuser:appuser /data
```

Ensure a named volume is mounted:

```yaml
volumes:
  - jobs-data:/data
```

## Testing

```bash
# Run all tests
cargo test

# Run tests for specific module
cargo test services::fasta
cargo test export::gff3
cargo test storage
cargo test auth
cargo test models::sequence
```

## License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this Service by you,
as defined in the Apache-2.0 license, shall be dually licensed as above, without any additional terms or conditions.
