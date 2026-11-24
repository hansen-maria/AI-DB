# Contributing to AI-DB

Thank you for your interest in AI-DB! Contributions of all kinds are welcome:
bug reports, feature ideas, documentation fixes, and code.

By participating in this project you agree to abide by our
[Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to contribute

- **Report a bug** – open a [bug report](https://github.com/hansen-maria/AI-DB/issues/new?template=bug_report.yml).
- **Report a wrong or incomplete annotation** – open an
  [annotation issue](https://github.com/hansen-maria/AI-DB/issues/new?template=annotation_issue.yml).
- **Suggest a feature** – open a [feature request](https://github.com/hansen-maria/AI-DB/issues/new?template=feature_request.yml).
- **Improve the documentation** – READMEs, the About/FAQ page, or API docs.
- **Submit code** – see the workflow below.

Please **do not** report security vulnerabilities in public issues. See
[SECURITY.md](SECURITY.md) instead.

For questions that are not bugs or feature requests, contact us at
ai-db@computational.bio.

## Before you start

For anything larger than a small fix, please open an issue first and describe
what you plan to do. This avoids duplicate work and lets us agree on the
approach before you invest time in it — especially for changes to the API
contract (`crates/aidb-types`), the database schemas, or the ingest logic.

## Repository layout

| Path                 | Content                                              |
|----------------------|------------------------------------------------------|
| `backend/`           | Rust/Axum API server (`ai-db-api`)                   |
| `crates/aidb-types/` | Shared API request/response types                    |
| `crates/aidb-client/`| Client library for the REST API                      |
| `crates/aidb-cli/`   | `aidb` command-line client                           |
| `frontend/`          | Vue 3 + TypeScript single-page application           |

See [`backend/README.md`](backend/README.md) and
[`frontend/README.md`](frontend/README.md) for details.

## Development setup

### Prerequisites

- Rust (stable) via [rustup](https://rustup.rs/)
- Node.js `^20.19.0` or `>=22.12.0` and npm
- Docker and Docker Compose (optional, for the full stack)
- A Bakta database for realistic lookups (set via `BAKTA_DB`, see
  [`backend/README.md`](backend/README.md#environment-variables))

### Run locally

```bash
# Full stack
docker compose up --build          # http://localhost:8080

# Backend only
cargo run -p ai-db-api

# Frontend only (separate terminal)
cd frontend
npm install
npm run dev
```

## Making changes

1. Fork the repository and create a branch from `main`
   (e.g. `fix/gff3-export-escaping` or `feat/cli-batch-submit`).
2. Make your changes in small, focused commits.
3. Add or update tests where it makes sense.
4. Update documentation (READMEs, OpenAPI annotations, About page) if behavior
   or the API changes.
5. Run the checks below.
6. Open a pull request and fill in the template.

### Checks

**Rust** (from the repository root):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

**Frontend** (in `frontend/`):

```bash
npm run type-check
npm run build
```

TypeScript is used in strict mode; please do not introduce `any` or
`@ts-ignore` without a good reason.

### Commit messages

- English, imperative mood, concise: `Add GFF3 escaping for product names`,
  not `Added ...` or `Fixes ...`.
- Keep the subject line under ~72 characters; add a body if the *why* is not
  obvious.
- Reference issues where relevant: `Fix pagination offset (#42)`.

### Code conventions

- **Rust**: `rustfmt` defaults, no `clippy` warnings. Prefer explicit error
  types (`thiserror`) over `unwrap()` in request paths.
- **Vue/TypeScript**: Composition API with `<script setup lang="ts">`. Keep
  views thin; put logic into composables (`src/composables/`) and focused child
  components.
- **Naming**: use the established terms consistently in code, logs, and UI text,
  e.g. *Bakta DB*, *AI-DB annotations DB*, *job owner*.
- **API changes**: request/response types live in `crates/aidb-types`. A
  breaking change must be reflected in `api_version` (reported by
  `GET /api/health`) and in the CLI/client crates.

## Data and privacy

AI-DB processes user sequences as MD5 hashes and only sends sequences to
external services (Bakta, Psos) when the job owner explicitly starts such an
analysis. Contributions must preserve this behavior. Please never commit real
user data, tokens, admin secrets, or database files.

## License

AI-DB is dual-licensed under [Apache-2.0](LICENSE-APACHE) and [MIT](LICENSE-MIT).
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in AI-DB by you shall be dual-licensed as above, without any
additional terms or conditions.
