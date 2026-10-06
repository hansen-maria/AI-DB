# AI-DB Backend

Rust/Axum backend for the AI-DB Hash-Based Annotation Service.

## Directory Structure

```
backend/src/
├── main.rs              # Entry point, router setup 
├── auth.rs              # Owner identification (cookie / API token), owner digest, sharing rules
├── state.rs             # AppState, DB connection, job management
├── storage.rs           # SQLite persistence (30-day retention)
│
├── models/              # Data structures 
│   ├── mod.rs           
│   ├── bakta.rs         # Data models for Bakta job persistence
│   ├── custom_db.rs     # AI-DB annotation ingest models (entries, provenance, status)
│   ├── error.rs         # ErrorResponse
│   ├── health.rs        # Health-Check types and responses
│   ├── job.rs           # JobResponse, JobStatus, JobSummary, JobCreateResponse
│   ├── pagination.rs    # PaginationInfo, PaginatedJobsResponse, Query types
│   ├── psos.rs          # Psos analysis result models
│   ├── sequence.rs      # SequenceInfo, SequenceFilter, AdvancedSequenceFilter
│   └── stats.rs         # FunctionalStats, CountItem, CogCategory, GoTerms
│
├── handlers/            # API endpoints 
│   ├── mod.rs           
│   ├── bakta.rs         # save/get/delete_bakta_job, ingest_bakta_results (verified, curated)
│   ├── download.rs      # download_job
│   ├── health.rs        # health_check, db_info
│   ├── jobs.rs          # get_job, create_job, list_jobs, delete_job, rename/retry/bulk delete
│   ├── kpi.rs           # Admin: KPI overview, curation review (shared secret)
│   ├── psos.rs          # save_psos_results, get_psos_results, delete_psos_results
│   └── stats.rs         # get_job_stats (functional analysis)
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
| **auth.rs**    | Owner identification (cookie / token), digest, auth  |
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

**The secret is never stored.** `auth::owner_digest` computes `md5("aidb-owner:" + secret)`
(32 hex characters); this digest is the `owner_id` in `jobs.db`, in the KPI counters and the basis of
the contributor id (`auth::contributor_id`, a second digest) used for community curation. At startup,
`storage::migrate_owner_ids_to_digest` / `migrate_kpi_owner_ids_to_digest` convert rows that still hold a raw
UUID (idempotent, in a transaction).

Anyone who knows a job ID can **view** it read-only (`GET /api/job/{id}` returns `is_owner`).
Download, rename, delete, retry, listing, Psos/Bakta state and ingest require the owner
(`auth::validate_owner`, `auth::authorize_owner`).

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

| Method   | Endpoint                              | Handler                                  |
|----------|---------------------------------------|------------------------------------------|
| `JOBS`   |                                       |                                          |
| `POST`   | `/api/job/`                           | `handlers::jobs::create_job`             |
| `GET`    | `/api/job/{id}`                       | `handlers::jobs::get_job`                |
| `DELETE` | `/api/job/{id}`                       | `handlers::jobs::delete_job`             |
| `GET`    | `/api/job/{id}/download/{format}`     | `handlers::download::download_job`       |
| `GET`    | `/api/job/{id}/stats`                 | `handlers::stats::get_job_stats`         |
| `PATCH`  | `/api/job/{id}`                       | `handlers::jobs::rename_job`             |
| `POST`   | `/api/job/{id}/retry`                 | `handlers::jobs::retry_job`              |
| `GET`    | `/api/job/{id}/sequence/{seq_id}`     | `handlers::jobs::get_sequence`           |
| `GET`    | `/api/job/{id}/stats/export`          | `handlers::stats::export_job_stats`      |
| `GET`    | `/api/jobs/`                          | `handlers::jobs::list_jobs`              |
| `DELETE` | `/api/jobs/`                          | `handlers::jobs::bulk_delete_jobs`       |
| `PSOS`   |                                       |                                          |
| `GET`    | `/api/job/{id}/psos`                  | `handlers::psos::get_psos_results`       |
| `POST`   | `/api/job/{id}/psos`                  | `handlers::psos::save_psos_results`      |
| `DELETE` | `/api/job/{id}/psos`                  | `handlers::psos::delete_psos_results`    |
| `BAKTA`  |                                       |                                          |
| `GET`    | `/api/job/{id}/bakta`                 | `handlers::bakta::get_bakta_job`         |
| `POST`   | `/api/job/{id}/bakta`                 | `handlers::bakta::save_bakta_job`        |
| `DELETE` | `/api/job/{id}/bakta`                 | `handlers::bakta::delete_bakta_job`      |
| `POST`   | `/api/job/{id}/bakta/ingest`          | `handlers::bakta::ingest_bakta_results`  |
| `ADMIN`  | header `X-Admin-Secret`               |                                          |
| `GET`    | `/api/admin/kpis`                     | `handlers::kpi::get_kpi_overview`        |
| `GET`    | `/api/admin/annotations`              | `handlers::kpi::list_admin_annotations`  |
| `POST`   | `/api/admin/annotations/{md5}/review` | `handlers::kpi::review_admin_annotation` |
| `HEALTH` |                                       |                                          |
| `GET`    | `/api/health`                         | `handlers::health::health_check`         |
| `GET`    | `/api/db/info`                        | `handlers::health::db_info`              |

## Functional Analysis

The `/api/job/{id}/stats` endpoint queries the Bakta database for:

- **Top Genes**: Most frequent gene names
- **Top Products**: Most frequent product descriptions
- **COG Categories**: Clusters of Orthologous Groups distribution
- **EC Classes**: Enzyme Commission classification
- **GO Terms**: Gene Ontology molecular functions

### Database Lookup Chain

```
sequence.aa_hash (MD5)
    → Hash lookup (per sequence during job processing):
      1. MD5(seq) → hash in Bakta DB?        → Annotation, else Step 2
      2. MD5(seq) → hash in AI-DB?           → Annotation (+ annotation_status), else Step 3
      3. No match
```

`annotation_source` is `bakta_db` or `aidb_db`; AI-DB matches additionally carry `annotation_status`
(`confirmed`, `candidate`, `conflicted`, `legacy`). Entries with status `rejected` are hidden from lookups.
TSV and GFF3 exports include the status (`annotation_status` column / attribute).

## Community Curation

`POST /api/job/{id}/bakta/ingest` does not blindly store what a client sends:

1. **Verification** (owner only): hashes must belong to the job's unmatched sequences, or to unconfirmed
   `aidb_db` matches of the same job (re-check); length must match; all fields are validated. Hashes from the Bakta DB are never accepted.
2. **Provenance, insert-only**: every contribution is stored in `annotation_submissions` (contributor digest, job,
   time, workflow, PSC identity/e-value), one vote per contributor and hash. Existing values are never overwritten by one contribution.
3. **Consensus**: new entries are `candidate`; `AI_DB_CONFIRMATIONS` (default 2) identical submissions from
   different contributors make them `confirmed`; differing ones make them `conflicted`; a different annotation
   reaching the threshold on its own replaces a first-come one. Pre-curation entries are `legacy`.
4. **Rate limit**: `AI_DB_INGEST_DAILY_LIMIT` contributions per contributor and 24 h (default 200000).
5. **Admin review**: `confirmed`/`rejected` lock an entry against consensus changes; `candidate` releases it.

```bash
curl -H "X-Admin-Secret: $ADMIN_KPI_SECRET" "https://HOST/api/admin/annotations?status=conflicted&limit=50"
curl -X POST -H "X-Admin-Secret: $ADMIN_KPI_SECRET" -H "Content-Type: application/json" \
     -d '{"status":"confirmed"}' https://HOST/api/admin/annotations/<md5>/review
```

"Independent" means different tokens; a single actor can obtain several (sybil), so review remains
the authority for sensitive entries. The schema is created or extended lazily
(`storage::ensure_provenance_schema`); `migrate-custom-annotations-db.sql` is the manual equivalent.

## Advanced Filtering

The `AdvancedSequenceFilter` supports:

| Filter        | Type      | Description                                |
|---------------|-----------|--------------------------------------------|
| `search`      | String    | Case-insensitive search in ID/gene/product |
| `min_length`  | usize     | Minimum sequence length                    |
| `max_length`  | usize     | Maximum sequence length                    |
| `cog`         | String    | COG category letter (A-Z)                  |
| `ec_class`    | String    | EC class prefix (1-7)                      |
| `has_gene`    | bool      | Only sequences with gene annotation        |
| `has_product` | bool      | Only sequences with product annotation     |

## Environment Variables

| Variable                        | Description                                                           | Default                            |
|---------------------------------|-----------------------------------------------------------------------|------------------------------------|
| `RUST_LOG`                      | Log level (trace, debug, info, warn, error)                           | `info`                             |
| `BAKTA_DB`                      | Path to Bakta database directory                                      | `/bakta-db`                        |
| `AI_DB_TEMP_DIR`                | Directory for temporary upload files                                  | `/tmp`                             |
| `AI_DB_JOBS_PATH`               | Path to SQLite jobs database                                          | `/data/jobs.db`                    |
| `AI_DB_PORT`                    | HTTP server port                                                      | `8000`                             |
| `AI_DB_HOST`                    | HTTP server bind address                                              | `0.0.0.0`                          |
| `AI_DB_CUSTOM_ANNOTATIONS_PATH` | Path to custom AI-DB database                                         | `/custom-db/custom_annotations.db` |
| `AI_DB_CONFIRMATIONS`           | Independent submissions needed to confirm                             | `2`                                |
| `AI_DB_INGEST_DAILY_LIMIT`      | Max contributions per contributor per 24 h                            | `200000`                           |
| `ADMIN_KPI_SECRET`              | Secret for `X-Admin-Secret` (admin routes disabled with 503 if unset) | unset                              |

## Usage

```bash
# Build
cargo build --release

# Run
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
cargo test --lib services::fasta
cargo test --lib export::gff3
cargo test --lib storage
cargo test --lib auth
```

## License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this Service by you,
as defined in the Apache-2.0 license, shall be dually licensed as above, without any additional terms or conditions.
