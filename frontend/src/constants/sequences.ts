import type { JobStatus, SequenceFilter } from '../api/jobs.ts'

// ── Filter / Status ──────────────────────────────────────────────────────────

export const filterOptions: { value: SequenceFilter; label: string }[] = [
  { value: 'all',        label: 'All' },
  { value: 'hash_match', label: 'Matched' },
  { value: 'bakta_db',   label: 'Match (Bakta)' },
  { value: 'aidb_db',    label: 'Match (AI-DB)' },
  { value: 'none',       label: 'No Match' },
]

export const statusColors: Record<JobStatus, string> = {
  pending:    '#ff9800',
  processing: '#2196f3',
  completed:  '#4caf50',
  failed:     '#f44336',
}

export const statusLabels: Record<JobStatus, string> = {
  pending:    'Pending',
  processing: 'Processing',
  completed:  'Completed',
  failed:     'Failed',
}

// ── Classification vocabularies ──────────────────────────────────────────────

export const cogCategories = [
  { value: 'A', label: 'A - RNA processing' },
  { value: 'B', label: 'B - Chromatin structure' },
  { value: 'C', label: 'C - Energy production' },
  { value: 'D', label: 'D - Cell cycle' },
  { value: 'E', label: 'E - Amino acid metabolism' },
  { value: 'F', label: 'F - Nucleotide metabolism' },
  { value: 'G', label: 'G - Carbohydrate metabolism' },
  { value: 'H', label: 'H - Coenzyme metabolism' },
  { value: 'I', label: 'I - Lipid metabolism' },
  { value: 'J', label: 'J - Translation' },
  { value: 'K', label: 'K - Transcription' },
  { value: 'L', label: 'L - Replication/repair' },
  { value: 'M', label: 'M - Cell wall/membrane' },
  { value: 'N', label: 'N - Cell motility' },
  { value: 'O', label: 'O - Post-translational mod.' },
  { value: 'P', label: 'P - Inorganic ion transport' },
  { value: 'Q', label: 'Q - Secondary metabolites' },
  { value: 'R', label: 'R - General function' },
  { value: 'S', label: 'S - Unknown function' },
  { value: 'T', label: 'T - Signal transduction' },
  { value: 'U', label: 'U - Trafficking/secretion' },
  { value: 'V', label: 'V - Defense mechanisms' },
  { value: 'X', label: 'X - Mobilome' },
]

export const ecClasses = [
  { value: '1', label: 'EC 1 - Oxidoreductases' },
  { value: '2', label: 'EC 2 - Transferases' },
  { value: '3', label: 'EC 3 - Hydrolases' },
  { value: '4', label: 'EC 4 - Lyases' },
  { value: '5', label: 'EC 5 - Isomerases' },
  { value: '6', label: 'EC 6 - Ligases' },
  { value: '7', label: 'EC 7 - Translocases' },
]

// ── Color palettes ───────────────────────────────────────────────────────────

export const sequentialColors = [
  '#00bd7e', '#00ad73', '#009d68', '#008d5d', '#007d52',
  '#006d47', '#005d3c', '#004d31', '#003d26', '#002d1b',
]

export const categoricalColors = [
  '#00bd7e', '#00a896', '#028090', '#05668d', '#6b5b95',
  '#d64161', '#ff7b25', '#f6ab3c', '#3d5a80', '#7eb77f',
]

export function getSequentialColor(index: number): string {
  return sequentialColors[Math.min(index, sequentialColors.length - 1)]
}

export function getCategoricalColor(index: number): string {
  return categoricalColors[index % categoricalColors.length]
}

// ── DB link helpers ──────────────────────────────────────────────────────────

export function getUniRef100Url(id: string) {
  return `https://www.uniprot.org/uniref/UniRef100_${id}`
}

/**
 * Builds the UniParc entry URL.
 *
 * Correct format: https://www.uniprot.org/uniparc/UPI{clusterId}/entry/{proteinAccession}
 * - The UPI prefix must be prepended to the raw cluster ID (Bakta/AI-DB store it without UPI).
 * - The protein accession is the same accession embedded in the UniRef ID
 *   (UniRef cluster IDs are formatted as "UniRef100_<accession>"), so callers
 *   pass `uniref100_id` as `proteinAccession`.
 * - If no protein accession is available, falls back to the entry's summary page
 *   (still valid, just without the specific member highlighted).
 */
export function getUniParcUrl(uniparcId: string, proteinAccession?: string | null) {
  const upi = uniparcId.toUpperCase().startsWith('UPI') ? uniparcId : `UPI${uniparcId}`
  const base = `https://www.uniprot.org/uniparc/${upi}/entry`
  return proteinAccession ? `${base}/${proteinAccession}` : base
}

export function getNcbiUrl(id: string) {
  return `https://www.ncbi.nlm.nih.gov/protein/${id}`
}

// ── Live existence checks ────────────────────────────────────────────────────
// UniRef clusters and NCBI protein records can be retired/merged after Bakta's
// DB snapshot was built, leaving stored IDs that 404 on the target site. These
// helpers batch-check a set of IDs against the live source and return only the
// ones that still resolve, so the UI can avoid rendering dead links.

const UNIPROT_CHUNK_SIZE = 50
const NCBI_CHUNK_SIZE    = 100

function chunk<T>(arr: T[], size: number): T[][] {
  const out: T[][] = []
  for (let i = 0; i < arr.length; i += size) out.push(arr.slice(i, i + size))
  return out
}

/**
 * Checks which UniRef100 IDs still exist by querying UniProt's REST search API.
 * @param ids Raw cluster IDs as stored (WITHOUT the "UniRef100_" prefix)
 * @returns Set of the subset of `ids` that still resolve
 */
export async function checkUniRefExists(ids: string[]): Promise<Set<string>> {
  const unique = Array.from(new Set(ids.filter(Boolean)))
  const found = new Set<string>()
  if (unique.length === 0) return found

  for (const batch of chunk(unique, UNIPROT_CHUNK_SIZE)) {
    try {
      const query = batch.map(id => `id:UniRef100_${id}`).join(' OR ')
      const url = `https://rest.uniprot.org/uniref/search?query=${encodeURIComponent(query)}&fields=id&format=json&size=${batch.length}`
      const res = await fetch(url, { headers: { Accept: 'application/json' } })
      if (!res.ok) continue // fail-open: leave this batch's links visible on error
      const data = await res.json()
      for (const entry of data.results ?? []) {
        const rawId = String(entry.id ?? '').replace(/^UniRef100_/, '')
        if (rawId) found.add(rawId)
      }
    } catch {
      // Network/CORS error – fail-open, don't hide links we couldn't verify
      for (const id of batch) found.add(id)
    }
  }
  return found
}

/**
 * Checks which NCBI protein accessions still resolve via NCBI E-utilities (esummary).
 * Matches the returned accession.version back to the requested ID so merged/replaced
 * records (which resolve under a *different* accession) are correctly treated as gone.
 *
 * Note: eutils.ncbi.nlm.nih.gov does not reliably send CORS headers for direct
 * browser requests. If these calls are blocked by CORS in production, this
 * fails open (see catch block) and all NCBI links remain visible as before –
 * a backend proxy endpoint would be needed to enforce the check in that case.
 *
 * @param ids NCBI protein accessions as stored (e.g. "WP_012345678.1")
 * @returns Set of the subset of `ids` that still resolve under the same accession
 */
export async function checkNcbiProteinExists(ids: string[]): Promise<Set<string>> {
  const unique = Array.from(new Set(ids.filter(Boolean)))
  const found = new Set<string>()
  if (unique.length === 0) return found

  for (const batch of chunk(unique, NCBI_CHUNK_SIZE)) {
    try {
      const url = `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi` +
          `?db=protein&retmode=json&id=${batch.map(encodeURIComponent).join(',')}`
      const res = await fetch(url, { headers: { Accept: 'application/json' } })
      if (!res.ok) continue // fail-open
      const data = await res.json()
      const uids: string[] = data?.result?.uids ?? []
      const requested = new Set(batch)
      for (const uid of uids) {
        const doc = data.result[uid]
        const accession = doc?.accessionversion || doc?.caption
        if (accession && requested.has(accession)) found.add(accession)
      }
    } catch {
      // Network/CORS error – fail-open, don't hide links we couldn't verify
      for (const id of batch) found.add(id)
    }
  }
  return found
}

export function hasAnnotationLinks(seq: {
  uniparc_id?: string | null
  ncbi_nrp_id?: string | null
  uniref100_id?: string | null
}): boolean {
  return !!(seq.uniparc_id || seq.ncbi_nrp_id || seq.uniref100_id)
}

export function formatDate(dateStr: string) {
  return new Date(dateStr).toLocaleString()
}
