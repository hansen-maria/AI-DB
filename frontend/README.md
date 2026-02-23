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
    │   ├── JobDetailView.vue  # Job results with tabs, search & analysis
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

### Production Build

```bash
npm run build
```

## Views

### HomeView (`/`)

Landing page featuring:
- Hero section with call-to-action
- Feature highlights
- How-it-works explanation

### SubmitJobView (`/submit`)

FASTA submission form with:
- File upload (drag & drop supported)
- Direct text input
- Gzip support (automatic detection)
- Real-time validation
- Auto-redirect to job details

### JobDetailView (`/job/:id`)

**Three-tab interface:**

#### Overview Tab
- Job metadata (ID, filename, timestamps)
- Processing statistics (total, hash matches, alignment matches)
- Download section with 4 export formats

#### Sequences Tab

**Search & Filter Bar:**
- Real-time text search (ID, gene, product)
- Basic filters: All / Matches / No Match
- Advanced filter panel (collapsible)

**Advanced Filters:**
- Sequence length range (min/max)
- COG functional category dropdown (23 categories)
- EC enzyme class dropdown (7 classes)
- Checkboxes: "Has gene name", "Has function description"
- Clear all filters button

**Client-Side Filtering:**
- All sequences loaded once (up to 10,000)
- Instant filtering without server requests
- 80ms debounce prevents UI flickering
- Smooth transitions and hover effects

**Sequence Table:**
- Paginated results (20 per page)
- Clickable database links (UniParc, UniRef100, NCBI)
- Sticky header for scrolling
- Row hover highlighting

#### Functional Analysis Tab

**Annotation Rate:**
- Visual progress ring showing percentage annotated

**Charts (Horizontal Bar Charts):**
- **Top Genes**: Sequential green color palette (darker = lower rank)
- **Top Products**: Sequential green color palette
- **COG Categories**: Categorical color palette (distinct colors)
- **EC Classes**: Categorical color palette
- **GO Terms**: Molecular function terms


### JobListView (`/jobs`)

Job history page with:
- Paginated job list
- Status indicators
- Quick actions (view, delete)

## API Client

The API client (`src/api/jobs.ts`) provides:

### Types

```typescript
type JobStatus = 'pending' | 'processing' | 'completed' | 'failed';
type SequenceFilter = 'all' | 'hash_match' | 'alignment' | 'none';
type DownloadFormat = 'tsv' | 'json' | 'fasta' | 'gff3';

interface AdvancedFilterOptions {
  filter?: SequenceFilter;
  search?: string;
  minLength?: number;
  maxLength?: number;
  cog?: string;
  ecClass?: string;
  hasGene?: boolean;
  hasProduct?: boolean;
}

interface FunctionalStats {
  total_sequences: number;
  annotated_sequences: number;
  top_genes: CountItem[];
  top_products: CountItem[];
  cog_categories: CogCategory[];
  ec_classes: CountItem[];
  go_terms: GoTerms;
}
```

### Functions

```typescript
// Get job with pagination and filtering
getJob(jobId, page?, perPage?, filter?, advancedFilters?): Promise<PaginatedJobResponse>

// Get functional statistics
getJobStats(jobId): Promise<FunctionalStats>

// Create, list, delete jobs
createJobWithFile(file, jobName?): Promise<JobCreateResponse>
listJobs(page?, perPage?): Promise<PaginatedJobsResponse>
deleteJob(jobId): Promise<void>

// Download results
downloadJobResults(jobId, format): Promise<void>
```

## Routing

| Route      | View          | Description                        |
|------------|---------------|------------------------------------|
| `/`        | HomeView      | Landing page                       |
| `/submit`  | SubmitJobView | Job submission                     |
| `/job/:id` | JobDetailView | Job results (tabs, search, charts) |
| `/jobs`    | JobListView   | Job history                        |
| `/docs`    | -             | Redirect to Swagger UI             |

## Styling

### CSS Variables

```css
:root {
  --color-primary: #00bd7e;
  --color-background: #ffffff;
  --color-text: #1a1a1a;
}

@media (prefers-color-scheme: dark) {
  :root {
    --color-background: #1a1a1a;
    --color-text: #ffffff;
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

## Performance Features

- **Client-side filtering**: No server requests during search/filter
- **Debounced search**: 80ms delay prevents excessive re-renders
- **Computed pagination**: Instant page navigation
- **Lazy loading**: Non-critical views loaded on demand

## Customization

### Logo

Replace these files:
- `src/assets/logo-light.png`
- `src/assets/logo-dark.png`
- `public/favicon.png`

### Domain

Update `nginx.conf` to replace `ai-db.computational.bio` with your domain.

### Colors

Edit CSS variables in `src/assets/main.css`.

## Scripts

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
