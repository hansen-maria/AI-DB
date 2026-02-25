/**
 * Psos API Client
 * Integration with Protein Sequence Observation Service
 */

const PSOS_API = 'https://psos.computational.bio/api/v1';
const PSOS_WEB = 'https://psos.computational.bio';

export type PsosProfile = 'bacteria-gram+' | 'bacteria-gram-' | 'eukaryote' | 'eukaryote-plant';

export interface PsosProfileOption {
  value: PsosProfile;
  label: string;
  description: string;
}

export const psosProfiles: PsosProfileOption[] = [
  { value: 'bacteria-gram+', label: 'Bacteria (Gram+)', description: 'Gram-positive bacteria' },
  { value: 'bacteria-gram-', label: 'Bacteria (Gram-)', description: 'Gram-negative bacteria' },
  { value: 'eukaryote', label: 'Eukaryote', description: 'Eukaryotic organisms' },
  { value: 'eukaryote-plant', label: 'Eukaryote (Plant)', description: 'Plant-specific analysis' },
];

export interface PsosJob {
  id: string;
  state: {
    label: string | null;
    value: string;  // 'Succeeded', 'Failed', 'Running', etc.
  };
  data?: {
    files?: PsosFile[];
  };
}

export interface PsosFile {
  name: string;
  type: 'input' | 'log' | 'result';
  path: string;
}

export interface PsosAnnotation {
  sequenceId: string;
  psosJobId: string;
  // Best homology hit from ghostx
  bestHit?: {
    dbxref: string;
    evalue: number;
    percentIdentity: number;
  };
  // Protein name from dbxrefs
  proteinName?: string;
  // Signal peptide prediction
  hasSignalPeptide?: boolean;
  // Transmembrane helices count
  transmembraneCount?: number;
  // Has homology hits
  hasHomology?: boolean;
}

/**
 * Parse Psos result JSON into annotation
 */
export function parsePsosResult(data: any): Partial<PsosAnnotation> {
  const result: Partial<PsosAnnotation> = {};

  if (!data.computations || !Array.isArray(data.computations)) {
    return result;
  }

  for (const comp of data.computations) {
    const toolName = comp.tool?.name?.toLowerCase() || '';

    // Homology search (ghostx)
    if (toolName === 'ghostx' && comp.results?.length > 0) {
      // Find best hit (lowest e-value)
      const bestHit = comp.results.reduce((best: any, curr: any) => {
        if (!best || (curr.target?.evalue < best.target?.evalue)) {
          return curr;
        }
        return best;
      }, null);

      if (bestHit?.target) {
        result.hasHomology = true;
        result.bestHit = {
          dbxref: bestHit.target.dbxref || '',
          evalue: bestHit.target.evalue,
          percentIdentity: bestHit.target.percent_identity
        };
      }
    }

    // Signal peptide (SignalP) - check signalpeptide boolean field
    if (toolName === 'signalp' && comp.results?.length > 0) {
      const spResult = comp.results[0];
      result.hasSignalPeptide = spResult.signalpeptide === true;
    }

    // Transmembrane (TMHMM) - check PredHel count
    if (toolName === 'tmhmm' && comp.results?.length > 0) {
      const tmResult = comp.results[0];
      if (tmResult.PredHel && tmResult.PredHel > 0) {
        result.transmembraneCount = tmResult.PredHel;
      }
    }
  }

  // Try to get protein name from dbxrefs
  if (data.dbxrefs && Array.isArray(data.dbxrefs)) {
    for (const ref of data.dbxrefs) {
      if (ref.recommended_name?.full) {
        result.proteinName = ref.recommended_name.full;
        break;
      }
    }
  }

  return result;
}

/**
 * Submit a sequence to Psos for analysis
 * POST /api/v1/job/submit
 */
export async function submitToPsos(
    sequenceId: string,
    sequence: string,
    profile: PsosProfile
): Promise<PsosJob> {
  const fastaContent = `>${sequenceId}\n${sequence}`;

  const request = {
    configuration: { profile },
    sequence: fastaContent
  };

  const response = await fetch(`${PSOS_API}/job/submit`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Accept': 'application/json',
    },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Psos submission failed (${response.status}): ${errorText}`);
  }

  return response.json();
}

/**
 * Get job status and files
 * GET /api/v1/job/{jobid}
 */
export async function getPsosJob(jobId: string): Promise<PsosJob> {
  const response = await fetch(`${PSOS_API}/job/${jobId}`, {
    headers: {
      'Accept': 'application/json',
    },
  });

  if (!response.ok) {
    throw new Error(`Failed to get Psos job: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Get file content from a Psos job
 * GET /api/v1/job/{jobid}/file/{filename}
 */
export async function getPsosFile(jobId: string, fileName: string): Promise<string> {
  const response = await fetch(`${PSOS_API}/job/${jobId}/file/${fileName}`, {
    headers: {
      'Accept': 'application/json, text/plain, */*',
    },
  });

  if (!response.ok) {
    throw new Error(`Failed to get Psos file: ${response.statusText}`);
  }

  return response.text();
}

/**
 * Get the URL to view a job on Psos web interface
 */
export function getPsosJobUrl(jobId: string): string {
  return `${PSOS_WEB}/psov2/${jobId}`;
}

/**
 * Poll a Psos job until completion
 */
export async function pollPsosJob(
    jobId: string,
    onProgress?: (job: PsosJob) => void,
    maxAttempts = 120,
    intervalMs = 2000
): Promise<PsosJob> {
  let attempts = 0;

  while (attempts < maxAttempts) {
    const job = await getPsosJob(jobId);

    if (onProgress) {
      onProgress(job);
    }

    // Check for terminal states (Succeeded, Failed, Error)
    const state = job.state?.value?.toLowerCase() || '';
    if (state === 'succeeded' || state === 'failed' || state === 'error') {
      return job;
    }

    await new Promise(resolve => setTimeout(resolve, intervalMs));
    attempts++;
  }

  throw new Error('Psos job timed out');
}

// ============================================================================
// Fallback: Link-based approach (if CORS blocks API calls)
// ============================================================================

/**
 * Generate a FASTA-formatted string for multiple sequences
 */
export function generateFasta(sequences: Array<{ id: string; sequence: string }>): string {
  return sequences
      .map(seq => `>${seq.id}\n${seq.sequence}`)
      .join('\n');
}

/**
 * Copy sequences to clipboard and open Psos
 */
export async function openInPsos(
    sequences: Array<{ id: string; sequence: string }>
): Promise<void> {
  const fasta = generateFasta(sequences);

  // Copy to clipboard
  await navigator.clipboard.writeText(fasta);

  // Open Psos in new tab
  window.open(PSOS_WEB, '_blank');
}

/**
 * Download sequences as FASTA file for manual upload to Psos
 */
export function downloadForPsos(
    sequences: Array<{ id: string; sequence: string }>,
    filename = 'unmatched_sequences.fasta'
): void {
  const fasta = generateFasta(sequences);
  const blob = new Blob([fasta], { type: 'text/plain' });
  const url = URL.createObjectURL(blob);

  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();

  URL.revokeObjectURL(url);
}
