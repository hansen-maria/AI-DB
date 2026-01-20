# AI-DB Frontend

Vue.js 3 frontend for the AI-DB Hash-Based Annotation Service.

## Tech Stack

| Technology       | Purpose                                |
|------------------|----------------------------------------|
| **Vue 3**        | Progressive JavaScript framework       |
| **Vue Router 4** | Client-side routing with history mode  |
| **TypeScript**   | Type-safe JavaScript                   |
| **Vite**         | Fast development server and build tool |
| **Nginx**        | Production web server with API proxy   |

## Directory Structure

```
frontend/
├── Dockerfile                 # Multi-stage Docker build
├── nginx.conf                 # Nginx configuration
├── package.json               # Dependencies and scripts
├── vite.config.ts             # Vite configuration
├── tsconfig.json              # TypeScript configuration
├── index.html                 # HTML entry point
├── public/
│   └── favicon.png            # Browser favicon
└── src/
    ├── main.ts                # Application entry point
    ├── App.vue                # Root component with navigation
    ├── router/
    │   └── index.ts           # Route definitions
    ├── api/
    │   └── jobs.ts            # API client with TypeScript types
    ├── views/
    │   ├── HomeView.vue       # Landing page
    │   ├── SubmitJobView.vue  # FASTA upload form
    │   ├── JobDetailView.vue  # Job results with filtering & download
    │   └── JobListView.vue    # Paginated job list
    └── assets/
        ├── main.css           # Global styles
        ├── logo-light.png     # Logo for light mode
        └── logo-dark.png      # Logo for dark mode
```

## Getting Started

### Prerequisites

- Node.js 18+ (LTS recommended)
- npm or yarn

### Installation

```bash
cd frontend
npm install
```

### Development

```bash
# Start development server on port 8080
npm run dev

# Type checking
npm run type-check
```

The development server runs at `http://localhost:8080` with hot module replacement.

**Note:** For API access during development, either:
- Run the backend locally on port 8000
- Configure a proxy in `vite.config.ts`

### Production Build

```bash
# Build for production
npm run build

# Preview production build
npm run preview
```

The build output is in the `dist/` directory.

## Docker

### Build Image

```bash
docker build -t ai-db-frontend .
```

### Multi-Stage Build

The Dockerfile uses a multi-stage build:

1. **Build Stage**: Node.js image for `npm run build`
2. **Production Stage**: Nginx Alpine for serving static files

### Run Container

```bash
docker run -p 80:80 ai-db-frontend
```

## Views

### HomeView (`/`)

Landing page featuring:
- Hero section with call-to-action
- Feature highlights
- How-it-works explanation
- Technology overview

### SubmitJobView (`/submit`)

FASTA submission form with:
- File upload (drag & drop supported)
- Direct text input
- Gzip support (automatic detection)
- Real-time validation
- Progress indication
- Auto-redirect to job details

### JobDetailView (`/job/:id`)

Job results page featuring:
- Job status with color-coded badges
- Statistics overview (total, hash matches, alignment matches)
- **Sequence filtering** by annotation source:
    - All sequences
    - Hash matches only
    - Alignment matches only
    - No matches only
- **Pagination** for large result sets
- **Download section** with 4 export formats:
    - TSV (spreadsheets)
    - JSON (programming)
    - FASTA (bioinformatics)
    - GFF3 (genome browsers)
- Clickable database links (UniParc, UniRef100, NCBI)
- Delete job functionality
- Auto-polling for pending/processing jobs

### JobListView (`/jobs`)

Job history page with:
- Paginated job list
- Status indicators
- Quick actions (view, delete)
- Empty state for new users

## API Client

The API client (`src/api/jobs.ts`) provides:

### Types

```typescript
type JobStatus = 'pending' | 'processing' | 'completed' | 'failed';
type SequenceFilter = 'all' | 'hash_match' | 'alignment' | 'none';
type DownloadFormat = 'tsv' | 'json' | 'fasta' | 'gff3';

interface SequenceInfo { ... }
interface PaginationInfo { ... }
interface PaginatedJobResponse { ... }
interface PaginatedJobsResponse { ... }
```

### Functions

```typescript
// Create job with file upload
createJobWithFile(file: File, jobName?: string): Promise<JobCreateResponse>

// Create job with FASTA content
createJobWithContent(content: string, jobName?: string): Promise<JobCreateResponse>

// Get job with pagination and filtering
getJob(jobId: string, page?: number, perPage?: number, filter?: SequenceFilter): Promise<PaginatedJobResponse>

// List all jobs (paginated)
listJobs(page?: number, perPage?: number): Promise<PaginatedJobsResponse>

// Delete job
deleteJob(jobId: string): Promise<void>

// Poll until completion
pollJobUntilComplete(jobId: string, onUpdate?: callback): Promise<PaginatedJobResponse>

// Download results
downloadJobResults(jobId: string, format: DownloadFormat): Promise<void>
```

### Features

- Automatic cookie handling (`credentials: 'include'`)
- Content-type validation (prevents HTML parsing errors)
- Error handling with typed responses
- Download with filename extraction from headers

## Routing

| Route      | View          | Description            |
|------------|---------------|------------------------|
| `/`        | HomeView      | Landing page           |
| `/submit`  | SubmitJobView | Job submission         |
| `/job/:id` | JobDetailView | Job results            |
| `/jobs`    | JobListView   | Job history            |
| `/docs`    | -             | Redirect to Swagger UI |

### Features

- History mode (clean URLs)
- Dynamic page titles
- Scroll behavior management
- Lazy loading for non-critical views

## Styling

### CSS Variables

The application uses CSS custom properties for theming:

```css
:root {
  --color-primary: #10b981;
  --color-background: #ffffff;
  --color-text: #1a1a1a;
  /* ... */
}

@media (prefers-color-scheme: dark) {
  :root {
    --color-background: #1a1a1a;
    --color-text: #ffffff;
    /* ... */
  }
}
```

### Status Colors

```typescript
const statusColors = {
  pending: '#ff9800',    // Orange
  processing: '#2196f3', // Blue
  completed: '#4caf50',  // Green
  failed: '#f44336'      // Red
}
```

## Nginx Configuration

The `nginx.conf` handles:

### API Proxy

```nginx
location ^~ /api/ {
    proxy_pass http://api:8000;
    client_max_body_size 100M;  # Large file uploads
    proxy_read_timeout 3600s;   # Long-running requests
}
```

### SPA Fallback

```nginx
location / {
    try_files $uri $uri/ /index.html;
}
```

### HTTPS & Security

- TLS 1.2/1.3 only
- HSTS headers
- X-Frame-Options
- X-Content-Type-Options

### Caching

- Static assets: 1 year (`immutable`)
- HTML: No caching

## 🔧 Customization

### Logo

Replace these files with your own logos:
- `src/assets/logo-light.png` - Logo for light mode
- `src/assets/logo-dark.png` - Logo for dark mode
- `public/favicon.png` - Browser favicon

### Domain

Update `nginx.conf` to replace `ai-db.computational.bio` with your domain.

### Colors

Edit CSS variables in `src/assets/main.css` to customize the color scheme.

## 📦 Scripts

| Script               | Description              |
|----------------------|--------------------------|
| `npm run dev`        | Start development server |
| `npm run build`      | Production build         |
| `npm run preview`    | Preview production build |
| `npm run type-check` | TypeScript validation    |

## Browser Support

- Chrome (latest)
- Firefox (latest)
- Safari (latest)
- Edge (latest)

Requires ES2020+ support (async/await, optional chaining).

## License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this Service by you,
as defined in the Apache-2.0 license, shall be dually licensed as above, without any additional terms or conditions.
