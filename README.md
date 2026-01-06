# AI-DB - Already Identified Database

Hash-Based Annotation Service for Microbial Sequencing Data

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

### Development

**Start backend (Rust):**
```bash
cd backend
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

| Method   | Endpoint        | Description                                   |
|----------|-----------------|-----------------------------------------------|
| `POST`   | `/api/job/`     | Create new job (FASTA file or its content)    |
| `GET`    | `/api/job/{id}` | Retrieve job status and results by id         |
| `GET`    | `/api/jobs/`    | List all jobs                                 |
| `DELETE` | `/api/job/{id}` | Delete job by id                              |

### API Documentation

The complete OpenAPI/Swagger documentation is available at:
- **Swagger UI**: `https://ai-db.computational.bio/api/docs/`
- **OpenAPI JSON**: `https://ai-db.computational.bio/api/openapi.json`

### Example: Create job

**With file upload:**
```bash
curl -X POST "https://ai-db.computational.bio/api/job/" \
  -F "file=@sequences.fasta" \
  -F "job_name=MyJob"
```

**With direct FASTA content:**
```bash
curl -X POST "https://ai-db.computational.bio/api/job/" \
  -F "fasta_content=>seq1
MKFLILLFNILCLFPVLAADNHGVGPQGASGVDPITFDINSNQTGV"
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
  "created_at": "2025-01-05T10:30:00Z",
  "updated_at": "2025-01-05T10:35:00Z",
  "filename": "sequences.fasta",
  "sequence_count": 100,
  "processed_count": 100,
  "hash_matches": 85,
  "alignment_matches": 12,
  "sequences": [...]
}
```

## Frontend Routes

| Route      | Component     | Description                |
|------------|---------------|----------------------------|
| `/`        | HomeView      | Landing page with features |
| `/submit`  | SubmitJobView | FASTA upload page          |
| `/jobs`    | JobListView   | List of all jobs           |
| `/job/:id` | JobDetailView | Job details and results    |
| `/docs`    | -             | Redirect to Swagger UI     |

## Rust Backend

The backend uses the following crates:
- **axum** - Ergonomic web framework from the Tokio developers
- **utoipa** - OpenAPI/Swagger documentation
- **tower-http** - HTTP middleware (CORS, tracing)
- **serde** - JSON serialization
- **tokio** - Async runtime
- **parking_lot** - Efficient locks for in-memory storage

### Build

```bash
cd backend
cargo build --release
```

The release binary is located in `target/release/ai-db-api`.

## Configuration

### Environment variables

**Backend:**
- `RUST_LOG=info` - Log level (trace, debug, info, warn, error)

### Logo files

- `frontend/src/assets/logo-light.png` - Logo for light mode
- `frontend/src/assets/logo-dark.png` - Logo for dark mode
- `frontend/public/favicon.png` - Browser favicon

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
- Security headers (HSTS, X-Frame-Options, etc.)
- CORS configured (adjust in production!)
- Non-root user in the backend container
- Memory-safe backend thanks to Rust

## License

MIT License - See LICENSE file
