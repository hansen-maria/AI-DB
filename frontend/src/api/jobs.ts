/**
 * AI-DB API Client
 * TypeScript client for the REST API
 */

const API_BASE = '/api';

// ============================================================================
// Types
// ============================================================================

export type JobStatus = 'pending' | 'processing' | 'completed' | 'failed';

export interface SequenceInfo {
    id: string;
    md5_hash: string;
    length: number;
    annotation: string | null;
    annotation_source: string | null;
    uniparc_id: string | null;
    ncbi_nrp_id: string | null;
    uniref100_id: string | null;
}

export interface PaginationInfo {
    page: number;
    per_page: number;
    total_items: number;
    total_pages: number;
    has_next: boolean;
    has_prev: boolean;
}

export interface JobSummary {
    job_id: string;
    status: JobStatus;
    created_at: string;
    updated_at: string;
    filename: string | null;
    sequence_count: number;
    processed_count: number;
    hash_matches: number;
    error_message: string | null;
}

export interface PaginatedJobsResponse {
    jobs: JobSummary[];
    pagination: PaginationInfo;
}

export type SequenceFilter = 'all' | 'hash_match' | 'alignment' | 'none';

export interface PaginatedJobResponse {
    job_id: string;
    status: JobStatus;
    created_at: string;
    updated_at: string;
    filename: string | null;
    sequence_count: number;
    processed_count: number;
    hash_matches: number;
    alignment_matches: number;
    error_message: string | null;
    sequences: SequenceInfo[];
    pagination: PaginationInfo;
    filter: SequenceFilter;
    filtered_count: number;
}

// Legacy type for backward compatibility
export interface JobResponse {
    job_id: string;
    status: JobStatus;
    created_at: string;
    updated_at: string;
    filename: string | null;
    sequence_count: number;
    processed_count: number;
    hash_matches: number;
    alignment_matches: number;
    sequences: SequenceInfo[] | null;
    error_message: string | null;
}

export interface JobCreateResponse {
    job_id: string;
    status: JobStatus;
    message: string;
    sequence_count: number;
}

export interface HealthCheckResponse {
    status: string;
    service: string;
    bakta_db: {
        status: 'connected' | 'error' | 'not_found' | 'not_configured';
        path: string | null;
    };
}

export interface DbInfoResponse {
    available: boolean;
    path: string | null;
    ups_entries?: number | null;
    version?: string | null;
    error?: string;
}

export interface ApiError {
    detail: string;
}

// ============================================================================
// API Functions
// ============================================================================

/**
 * Create a new annotation job with a FASTA file
 */
export async function createJobWithFile(
    file: File,
    jobName?: string
): Promise<JobCreateResponse> {
    const formData = new FormData();
    formData.append('file', file);
    if (jobName) {
        formData.append('job_name', jobName);
    }

    const response = await fetch(`${API_BASE}/job/`, {
        method: 'POST',
        body: formData,
        credentials: 'include', // Include cookies
    });

    // Check content type to avoid parsing HTML as JSON
    const contentType = response.headers.get('content-type');
    if (!contentType || !contentType.includes('application/json')) {
        throw new Error('API not available. Please try again later.');
    }

    if (!response.ok) {
        try {
            const error: ApiError = await response.json();
            throw new Error(error.detail || 'Failed to create job');
        } catch {
            throw new Error('Failed to create job');
        }
    }

    return response.json();
}

/**
 * Create a new annotation job with direct FASTA content
 */
export async function createJobWithContent(
    fastaContent: string,
    jobName?: string
): Promise<JobCreateResponse> {
    const formData = new FormData();
    formData.append('fasta_content', fastaContent);
    if (jobName) {
        formData.append('job_name', jobName);
    }

    const response = await fetch(`${API_BASE}/job/`, {
        method: 'POST',
        body: formData,
        credentials: 'include', // Include cookies
    });

    // Check content type to avoid parsing HTML as JSON
    const contentType = response.headers.get('content-type');
    if (!contentType || !contentType.includes('application/json')) {
        throw new Error('API not available. Please try again later.');
    }

    if (!response.ok) {
        try {
            const error: ApiError = await response.json();
            throw new Error(error.detail || 'Failed to create job');
        } catch {
            throw new Error('Failed to create job');
        }
    }

    return response.json();
}

/**
 * Get job status and results with pagination and filtering
 */
export async function getJob(
    jobId: string,
    page = 1,
    perPage = 20,
    filter: SequenceFilter = 'all'
): Promise<PaginatedJobResponse> {
    const params = new URLSearchParams({
        page: page.toString(),
        per_page: perPage.toString(),
        filter: filter,
    });

    const response = await fetch(`${API_BASE}/job/${jobId}?${params}`, {
        credentials: 'include', // Include cookies
    });

    // Check content type to avoid parsing HTML as JSON
    const contentType = response.headers.get('content-type');
    if (!contentType || !contentType.includes('application/json')) {
        throw new Error('API not available');
    }

    if (!response.ok) {
        if (response.status === 404) {
            throw new Error(`Job with ID '${jobId}' not found`);
        }
        try {
            const error: ApiError = await response.json();
            throw new Error(error.detail || 'Failed to get job');
        } catch {
            throw new Error('Failed to get job');
        }
    }

    return response.json();
}

/**
 * List all jobs with pagination
 */
export async function listJobs(
    page = 1,
    perPage = 20
): Promise<PaginatedJobsResponse> {
    const params = new URLSearchParams({
        page: page.toString(),
        per_page: perPage.toString(),
    });

    const response = await fetch(`${API_BASE}/jobs/?${params}`, {
        credentials: 'include', // Include cookies
    });

    // Check content type to avoid parsing HTML as JSON
    const contentType = response.headers.get('content-type');
    if (!contentType || !contentType.includes('application/json')) {
        // API not available or returned HTML (e.g., 404 page)
        console.warn('API returned non-JSON response, returning empty list');
        return {
            jobs: [],
            pagination: {
                page: 1,
                per_page: perPage,
                total_items: 0,
                total_pages: 0,
                has_next: false,
                has_prev: false,
            },
        };
    }

    if (!response.ok) {
        try {
            const error: ApiError = await response.json();
            throw new Error(error.detail || 'Failed to list jobs');
        } catch {
            throw new Error('Failed to list jobs');
        }
    }

    return response.json();
}

/**
 * Delete a job
 */
export async function deleteJob(jobId: string): Promise<void> {
    const response = await fetch(`${API_BASE}/job/${jobId}`, {
        method: 'DELETE',
        credentials: 'include', // Include cookies
    });

    if (!response.ok) {
        if (response.status === 404) {
            throw new Error(`Job with ID '${jobId}' not found`);
        }
        const error: ApiError = await response.json();
        throw new Error(error.detail || 'Failed to delete job');
    }
}

/**
 * Poll job status until completion (only fetches first page of sequences)
 */
export async function pollJobUntilComplete(
    jobId: string,
    onUpdate?: (job: PaginatedJobResponse) => void,
    intervalMs = 1000,
    maxAttempts = 300
): Promise<PaginatedJobResponse> {
    let attempts = 0;

    while (attempts < maxAttempts) {
        const job = await getJob(jobId, 1, 20);

        if (onUpdate) {
            onUpdate(job);
        }

        if (job.status === 'completed' || job.status === 'failed') {
            return job;
        }

        await new Promise(resolve => setTimeout(resolve, intervalMs));
        attempts++;
    }

    throw new Error('Job polling timed out');
}

// ============================================================================
// Download Functions
// ============================================================================

export type DownloadFormat = 'tsv' | 'json' | 'fasta' | 'gff3';

export interface DownloadOption {
    format: DownloadFormat;
    label: string;
    description: string;
}

export const downloadOptions: DownloadOption[] = [
    {
        format: 'tsv',
        label: 'TSV',
        description: 'Tab-separated values for spreadsheets',
    },
    {
        format: 'json',
        label: 'JSON',
        description: 'Full data with metadata for programming',
    },
    {
        format: 'fasta',
        label: 'FASTA',
        description: 'Annotated sequences for bioinformatics',
    },
    {
        format: 'gff3',
        label: 'GFF3',
        description: 'Genome feature format for browsers',
    },
];

/**
 * Download job results in specified format
 */
export async function downloadJobResults(
    jobId: string,
    format: DownloadFormat
): Promise<void> {
    const url = `${API_BASE}/job/${jobId}/download/${format}`;

    const response = await fetch(url, {
        credentials: 'include',
    });

    if (!response.ok) {
        if (response.status === 404) {
            throw new Error(`Job with ID '${jobId}' not found`);
        }
        if (response.status === 403) {
            throw new Error('Not authorized to download this job');
        }
        if (response.status === 400) {
            const error = await response.json();
            throw new Error(error.detail || 'Invalid request');
        }
        throw new Error('Failed to download results');
    }

    // Get filename from Content-Disposition header
    const contentDisposition = response.headers.get('Content-Disposition');
    let filename = `results.${format}`;
    if (contentDisposition) {
        const match = contentDisposition.match(/filename="(.+)"/);
        if (match) {
            filename = match[1];
        }
    }

    // Download the file
    const blob = await response.blob();
    const downloadUrl = window.URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = downloadUrl;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    window.URL.revokeObjectURL(downloadUrl);
}

/**
 * Health check endpoint - includes database status
 */
export async function checkHealth(): Promise<HealthCheckResponse> {
    const response = await fetch(`${API_BASE}/health`, {
        credentials: 'include',
    });

    // Check content type
    const contentType = response.headers.get('content-type');
    if (!contentType || !contentType.includes('application/json')) {
        throw new Error('API not available');
    }

    if (!response.ok) {
        try {
            const error: ApiError = await response.json();
            throw new Error(error.detail || 'Health check failed');
        } catch {
            throw new Error(`Health check failed: ${response.statusText}`);
        }
    }

    return response.json();
}

/**
 * Database info endpoint - provide details about the Bakta database
 */
export async function dbInfo(): Promise<DbInfoResponse> {
    const response = await fetch(`${API_BASE}/db/info`, {
        credentials: 'include',
    });

    // Check content type
    const contentType = response.headers.get('content-type');
    if (!contentType || !contentType.includes('application/json')) {
        throw new Error('API not available');
    }

    if (!response.ok) {
        try {
            const error: ApiError = await response.json();
            throw new Error(error.detail || 'Failed to fetch DB info');
        } catch {
            throw new Error(`Failed to fetch DB info: ${response.statusText}`);
        }
    }

    return response.json();
}