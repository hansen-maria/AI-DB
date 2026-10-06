# AI-DB - Already Identified Database

Hash-Based Annotation Service for Microbial Sequencing Data

AI-DB accelerates the analysis of microbial sequencing data through cryptographic hash-based annotations
using the [Bakta](https://github.com/oschwengers/bakta) database (~350 million protein sequences) alongside a custom, 
user-expandable AI-DB Annotations Database.

## Features

- **Fast**: Hash-based annotations in seconds instead of hours
- **Privacy**: Sequence data is processed locally as MD5 hashes
- **Comprehensive**: Access to UniRef100, UniParc, and NCBI protein annotations
- **Extensible Knowledge Base**: Further analyze unmatched sequences using Psos and Bakta. 
    You can ingest new Bakta results directly into the custom AI-DB annotations database 
    to continuously build and grow the knowledge base.
- **Functional Analysis**: Interactive visualizations of COG categories, EC classes, and top genes/products
- **Advanced Search**: Real-time client-side filtering by sequence ID, gene name, product, length, and functional categories
- **Persistent**: Jobs are stored for 30 days
- **User-friendly**: Jobs are associated with users via cookies (browser) or an API token (scripts/pipelines)
- **Shareable**: Jobs can be shared via Job-ID (read-only for everyone except the owner)
- **Pipeline-ready**: Full REST API with OpenAPI spec; no cookie handling needed
- **Export**: Download results in TSV, JSON, FASTA, or GFF3 format
- **Pagination**: Efficient browsing of large result sets
- **Filtering**: Filter sequences by annotation source (hash match, no match)

## Project Structure

```
ai-db/
├── docker-compose.yml
├── frontend/                           # Vue.js Frontend
│   ├── Dockerfile
│   ├── nginx.conf
│   ├── package.json
│   ├── vite.config.ts
│   ├── src/
│   │   ├── App.vue                     # Main app with navigation
│   │   ├── main.ts                     # Entry point
│   │   ├── router/
│   │   │   └── index.ts
│   │   ├── api/
│   │   │   ├── bakta.ts                # API client for Bakta analysis
│   │   │   ├── jobs.ts                 # API client with types
│   │   │   └── psos.ts                 # API client for Psos analysis
│   │   ├── constants/
│   │   │   └── sequences.ts            # Shared filter options, COG/EC vocabularies,
│   │   │                               # color palettes, and DB link helpers
│   │   ├── composables/
│   │   │   ├── useJobPolling.ts        # Job fetching, background polling, stats
│   │   │   ├── useSequenceFilters.ts   # Client-side filtering, pagination, download
│   │   │   ├── usePsosAnalysis.ts      # Psos state, API calls, result persistence
│   │   │   └── useBaktaAnalysis.ts     # Bakta state, API calls, annotation ingest
│   │   ├── components/
│   │   │   └── job/
│   │   │       ├── AnalysisTab.vue     # Annotation rate ring + functional charts
│   │   │       ├── SequencesTab.vue    # Search bar, filter panel, sequence table
│   │   │       ├── PsosPanel.vue       # Collapsible Psos analysis section
│   │   │       └── BaktaPanel.vue      # Collapsible Bakta annotation section
│   │   ├── views/
│   │   │   ├── HomeView.vue            # Landing page
│   │   │   ├── ContactView.vue         # Contact page
│   │   │   ├── SubmitJobView.vue       # FASTA upload
│   │   │   ├── JobDetailView.vue       # Job details with tabs, search, analysis & ingestion
│   │   │   └── JobListView.vue         # Jobs list (own jobs)
│   │   └── assets/
│   │       ├── main.css
│   │       └── logo-*.png
│   └── public/
└── backend/                            # Rust/Axum Backend
    ├── Dockerfile
    ├── Cargo.toml
    └── src/
        ├── main.rs                     # Entry point, router, OpenAPI
        ├── auth.rs                     # Owner auth (cookie or API token), read-only sharing
        ├── state.rs                    # AppState, DB connection, job management
        ├── storage.rs                  # SQLite job persistence (30 days)
        ├── models/                     # Data structures
        │   ├── bakta.rs                # Models for Bakta analysis
        │   ├── custom_db.rs            # Models for custom AI-DB annotations
        │   ├── job.rs                  # JobResponse, JobStatus
        │   ├── psos.rs                 # Models for Psos analysis
        │   ├── sequence.rs             # SequenceInfo, SequenceFilter
        │   ├── pagination.rs           # PaginationInfo, query types
        │   ├── stats.rs                # FunctionalStats for analysis
        │   ├── health.rs               # Health check types
        │   └── error.rs                # ErrorResponse
        ├── handlers/                   # API endpoints
        │   ├── jobs.rs                 # CRUD operations for jobs
        │   ├── bakta.rs                # Bakta analysis & data ingestion endpoints
        │   ├── psos.rs                 # Psos analysis endpoints
        │   ├── stats.rs                # Functional analysis endpoint
        │   ├── download.rs             # Export handler
        │   └── health.rs               # Health check & DB info
        ├── services/                   # Business Logic
        │   ├── fasta.rs                # FASTA parsing & MD5 computation
        │   └── annotation.rs           # DB lookup (Bakta -> Custom DB), job processing
        └── export/                     # Download Formats
            ├── tsv.rs
            ├── json.rs
            ├── fasta.rs
            └── gff3.rs
```

## Components

### Backend (`backend/`)

| Technology        | Role                               |
|-------------------|------------------------------------|
| Rust + Axum       | HTTP API server                    |
| SQLite + rusqlite | Job persistence (30-day retention) |
| MD5 hashing       | Sequence identity lookup           |
| Bakta DB          | Annotation data source             |

See [`backend/README.md`](backend/README.md) for build instructions, API
reference, and Docker configuration.

### Frontend (`frontend/`)

| Technology         | Role                              |
|--------------------|-----------------------------------|
| Vue 3 + TypeScript | Single-page application           |
| Vue Router 4       | Client-side routing               |
| Vite               | Build tool and dev server         |
| Nginx              | Production web server + API proxy |

See [`frontend/README.md`](frontend/README.md) for setup instructions,
directory structure, and component documentation.

## Quick Start

### With Docker Compose

```bash
docker compose up --build
```

The application will be available at `http://localhost:8080`.

### Manual Setup

```bash
# Backend
cd backend
cargo build --release
./target/release/ai-db

# Frontend (separate terminal)
cd frontend
npm install
npm run dev
```

## Workflow

```
User uploads FASTA
       │
       ▼
Backend hashes each sequence (MD5)
       │
       ├─── Hash found ──► Return annotation from AI-DB
       │
       └─── No match   ──► Mark as unmatched
                               │
                               ├─ Psos API  (signal peptide, TM domains)
                               └─ Bakta API (full genome / protein annotation)
                                       │
                                       ▼
                               Ingest results into local AI-DB annotations DB
                               (future jobs recognize these sequences via hash)
```

### Result Download Formats

| Format   | Endpoint                       | Content-Type                | Use Case                       |
|----------|--------------------------------|-----------------------------|--------------------------------|
| TSV      | `/api/job/{id}/download/tsv`   | `text/tab-separated-values` | Excel, R, Python               |
| JSON     | `/api/job/{id}/download/json`  | `application/json`          | Programmatic access            |
| FASTA    | `/api/job/{id}/download/fasta` | `text/x-fasta`              | Bioinformatics tools           |
| GFF3     | `/api/job/{id}/download/gff3`  | `text/x-gff3`               | Genome browsers (IGV, JBrowse) |

### Authorization and Sharing

A job belongs to a random owner identifier (UUIDv4), transported in one of two ways:

- **Browser**: on first job submission, an HTTP-only `ai_db_user` cookie is set automatically (valid for 1 year).
- **API / pipelines**: generate your own UUIDv4 once (`uuidgen`) and send it with every request as
  `Authorization: Bearer <token>` or `X-API-Key: <token>`. No cookies are needed; no cookie is set.
  Only UUIDv4 values are accepted (anything else: `401` on job creation, anonymous otherwise).
  Treat the token like a password. If both token and cookie are sent, the token wins.
- **Storage**: the secret (cookie value or token) is **never stored**. The server keeps only a one-way digest
  (`md5("aidb-owner:" + secret)`, 32 hex characters) as `owner_id`, in the jobs DB and in the KPI counters.
  A database leak therefore does not reveal usable credentials. On startup, existing rows that still hold a
  raw UUID are converted automatically (idempotent). Contributor ids in the curation DB are a second digest of the owner digest.

```bash
TOKEN=$(uuidgen)
curl -H "Authorization: Bearer $TOKEN" -F "file=@proteins.faa" https://ai-db.computational.bio/api/job/
curl -H "Authorization: Bearer $TOKEN" -OJ https://ai-db.computational.bio/api/job/<id>/download/tsv
```

| Operation                                                              | Who                                                                        |
|------------------------------------------------------------------------|----------------------------------------------------------------------------|
| View job, sequences, stats, saved Psos/Bakta results                   | Anyone with the Job-ID (read-only; `GET /api/job/{id}` returns `is_owner`) |
| List jobs                                                              | Owner (only own jobs)                                                      |
| Download (TSV/JSON/FASTA/GFF3)                                         | Owner                                                                      |
| Delete, bulk delete, rename, retry                                     | Owner                                                                      |
| Save/delete Psos and Bakta state, ingest into the AI-DB annotations DB | Owner                                                                      |

Shared jobs are therefore **read-only** for everyone but the owner. Jobs created before token/ownership
enforcement may have no owner and can then no longer be modified (they expire after 30 days).

### Community curation of the AI-DB annotations DB

Annotations contributed from Bakta runs are **verified, versioned and curated**, not blindly trusted:

1. **Verification** – only hashes of the submitting job's unmatched sequences (or of its unreviewed community matches)
   are accepted; length and all fields (UniRef/EC/GO/COG formats, text without control characters) are validated.
   Everything else is rejected. Contributions are rate-limited per contributor (`AI_DB_INGEST_DAILY_LIMIT`).
2. **Provenance, no overwriting** – every contribution is stored with a pseudonymous contributor id (a one-way digest;
   the API token itself is never stored), job, timestamp, workflow (`bakta`/`baktfold`) and the PSC identity / e-value.
   An existing annotation is never overwritten by a single contribution.
3. **Consensus** – a new entry is `candidate`. When `AI_DB_CONFIRMATIONS` (default 2) independent contributors submit
   the *same* annotation it becomes `confirmed`; competing annotations make it `conflicted`; if a different annotation
   reaches the threshold on its own, it replaces the first-come one. Admins can confirm or reject entries
   (`GET /api/admin/annotations?status=…`, `POST /api/admin/annotations/{md5}/review` with
   `{"status":"confirmed|rejected|candidate"}`; header `X-Admin-Secret`, same secret as `ADMIN_KPI_SECRET`).
   Reviewed entries are locked against consensus changes. In the UI, job owners can opt in to **re-check
   unconfirmed community entries** with Bakta; an independent result counts as a vote. Entries from before curation tracking are `legacy`.

Lookups return the status as `annotation_status` (`confirmed`, `candidate`, `conflicted`, `legacy`; absent for Bakta DB
matches). The first start after the update migrates `custom_annotations.db` additively
(see `migrate-custom-annotations-db.sql`); existing data is kept.

Limits: "independent" means different API tokens / browsers, which raises the cost of manipulation but is not
identity-proof; and the status stored in a job is the status at lookup time (retry refreshes it).

### API Documentation

Full OpenAPI/Swagger documentation is available at:
- **Swagger UI**: `https://ai-db.computational.bio/api/docs/`
- **OpenAPI JSON**: `https://ai-db.computational.bio/api/openapi.json`

### Job Persistence

Jobs are stored in a SQLite database and **persist for 30 days**. 
The database survives container restarts and redeployments.

## Deployment

The application is designed for deployment on OpenStack or any container
platform. Both services publish Docker images via multi-stage builds.

### Nginx Proxy

The frontend Nginx configuration proxies `/api/` to the backend service.
Update `nginx.conf` to set your domain and backend address before deploying.

## Security

- HTTPS with Let's Encrypt
- HTTP-Only cookies with SameSite=Lax; API tokens are random UUIDv4 values (UUIDv4 only); only a one-way digest of the secret is stored
- State-changing endpoints are owner-only; shared Job-IDs are read-only
- Security headers (HSTS, X-Frame-Options)
- CORS mirrors the request origin with credentials; cookies are SameSite=Lax (not sent on cross-site requests) and API tokens are never sent automatically by browsers
- Non-root container user
- Memory-safe Rust backend
- Read-only Bakta database mount

## License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this Service by you, 
as defined in the Apache-2.0 license, shall be dually licensed as above, without any additional terms or conditions.

## Links

- [Bakta GitHub](https://github.com/oschwengers/bakta)
- [Bakta Database on Zenodo](https://zenodo.org/record/14916843)
- [UniProt](https://www.uniprot.org/)
- [NCBI Protein](https://www.ncbi.nlm.nih.gov/protein/)