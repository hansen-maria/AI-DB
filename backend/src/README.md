# AI-DB Backend - Modular Structure

## Directory Structure

```
backend/src/
├── main.rs              # Entry point, router setup 
├── auth.rs              # Cookie-based authentication
├── state.rs             # AppState, DB connection
├── openapi.rs           # Swagger/OpenAPI config
│
├── models/              # Data structures 
│   ├── mod.rs           
│   ├── error.rs         # ErrorResponse
│   ├── job.rs           # JobResponse, JobStatus, JobSummary, JobCreateResponse
│   ├── sequence.rs      # SequenceInfo, HashLookupResult, SequenceFilter
│   └── pagination.rs    # PaginationInfo, PaginatedJobsResponse, Query types
│
├── handlers/            # API endpoints 
│   ├── mod.rs           
│   ├── jobs.rs          # get_job, create_job, list_jobs, delete_job
│   ├── download.rs      # download_job
│   └── health.rs        # health_check, db_info
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

| Module         | Purpose                                        |
|----------------|------------------------------------------------|
| **main.rs**    | Application entry point, router configuration  |
| **auth.rs**    | Cookie-based owner identification              |
| **state.rs**   | Shared state, database connection pool         |
| **openapi.rs** | Swagger documentation                          |
| **models/**    | All data structures and types                  |
| **handlers/**  | HTTP request/response handling                 |
| **services/**  | Core business logic (FASTA parsing, DB lookup) |
| **export/**    | Output format generation                       |


## Usage

```bash
# Build
cargo build --release

# Run
cargo run

# Run with logging
RUST_LOG=debug cargo run

# With custom temp directory
AI_DB_TEMP_DIR=/mnt/ai-db-tmp cargo run

# With Bakta database
BAKTA_DB=/path/to/bakta cargo run
```

## Testing

```bash
# Run all tests
cargo test

# Run tests for specific module
cargo test --lib services::fasta
cargo test --lib export::gff3
```