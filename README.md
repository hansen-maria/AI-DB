# AI-DB - Already Identified Database

Hash-Based Annotation Service for Microbial Sequencing Data

AI-DB accelerates the analysis of microbial sequencing data through cryptographic hash-based annotations
using the [Bakta](https://github.com/oschwengers/bakta) database (~350 million protein sequences).

## Features

- **Fast**: Hash-based annotations in seconds instead of hours
- **Privacy**: Sequence data is processed locally as MD5 hashes
- **Comprehensive**: Access to UniRef100, UniParc, and NCBI protein annotations
- **Functional Analysis**: Interactive visualizations of COG categories, EC classes, and top genes/products
- **Advanced Search**: Real-time client-side filtering by sequence ID, gene name, product, length, and functional categories
- **Persistent**: Jobs are stored for 30 days and survive container restarts
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
│   │   ├── App.vue                 # Main app with navigation
│   │   ├── main.ts                 # Entry point
│   │   ├── router/
│   │   │   └── index.ts
│   │   ├── api/
│   │   │   └── jobs.ts             # API client with types
│   │   ├── views/
│   │   │   ├── HomeView.vue        # Landing page
│   │   │   ├── SubmitJobView.vue   # FASTA upload
│   │   │   ├── JobDetailView.vue   # Job details with tabs, search & filter
│   │   │   └── JobListView.vue     # Jobs list (own jobs)
│   │   └── assets/
│   │       └── main.css
│   └── public/
└── backend/                        # Rust/Axum Backend
    ├── Dockerfile
    ├── Cargo.toml
    └── src/
        ├── main.rs                 # Entry point, router, OpenAPI
        ├── auth.rs                 # Cookie-based authentication
        ├── state.rs                # AppState, DB connection
        ├── storage.rs              # SQLite job persistence
        ├── models/
        │   ├── job.rs              # JobResponse, JobStatus
        │   ├── sequence.rs         # SequenceInfo, SequenceFilter
        │   ├── pagination.rs       # PaginationInfo, query types
        │   ├── stats.rs            # FunctionalStats for analysis
        │   └── error.rs            # ErrorResponse
        ├── handlers/
        │   ├── jobs.rs             # CRUD operations
        │   ├── stats.rs            # Functional analysis endpoint
        │   ├── download.rs         # Export handler
        │   └── health.rs           # Health check
        ├── services/
        │   ├── fasta.rs            # FASTA parsing
        │   └── annotation.rs       # DB lookup, job processing
        └── export/
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
| `GET`    | `/api/job/{id}/stats`             | Get functional analysis     | Public           |
| `GET`    | `/api/job/{id}/download/{format}` | Download results            | Owner only       |
| `GET`    | `/api/jobs/`                      | List own jobs (paginated)   | Cookie required  |
| `DELETE` | `/api/job/{id}`                   | Delete job                  | Owner only       |
| `GET`    | `/api/health`                     | Health check with DB status | Public           |
| `GET`    | `/api/db/info`                    | Database information        | Public           |

### Pagination Parameters

Both `/api/job/{id}` and `/api/jobs/` support pagination:

| Parameter   | Description             | Default  | Max   |
|-------------|-------------------------|----------|-------|
| `page`      | Page number (1-indexed) | 1        | -     |
| `per_page`  | Items per page          | 20       | 10000 |

### Filtering (Job Details)

The `/api/job/{id}` endpoint supports sequence filtering:

| Parameter     | Values                                   | Description                   |
|---------------|------------------------------------------|-------------------------------|
| `filter`      | `all`, `hash_match`, `alignment`, `none` | Filter by annotation source   |
| `search`      | text                                     | Search in ID, gene, product   |
| `min_length`  | number                                   | Minimum sequence length       |
| `max_length`  | number                                   | Maximum sequence length       |
| `cog`         | A-Z                                      | COG functional category       |
| `ec_class`    | 1-7                                      | Enzyme class                  |
| `has_gene`    | true/false                               | Only sequences with gene name |
| `has_product` | true/false                               | Only sequences with product   |

### Functional Analysis

The `/api/job/{id}/stats` endpoint returns:

```json
{
  "total_sequences": 4155,
  "annotated_sequences": 4111,
  "top_genes": [{"name": "rpsA", "count": 42}, ...],
  "top_products": [{"name": "hypothetical protein", "count": 156}, ...],
  "cog_categories": [{"code": "J", "name": "Translation", "count": 89}, ...],
  "ec_classes": [{"name": "2 - Transferases", "count": 234}, ...],
  "go_terms": {
    "molecular_function": [{"name": "GO:0003735", "count": 45}, ...]
  }
}
```

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

### Example: Get Functional Statistics

```bash
curl "https://ai-db.computational.bio/api/job/{id}/stats"
```

## Frontend Features

### Job Detail View

The job detail page features three tabs:

#### Overview Tab
- Job metadata (ID, filename, timestamps)
- Processing statistics
- Download options (TSV, JSON, FASTA, GFF3)

#### Sequences Tab
- **Real-time search**: Filter by sequence ID, gene name, or product description
- **Advanced filters**: Length range, COG category, EC class, annotation status
- **Client-side filtering**: Instant results without server requests
- **Pagination**: Navigate through large result sets
- Clickable database links (UniParc, UniRef100, NCBI)

#### Functional Analysis Tab
- **Annotation Rate**: Visual progress indicator
- **Top Genes**: Horizontal bar chart with sequential color palette
- **Top Products**: Most common functional descriptions
- **COG Categories**: Distribution across 23 functional categories
- **EC Classes**: Enzyme classification distribution
- **GO Terms**: Gene Ontology molecular functions

## Job Persistence

Jobs are stored in a SQLite database and **persist for 30 days**. The database survives container restarts and redeployments.

### Volume Configuration

```yaml
services:
  api:
    volumes:
      - jobs-data:/data  # Named volume for persistence

volumes:
  jobs-data:  # Docker-managed volume
```

### Environment Variables

| Variable           | Description                                 | Default         |
|--------------------|---------------------------------------------|-----------------|
| `RUST_LOG`         | Log level (trace, debug, info, warn, error) | `info`          |
| `BAKTA_DB`         | Path to Bakta database                      | `/bakta-db`     |
| `AI_DB_TEMP_DIR`   | Directory for temporary upload files        | `/tmp`          |
| `AI_DB_JOBS_PATH`  | Path to SQLite jobs database                | `/data/jobs.db` |

### Verifying Persistence

After deployment, check the logs:
```bash
docker logs ai-db-api | grep "Loaded.*jobs from database"
# Should show: "Loaded X existing jobs from database"
```

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

## Performance & Memory Management

- **Streaming Upload**: Files streamed to temp storage
- **Iterator-based Parsing**: One sequence at a time
- **Client-side Filtering**: Instant search without server requests
- **Batch Progress**: Updates every 1,000 sequences
- **Memory Limits**:
  - 100 MB maximum upload size
  - 1M results max per job
  - 5 MB max per sequence

## Security

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