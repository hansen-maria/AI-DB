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


## Project Structure

```
ai-db-web/
├── docker-compose.yml          # Docker compose configuration
├── frontend/                   # Vue.js frontend
│   ├── Dockerfile
│   ├── nginx.conf              # Nginx configuration with API proxy
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
│   │   │   ├── JobDetailView.vue  # Job details
│   │   │   └── JobListView.vue    # Jobs list
│   │   └── assets/
│   │       └── main.css
│   └── public/
└── backend/                    # Rust/Axum Backend
    ├── Dockerfile
    ├── Cargo.toml
    └── src/
        └── main.rs            # REST API with OpenAPI/Swagger
```

## Quick Start

### Prerequisites

- Docker & Docker Compose
- Bakta database (Full or Light, but Full recommended) on a separate volume

### Setting up the Bakta Database

```bash
# Create and mount volume
sudo mkfs.ext4 <path/to/dest/partition>
sudo mkdir -p /mnt/bakta-db
sudo mount <path/to/dest/partition> /mnt/bakta-db

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
cargo run
# Server running at http://localhost:8000
# Swagger UI: http://localhost:8000/api/docs/
```

**Start frontend (Vue.js):**
```bash
cd frontend
npm install
npm run dev
```

### Production with Docker

```bash
docker-compose up -d --build
```

## REST API Endpoints

| Method   | Endpoint        | Description                 | Authentication       |
|----------|-----------------|-----------------------------|----------------------|
| `POST`   | `/api/job/`     | Create new job              | Sets cookie          |
| `GET`    | `/api/job/{id}` | Get job status              | Public (with Job-ID) |
| `GET`    | `/api/jobs/`    | List own jobs               | Cookie required      |
| `DELETE` | `/api/job/{id}` | Delete job                  | Own jobs only        |
| `GET`    | `/api/health`   | Health check with DB status | Public               |
| `GET`    | `/api/db/info`  | Database information        | Public               |

### Cookie-based Authorization

- On first job submission, an `ai_db_owner` cookie is automatically set (valid for 1 year)
- The job list shows only your own jobs
- Jobs can only be deleted by their creator
- Anyone with the Job-ID can access a job (for sharing)

### API Documentation

The complete OpenAPI/Swagger documentation is available at:
- **Swagger UI**: `https://ai-db.computational.bio/api/docs/`
- **OpenAPI JSON**: `https://ai-db.computational.bio/api/openapi.json`

### Example: Create job

**With file upload:**
```bash
curl -X POST "https://ai-db.computational.bio/api/job/" \
  -F "file=@sequences.fasta" \
  -F "job_name=MyJob" \
  -c cookies.txt
```

**With direct FASTA content:**
```bash
curl -X POST "https://ai-db.computational.bio/api/job/" \
  -F "fasta_content=>seq1
MKFLILLFNILCLFPVLAADNHGVGPQGASGVDPITFDINSNQTGV" \
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

### Example: Retrieve job status

```bash
curl "https://ai-db.computational.bio/api/job/550e8400-e29b-41d4-a716-446655440000"
```

**Response:**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "created_at": "2026-01-13T10:30:00Z",
  "updated_at": "2026-01-13T10:30:05Z",
  "filename": "sequences.fasta",
  "sequence_count": 100,
  "processed_count": 100,
  "hash_matches": 85,
  "alignment_matches": 0,
  "sequences": [
    {
      "id": "seq1",
      "md5_hash": "a1b2c3d4e5f6...",
      "length": 245,
      "annotation": "UniRef100:A0A003 | UniParc:UPI0000E5B23F | NCBI:WP_012345678.1",
      "annotation_source": "hash_match",
      "uniparc_id": "UPI0000E5B23F",
      "ncbi_nrp_id": "WP_012345678.1",
      "uniref100_id": "A0A003"
    }
  ]
}
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

| Route      | Component     | Description                                 |
|------------|---------------|---------------------------------------------|
| `/`        | HomeView      | Landing page with features                  |
| `/submit`  | SubmitJobView | FASTA upload page                           |
| `/jobs`    | JobListView   | List of own jobs                            |
| `/job/:id` | JobDetailView | Job details with clickable annotation links |
| `/docs`    | -             | Redirect to Swagger UI                      |

### Annotation Links

Found annotations are displayed as clickable links to the respective databases:

| Database     | URL Format                                      |
|--------------|-------------------------------------------------|
| UniRef100    | `https://www.uniprot.org/uniref/UniRef100_{id}` |
| UniParc      | `https://www.uniprot.org/uniparc/{id}`          |
| NCBI Protein | `https://www.ncbi.nlm.nih.gov/protein/{id}`     |

## Rust Backend

The backend uses the following crates:

| Crate           | Purpose                               |
|-----------------|---------------------------------------|
| **axum**        | Ergonomic web framework               |
| **axum-extra**  | Cookie handling                       |
| **rusqlite**    | SQLite access for Bakta-DB            |
| **utoipa**      | OpenAPI/Swagger documentation         |
| **tower-http**  | HTTP middleware (CORS, Tracing)       |
| **serde**       | JSON serialization                    |
| **tokio**       | Async runtime                         |
| **parking_lot** | Efficient locks for in-memory storage |
| **md-5**        | MD5 hash computation                  |

### Build

```bash
cd backend
cargo build --release
```

The release binary is located in `target/release/ai-db-api`.

## Configuration

### Environment variables

**Backend:**

| Variable   | Description                                 | Default     |
|------------|---------------------------------------------|-------------|
| `RUST_LOG` | Log level (trace, debug, info, warn, error) | `info`      |
| `BAKTA_DB` | Path to Bakta database                      | `/bakta-db` |

### Docker-Compose Volume Configuration

```yaml
services:
  api:
    volumes:
      - /mnt/bakta-db/db:/bakta-db:ro
    environment:
      - BAKTA_DB=/bakta-db
```

### Logo files

- `frontend/src/assets/logo-light.png` - Logo for light mode
- `frontend/src/assets/logo-dark.png` - Logo for dark mode
- `frontend/public/favicon.png` - Browser favicon

## Bakta Database

### Automatic Updates

Set up a daily update job:

```bash
# /etc/cron.d/bakta-db-update
0 3 * * * root /usr/local/bin/bakta-db-update.sh >> /var/log/bakta-db-update.log 2>&1
```

### SSL certificates

For the first certificate issuance:
```bash
docker-compose run --rm certbot certonly \
  --webroot \
  --webroot-path=/var/www/certbot \
  -d ai-db.computational.bio
```

## FASTA Format

The expected input format:
```
>sequence_id_1 optional description
MKFLILLFNILCLFPVLAADNHGVGPQGASGVDPITFDINSNQTGV
ASLLNFLGGTTVGSLQGKPLGQLACNPNQVKRKGDHIIYPGQQYTP
>sequence_id_2
MRYILAAVLLPMFAQSYKVDQTGSGPKNTFFINSNQTGVPEQYGDL
```

## Security

- HTTPS with Let's Encrypt
- HTTP-Only cookies with SameSite=Lax
- Security headers (HSTS, X-Frame-Options, etc.)
- CORS with explicit origins for credentials
- Non-root user in backend container
- Memory-safe backend through Rust
- Read-only Bakta-DB mount

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