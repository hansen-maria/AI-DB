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
}

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
 * Get job status and results with pagination
 */
export async function getJob(
    jobId: string,
    page = 1,
    perPage = 20
): Promise<PaginatedJobResponse> {
    const params = new URLSearchParams({
        page: page.toString(),
        per_page: perPage.toString(),
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
 * Poll job status until completion
 */
export async function pollJobUntilComplete(
    jobId: string,
    onUpdate?: (job: JobResponse) => void,
    intervalMs = 1000,
    maxAttempts = 300
): Promise<JobResponse> {
    let attempts = 0;

    while (attempts < maxAttempts) {
        const job = await getJob(jobId);

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