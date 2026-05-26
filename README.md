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
- **User-friendly**: Jobs are associated with users via cookies
- **Shareable**: Jobs can be shared via Job-ID
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
        ├── auth.rs                     # Cookie-based authentication
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

### Cookie-based Authorization

- On first job submission, an `ai_db_owner` cookie is automatically set (valid for 1 year)
- The job list shows only your own jobs
- Jobs can only be deleted by their creator
- Analysis triggers (Psos/Bakta) and ingestion require ownership
- Downloads require ownership
- Anyone with the Job-ID can view a job (for sharing)

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
- HTTP-Only cookies with SameSite=Lax
- Security headers (HSTS, X-Frame-Options)
- CORS with explicit origins
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