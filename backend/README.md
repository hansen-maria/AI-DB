# AI-DB Backend

Rust/Axum backend for the AI-DB Hash-Based Annotation Service.

## Directory Structure

```
backend/src/
├── main.rs              # Entry point, router setup 
├── auth.rs              # Cookie-based authentication
├── state.rs             # AppState, DB connection, job management
├── storage.rs           # SQLite persistence (30-day retention)
│
├── models/              # Data structures 
│   ├── mod.rs           
│   ├── bakta.rs         # Data models for Bakta job persistence
│   ├── custom_db.rs     # Data models for AI-DB Annotations
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
│   ├── bakta.rs         # save_bakta_job, get_bakta_job, delete_bakta_job
│   ├── download.rs      # download_job
│   ├── health.rs        # health_check, db_info
│   ├── jobs.rs          # get_job, create_job, list_jobs, delete_job
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
| **auth.rs**    | Cookie-based owner identification                    |
| **state.rs**   | Shared state, database connection, job cache         |
| **storage.rs** | SQLite persistence, 30-day retention, cleanup        |
| **models/**    | All data structures and types                        |
| **handlers/**  | HTTP request/response handling                       |
| **services/**  | Core business logic (FASTA parsing, DB lookup)       |
| **export/**    | Output format generation                             |

## Job Persistence

Jobs are persisted to SQLite and survive container restarts:

- **Location**: `/data/jobs.db` (configurable via `AI_DB_JOBS_PATH`)
- **Retention**: 30 days (automatic cleanup on startup)
- **Storage**: Jobs serialized as JSON in SQLite

### Database Schema

```sql
CREATE TABLE IF NOT EXISTS jobs (
    job_id TEXT PRIMARY KEY,
    owner_id TEXT,
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

| Method   | Endpoint                          | Handler                                 |
|----------|-----------------------------------|-----------------------------------------|
| `JOBS`   |                                   |                                         |
| `POST`   | `/api/job/`                       | `handlers::jobs::create_job`            |
| `GET`    | `/api/job/{id}`                   | `handlers::jobs::get_job`               |
| `DELETE` | `/api/job/{id}`                   | `handlers::jobs::delete_job`            |
| `GET`    | `/api/job/{id}/download/{format}` | `handlers::download::download_job`      |
| `GET`    | `/api/job/{id}/stats`             | `handlers::stats::get_job_stats`        |
| `GET`    | `/api/jobs/`                      | `handlers::jobs::list_jobs`             |
| `PSOS`   |                                   |                                         |
| `GET`    | `/api/job/{id}/psos`              | `handlers::psos::get_psos_results`      |
| `POST`   | `/api/job/{id}/psos`              | `handlers::psos::save_psos_results`     |
| `DELETE` | `/api/job/{id}/psos`              | `handlers::psos::delete_psos_results`   |
| `BAKTA`  |                                   |                                         |
| `GET`    | `/api/job/{id}/bakta`             | `handlers::bakta::get_bakta_job`        |
| `POST`   | `/api/job/{id}/bakta`             | `handlers::bakta::save_bakta_job`       |
| `DELETE` | `/api/job/{id}/bakta`             | `handlers::bakta::delete_bakta_job`     |
| `POST`   | `/api/job/{id}/bakta/ingest`      | `handlers::bakta::ingest_bakta_results` |
| `HEALTH` |                                   |                                         |
| `GET`    | `/api/health`                     | `handlers::health::health_check`        |
| `GET`    | `/api/db/info`                    | `handlers::health::db_info`             |

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
      2. MD5(seq) → hash in AI-DB?           → Annotation, else Step 3
      3. No match
```

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

| Variable                        | Description                                 | Default                            |
|---------------------------------|---------------------------------------------|------------------------------------|
| `RUST_LOG`                      | Log level (trace, debug, info, warn, error) | `info`                             |
| `BAKTA_DB`                      | Path to Bakta database directory            | `/bakta-db`                        |
| `AI_DB_TEMP_DIR`                | Directory for temporary upload files        | `/tmp`                             |
| `AI_DB_JOBS_PATH`               | Path to SQLite jobs database                | `/data/jobs.db`                    |
| `AI_DB_PORT`                    | HTTP server port                            | `8000`                             |
| `AI_DB_HOST`                    | HTTP server bind address                    | `0.0.0.0`                          |
| `AI_DB_CUSTOM_ANNOTATIONS_PATH` | Path to custom AI-DB database               | `/custom-db/custom_annotations.db` |

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
```

## License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this Service by you,
as defined in the Apache-2.0 license, shall be dually licensed as above, without any additional terms or conditions.
