/**
 * Owner token of this browser session
 *
 * The owner identity lives in an HttpOnly cookie that JavaScript cannot read.
 * These calls ask the backend for it, so the user can copy it into the AI-DB
 * CLI or a script and manage the same jobs there. The backend only answers
 * same-origin requests.
 */

const API_BASE = '/api';

interface AuthTokenResponse {
    token: string;
}

/** Thrown when the stored identity cannot be used as an API token (HTTP 409). */
export class TokenNotExportableError extends Error {}

async function errorDetail(response: Response, fallback: string): Promise<string> {
    try {
        const body = await response.json();
        return body.detail || fallback;
    } catch {
        return fallback;
    }
}

async function readToken(response: Response, fallback: string): Promise<string> {
    if (response.status === 409) {
        throw new TokenNotExportableError(await errorDetail(response, fallback));
    }
    if (!response.ok) {
        throw new Error(await errorDetail(response, fallback));
    }
    const body: AuthTokenResponse = await response.json();
    return body.token;
}

/**
 * Returns the access token of this browser, or `null` if none exists yet
 * (it is created with the first job).
 */
export async function getAccessToken(): Promise<string | null> {
    const response = await fetch(`${API_BASE}/auth/token`, {
        credentials: 'same-origin',
        cache: 'no-store',
    });
    if (response.status === 404) return null;
    return readToken(response, 'Failed to load access token');
}

/**
 * Returns the access token of this browser, creating one if none exists yet.
 * An existing token is never replaced.
 */
export async function createAccessToken(): Promise<string> {
    const response = await fetch(`${API_BASE}/auth/token`, {
        method: 'POST',
        credentials: 'same-origin',
        cache: 'no-store',
    });
    return readToken(response, 'Failed to create access token');
}
