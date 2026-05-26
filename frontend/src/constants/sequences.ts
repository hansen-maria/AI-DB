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

export function getUniParcUrl(id: string) {
  return `https://www.uniprot.org/uniparc/${id}`
}

export function getNcbiUrl(id: string) {
  return `https://www.ncbi.nlm.nih.gov/protein/${id}`
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
