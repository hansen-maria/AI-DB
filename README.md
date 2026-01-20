# AI-DB - Already Identified Database

Hash-Based Annotation Service for Microbial Sequencing Data

AI-DB accelerates the analysis of microbial sequencing data through cryptographic hash-based annotations 
using the [Bakta](https://github.com/oschwengers/bakta) database (~350 million protein sequences).

## Features

- **Fast**: Hash-based annotations in seconds instead of hours
- **Privacy**: Sequence data is processed locally as MD5 hashes
- **Comprehensive**: Access to UniRef100, UniParc, and NCBI protein annotations
- **User-friendly**: Jobs are associated with users via cookies
- **Shareable**: Jobs can be shared via Job-ID
- **Export**: Download results in TSV, JSON, FASTA, or GFF3 format
- **Pagination**: Efficient browsing of large result sets
- **Filtering**: Filter sequences by annotation source (hash match, alignment, no match)

## Project Structure

```
ai-db/
├── docker-compose.yml
├── frontend/                       # Vue.js Frontend
│   ├── Dockerfile
│   ├── nginx.conf
│   ├── package.json
│   ├── vite.config.ts
│   ├── src/
│   │   ├── App.vue             # Main app with navigation
│   │   ├── main.ts             # Entry point
│   │   ├── router/             # Vue router
│   │   │   └── index.ts
│   │   ├── api/                # API client
│   │   │   └── jobs.ts
│   │   ├── views/                 # Pages
│   │   │   ├── HomeView.vue       # Landing page
│   │   │   ├── SubmitJobView.vue  # FASTA upload
│   │   │   ├── JobDetailView.vue  # Job details with annotation links
│   │   │   └── JobListView.vue    # Jobs list (own jobs)
│   │   └── assets/
│   │       └── main.css
│   └── public/
└── backend/                    # Rust/Axum Backend
    ├── Dockerfile
    ├── Cargo.toml
    └── src/
        ├── main.rs                 # Entry point, router, OpenAPI
        ├── auth.rs                 # Cookie-based authentication
        ├── state.rs                # AppState, DB connection
        ├── models/                 # Data structures
        │   ├── job.rs              # JobResponse, JobStatus
        │   ├── sequence.rs         # SequenceInfo, SequenceFilter
        │   ├── pagination.rs       # PaginationInfo, query types
        │   └── error.rs            # ErrorResponse
        ├── handlers/               # API endpoints
        │   ├── jobs.rs             # CRUD operations
        │   ├── download.rs         # Export handler
        │   └── health.rs           # Health check
        ├── services/               # Business logic
        │   ├── fasta.rs            # FASTA parsing
        │   └── annotation.rs       # DB lookup, job processing
        └── export/                 # Download formats
            ├── tsv.rs
            ├── json.rs
            ├── fasta.rs
            └── gff3.rs
```

## Quick Start

### Prerequisites

- Docker & Docker Compose
- Bakta database (Full or Light, but Full recommended) on a separate volume

### Setting up the Bakta Database

```bash
# Create and mount volume
sudo mkfs.ext4 /dev/sdb
sudo mkdir -p /mnt/bakta-db
sudo mount /dev/sdb /mnt/bakta-db

# Download database (~40GB for Full DB)
cd /mnt/bakta-db
sudo curl -L -o db.tar.xz https://zenodo.org/record/14916843/files/db.tar.xz
sudo tar -xJf db.tar.xz
sudo rm db.tar.xz

# Initialize AMRFinderPlus
docker run --rm -v /mnt/bakta-db:/db --entrypoint /bin/bash oschwengers/bakta:latest \
    -c "amrfinder_update --force_update --database /db/db/amrfinderplus-db/"
```

### Production with Docker

```bash
docker-compose up -d --build
```

### Development

**Start backend (Rust):**
```bash
cd backend
BAKTA_DB=/mnt/bakta-db/db cargo run
# Server runs on http://localhost:8000
# Swagger UI: http://localhost:8000/api/docs/
```

**Start frontend (Vue.js):**
```bash
cd frontend
npm install
npm run dev
```

## REST API Endpoints

| Method   | Endpoint                          | Description                 | Authentication   |
|----------|-----------------------------------|-----------------------------|------------------|
| `POST`   | `/api/job/`                       | Create new job              | Sets cookie      |
| `GET`    | `/api/job/{id}`                   | Get job status (paginated)  | Public           |
| `GET`    | `/api/job/{id}/download/{format}` | Download results            | Owner only       |
| `GET`    | `/api/jobs/`                      | List own jobs (paginated)   | Cookie required  |
| `DELETE` | `/api/job/{id}`                   | Delete job                  | Owner only       |
| `GET`    | `/api/health`                     | Health check with DB status | Public           |
| `GET`    | `/api/db/info`                    | Database information        | Public           |

### Pagination Parameters

Both `/api/job/{id}` and `/api/jobs/` support pagination:

| Parameter   | Description             | Default  | Max  |
|-------------|-------------------------|----------|------|
| `page`      | Page number (1-indexed) | 1        | -    |
| `per_page`  | Items per page          | 20       | 100  |

### Filtering (Job Details)

The `/api/job/{id}` endpoint supports sequence filtering:

| Parameter   | Values       | Description                           |
|-------------|--------------|---------------------------------------|
| `filter`    | `all`        | All sequences (default)               |
|             | `hash_match` | Only sequences with hash matches      |
|             | `alignment`  | Only sequences with alignment matches |
|             | `none`       | Only sequences without annotations    |

### Download Formats

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
- Downloads require ownership
- Anyone with the Job-ID can view a job (for sharing)

### API Documentation

Full OpenAPI/Swagger documentation is available at:
- **Swagger UI**: `https://ai-db.computational.bio/api/docs/`
- **OpenAPI JSON**: `https://ai-db.computational.bio/api/openapi.json`

### Example: Create Job

```bash
curl -X POST "https://ai-db.computational.bio/api/job/" \
  -F "file=@sequences.fasta" \
  -F "job_name=MyJob" \
  -c cookies.txt
```

**Response:**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "pending",
  "message": "Job successfully created. Processing started.",
  "sequence_count": 1
}
```

### Example: Get Job with Pagination and Filter

```bash
curl "https://your-domain/api/job/550e8400-e29b-41d4-a716-446655440000?page=1&per_page=50&filter=hash_match"
```

**Response:**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "created_at": "2026-01-13T10:30:00Z",
  "updated_at": "2026-01-13T10:30:05Z",
  "filename": "sequences.fasta",
  "sequence_count": 4155,
  "processed_count": 4155,
  "hash_matches": 4111,
  "alignment_matches": 0,
  "filter": "hash_match",
  "filtered_count": 4111,
  "sequences": [...],
  "pagination": {
    "page": 1,
    "per_page": 50,
    "total_items": 4111,
    "total_pages": 83,
    "has_next": true,
    "has_prev": false
  }
}
```

### Example: Download Results

```bash
# Download as TSV
curl -b cookies.txt -O "https://ai-db.computational.bio/api/job/{id}/download/tsv"

# Download as GFF3
curl -b cookies.txt -O "https://ai-db.computational.bio/api/job/{id}/download/gff3"
```

### Example: Check Database Status

```bash
curl "https://ai-db.computational.bio/api/db/info"
```

**Response:**
```json
{
  "available": true,
  "path": "/bakta-db/bakta.db",
  "ups_entries": 351847263,
  "version": "6.0"
}
```

## Frontend Routes

| Route      | Component     | Description                           |
|------------|---------------|---------------------------------------|
| `/`        | HomeView      | Landing page with features            |
| `/submit`  | SubmitJobView | FASTA upload page                     |
| `/jobs`    | JobListView   | Paginated list of own jobs            |
| `/job/:id` | JobDetailView | Job details with filtering & download |
| `/docs`    | -             | Redirect to Swagger UI                |

### Annotation Links

Found annotations are displayed as clickable links to the respective databases:

| Database     | URL Format                                      |
|--------------|-------------------------------------------------|
| UniRef100    | `https://www.uniprot.org/uniref/UniRef100_{id}` |
| UniParc      | `https://www.uniprot.org/uniparc/{id}`          |
| NCBI Protein | `https://www.ncbi.nlm.nih.gov/protein/{id}`     |

## Rust Backend

### Modular Architecture

The backend is organized into focused modules:

| Module      | Responsibility                           |
|-------------|------------------------------------------|
| `models/`   | Data structures, serialization           |
| `handlers/` | HTTP request/response handling           |
| `services/` | FASTA parsing, DB lookup, job processing |
| `export/`   | TSV, JSON, FASTA, GFF3 generation        |
| `state.rs`  | Application state, DB connection         |
| `auth.rs`   | Cookie-based authentication              |

### Build

```bash
cd backend
cargo build --release
```

The release binary is located at `target/release/ai-db-api`.

## Configuration

### Environment Variables

| Variable         | Description                                 | Default     |
|------------------|---------------------------------------------|-------------|
| `RUST_LOG`       | Log level (trace, debug, info, warn, error) | `info`      |
| `BAKTA_DB`       | Path to Bakta database                      | `/bakta-db` |
| `AI_DB_TEMP_DIR` | Directory for temporary upload files        | `/tmp`      |

### Docker-Compose Volume Configuration

```yaml
services:
  api:
    volumes:
      - /mnt/bakta-db/db:/bakta-db:ro      # Bakta database (read-only)
      - /mnt/ai-db-tmp:/tmp-data           # Temp storage for uploads
    environment:
      - BAKTA_DB=/bakta-db
      - AI_DB_TEMP_DIR=/tmp-data
```

### Logo Files

- `frontend/src/assets/logo-light.png` - Logo for light mode
- `frontend/src/assets/logo-dark.png` - Logo for dark mode
- `frontend/public/favicon.png` - Browser favicon

## FASTA Format

Expected input format (protein sequences):
```
>sequence_id_1 optional description
MKFLILLFNILCLFPVLAADNHGVGPQGASGVDPITFDINSNQTGV
ASLLNFLGGTTVGSLQGKPLGQLACNPNQVKRKGDHIIYPGQQYTP
>sequence_id_2
MRYILAAVLLPMFAQSYKVDQTGSGPKNTFFINSNQTGVPEQYGDL
```

### Supported File Formats

| Extension                       | Description           |
|---------------------------------|-----------------------|
| `.fasta`, `.fa`, `.fna`, `.faa` | Standard FASTA files  |
| `.txt`                          | Plain text FASTA      |
| `.gz`                           | Gzip compressed FASTA |

Compressed files are automatically detected via magic bytes.

## Export Formats

### TSV (Tab-Separated Values)

Includes metadata header and data columns:
- `sequence_id`, `length`, `md5_hash`, `annotation_source`
- `annotation`, `uniparc_id`, `ncbi_nrp_id`, `uniref100_id`
- Direct URLs to UniParc, NCBI, and UniRef100

### JSON

Structured export with:
- Job metadata (ID, filename, timestamps)
- Statistics (total, hash matches, alignment matches, no matches)
- Sequences with database IDs and URLs

### Annotated FASTA

Original sequences with annotation info in headers:
```
>WP_000001234.1 | source=hash_match | annotation=UniRef100:A0A003 | UniParc=UPI000 | length=486 | md5=a1b2c3
MKFLILLFNILCLFPVLAADNHGVGPQGASGVDPITFDINSNQTGVASLLNFLGGTTVGS
```

### GFF3

Standard genome feature format for genome browsers:
```
##gff-version 3
#!annotation-source AI-DB v1.0
##sequence-region WP_000001234.1 1 486
###
WP_000001234.1	ai-db	protein_match	1	486	.	.	.	ID=seq_000001;Name=WP_000001234.1;md5=a1b2c3;Dbxref=UniParc:UPI000,UniRef100:A0A003
```

## Bakta Database

### Automatic Updates

Set up a daily update job:

```bash
# /etc/cron.d/bakta-db-update
0 3 * * * root /usr/local/bin/bakta-db-update.sh >> /var/log/bakta-db-update.log 2>&1
```

## ⚡ Performance & Memory Management

- **Streaming Upload**: Files streamed to temp storage
- **Iterator-based Parsing**: One sequence at a time
- **Batch Progress**: Updates every 1,000 sequences
- **Memory Limits**:
    - 100 MB maximum upload size 
    - 1M results max per job
    - 5 MB max per sequence
- **Gzip Support**: On-the-fly decompression


### Temp Volume Setup

A dedicated volume is required for temporary file storage:

```bash
# Format and mount temp volume
sudo mkfs.ext4 <path/to/dest/partition>
sudo mkdir -p /mnt/ai-db-tmp
sudo mount <path/to/dest/partition> /mnt/ai-db-tmp
sudo chmod 1777 /mnt/ai-db-tmp

# Add to /etc/fstab for persistence
echo '<path/to/dest/partition> /mnt/ai-db-tmp ext4 defaults 0 2' | sudo tee -a /etc/fstab
```

## 🛡️ Security

- HTTPS with Let's Encrypt
- HTTP-Only cookies with SameSite=Lax
- Security headers (HSTS, X-Frame-Options)
- CORS with explicit origins
- Non-root container user
- Memory-safe Rust backend
- Read-only database mount

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