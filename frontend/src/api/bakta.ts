/**
 * Bakta Web API Client – V1 (nucleotide) + V2 (protein)
 * Spec: https://api.bakta.computational.bio/swagger-ui/
 *
 * Routing:
 *   Nucleotide sequences → V1 API  (bakta workflow, full genome annotation)
 *   Protein sequences   → V2 API  (bakta_proteins workflow)
 *
 * Auto-detection via detectSequenceType().
 */

const BAKTA_V1 = 'https://api.staging.bakta.computational.bio/api/v1'
const BAKTA_V2 = 'https://api.staging.bakta.computational.bio/api/v2'

// ═══════════════════════════════════════════════════════════════════════════
// Shared types
// ═══════════════════════════════════════════════════════════════════════════

export type SequenceType = 'nucleotide' | 'protein'
export type DermType = 'unknown' | 'monoderm' | 'diderm'
export type RepliconTableType = 'CSV' | 'TSV'

/** Normalised job status (uppercase) used throughout the workflow functions. */
export type BaktaJobStatusEnum = 'INIT' | 'RUNNING' | 'SUCCESSFUL' | 'ERROR'

// ═══════════════════════════════════════════════════════════════════════════
// V1 types  (camelCase keys, uppercase enums)
// ═══════════════════════════════════════════════════════════════════════════

/** Schema: Job (V1 job handle) */
export interface BaktaJobRef {
  jobID: string     // uuid
  secret: string
}

/** Schema: JobConfig – V1 required fields + optional nullable fields */
export interface BaktaJobConfig {
  // Required
  translationTable: number    // int32, >= 0; default 11
  completeGenome: boolean
  keepContigHeaders: boolean
  minContigLength: number     // int64, >= 0
  compliant: boolean
  // Optional / nullable
  dermType?: DermType | null
  genus?: string | null
  hasReplicons?: boolean
  locus?: string | null
  locusTag?: string | null
  plasmid?: string | null
  prodigalTrainingFile?: string | null
  species?: string | null
  strain?: string | null
}

export interface BaktaInitRequest {
  name: string
  repliconTableType: RepliconTableType
}

export interface BaktaInitResponse {
  job: BaktaJobRef
  uploadLinkFasta: string
  uploadLinkProdigal: string
  uploadLinkReplicons: string
}

export interface BaktaV1JobStatus {
  jobID: string
  jobStatus: BaktaJobStatusEnum   // V1 already uppercase
  started: string
  updated: string
  name: string
}

export interface BaktaListRequest {
  jobs: BaktaJobRef[]
}

export interface BaktaListResponse {
  jobs: BaktaV1JobStatus[]
  failedJobs: Array<{ jobID: string; jobStatus: string }>
}

export interface BaktaStartRequest {
  job: BaktaJobRef
  config: BaktaJobConfig
}

/** All 14 result files are required in V1 responses. */
export interface BaktaResultFiles {
  EMBL: string
  FAA: string
  FAAHypothetical: string
  FFN: string
  FNA: string
  GBFF: string
  GFF3: string
  JSON: string
  TSV: string
  TSVHypothetical: string
  TSVInference: string
  TXTLogs: string
  PNGCircularPlot: string
  SVGCircularPlot: string
}

export interface BaktaResultResponse {
  ResultFiles: BaktaResultFiles
  jobID: string
  name: string
  started: string
  updated: string
}

// ═══════════════════════════════════════════════════════════════════════════
// V2 types  (snake_case keys, lowercase enums)
// ═══════════════════════════════════════════════════════════════════════════

/** V2 job handle – uses snake_case job_id */
export interface JobReference {
  job_id: string    // uuid
  secret: string
}

export interface V2InitRequest {
  name: string
  workflow_kind: 'bakta_proteins'   // the only V2 kind used here
}

export interface V2UploadLink {
  upload_kind: string
  required: boolean
  url: string
}

export interface V2InitResponse {
  job: JobReference
  workflow_kind: string
  uploads: V2UploadLink[]
}

/** V2 job status uses lowercase values */
export interface V2JobStatus {
  job_id: string
  status: string            // 'init' | 'running' | 'successful' | 'error'
  workflow_kind: string
  result_kind: string
  started: string
  updated: string
  name: string
}

export interface V2ListRequest {
  jobs: JobReference[]
}

export interface V2ListResponse {
  jobs: V2JobStatus[]
  failed_jobs: Array<{ job_id: string; status: string }>
}

/** Schema: BaktaProteinsResultFiles – json required, rest optional */
export interface BaktaProteinsResultFiles {
  json: string
  faa?: string | null
  tsv?: string | null
  hypotheticals_tsv?: string | null
}

export interface V2ResultResponse {
  job_id: string
  workflow_kind: string
  name: string
  started: string
  updated: string
  result: {
    result_kind: 'bakta_proteins'
    files: BaktaProteinsResultFiles
  }
}

export interface V2StageLog {
  stage: string
  status: string    // 'pending' | 'running' | 'succeeded' | 'failed' | 'error' | 'unknown'
  content: string
}

export interface V2LogsResponse {
  workflow_kind: string
  stages: V2StageLog[]
}

// ═══════════════════════════════════════════════════════════════════════════
// Bakta JSON result (annotation output, not part of the REST API spec)
// ═══════════════════════════════════════════════════════════════════════════

export interface BaktaFeature {
  type: string
  contig: string
  start: number
  stop: number
  strand: string
  gene?: string
  product?: string
  locus_tag?: string
  db_xrefs?: string[]
  [key: string]: unknown
}

export interface BaktaJsonResult {
  genome?: { genus?: string; species?: string; strain?: string }
  stats?: {
    no_sequences?: number
    size?: number
    gc?: number
    n_ratio?: number
    coding_ratio?: number
    no_cdss?: number
    no_hypotheticals?: number
    no_pseudogenes?: number
    no_trnas?: number
    no_rrnas?: number
    no_ncrnas?: number
    no_gaps?: number
    no_oriCs?: number
    no_oriVs?: number
    no_oriTs?: number
    no_sORFs?: number
    no_crispr_arrays?: number
  }
  features?: BaktaFeature[]
}

// ═══════════════════════════════════════════════════════════════════════════
// High-level summary (returned by both workflow functions)
// ═══════════════════════════════════════════════════════════════════════════

export interface BaktaAnnotationSummary {
  jobID: string
  secret: string
  jobStatus: BaktaJobStatusEnum
  sequenceType: SequenceType
  /** Present when sequenceType === 'nucleotide' (V1 workflow) */
  resultFilesNucleotide?: BaktaResultFiles
  /** Present when sequenceType === 'protein' (V2 workflow) */
  resultFilesProtein?: BaktaProteinsResultFiles
  stats?: BaktaJsonResult['stats']
  featureCount?: number
  features?: BaktaFeature[]     // first 200 for display
  webViewerUrl: string
}

// ═══════════════════════════════════════════════════════════════════════════
// User-facing config options (V1 only; V2 protein has no configurable options)
// ═══════════════════════════════════════════════════════════════════════════

export interface BaktaJobOptions {
  genus?: string
  species?: string
  strain?: string
  completeGenome?: boolean
  compliant?: boolean
  dermType?: DermType | null
  keepContigHeaders?: boolean
  locus?: string
  locusTag?: string
  minContigLength?: number
  plasmid?: string
  translationTable?: number
}

const DEFAULT_V1_CONFIG: BaktaJobConfig = {
  translationTable: 11,
  completeGenome: false,
  keepContigHeaders: true,
  minContigLength: 0,
  compliant: false,
}

/** Merge user options with safe defaults to produce a fully valid V1 JobConfig. */
export function buildJobConfig(options: BaktaJobOptions = {}): BaktaJobConfig {
  return {
    ...DEFAULT_V1_CONFIG,
    completeGenome: options.completeGenome ?? DEFAULT_V1_CONFIG.completeGenome,
    compliant: options.compliant ?? DEFAULT_V1_CONFIG.compliant,
    keepContigHeaders: options.keepContigHeaders ?? DEFAULT_V1_CONFIG.keepContigHeaders,
    minContigLength: options.minContigLength ?? DEFAULT_V1_CONFIG.minContigLength,
    translationTable: options.translationTable ?? DEFAULT_V1_CONFIG.translationTable,
    ...(options.genus !== undefined     && { genus: options.genus }),
    ...(options.species !== undefined   && { species: options.species }),
    ...(options.strain !== undefined    && { strain: options.strain }),
    ...(options.dermType !== undefined  && { dermType: options.dermType }),
    ...(options.locus !== undefined     && { locus: options.locus }),
    ...(options.locusTag !== undefined  && { locusTag: options.locusTag }),
    ...(options.plasmid !== undefined   && { plasmid: options.plasmid }),
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Sequence type detection
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Detects whether sequences are nucleotide or protein by checking for amino
 * acid characters that cannot appear in IUPAC nucleotide sequences.
 * Definitively protein-only letters: E, F, I, L, P, Q
 */
export function detectSequenceType(
  sequences: Array<{ sequence: string }>,
): SequenceType {
  const PROTEIN_ONLY = /[EFILPQefilpq]/
  // Sample up to the first 10 sequences, 200 chars each
  const sample = sequences
    .slice(0, 10)
    .map(s => s.sequence.slice(0, 200))
    .join('')
  return PROTEIN_ONLY.test(sample) ? 'protein' : 'nucleotide'
}

// ═══════════════════════════════════════════════════════════════════════════
// Shared upload helper
// ═══════════════════════════════════════════════════════════════════════════

/** PUT FASTA content to a presigned S3 URL (used by both V1 and V2). */
export async function uploadFastaToS3(uploadUrl: string, fastaContent: string): Promise<void> {
  const resp = await fetch(uploadUrl, {
    method: 'PUT',
    body: new Blob([fastaContent], { type: 'text/plain' }),
  })
  if (!resp.ok) {
    throw new Error(`S3 upload failed (${resp.status}): ${resp.statusText}`)
  }
}

/** Fetch and parse the Bakta JSON result file from S3. Returns null on CORS/error. */
export async function fetchBaktaJsonResult(jsonUrl: string): Promise<BaktaJsonResult | null> {
  try {
    const resp = await fetch(jsonUrl)
    if (!resp.ok) return null
    return resp.json() as Promise<BaktaJsonResult>
  } catch {
    return null
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// V1 API calls  (nucleotide / genome annotation)
// ═══════════════════════════════════════════════════════════════════════════

/** POST /api/v1/job/init */
export async function initBaktaJob(
  name: string,
  repliconTableType: RepliconTableType = 'TSV',
): Promise<BaktaInitResponse> {
  const resp = await fetch(`${BAKTA_V1}/job/init`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({ name, repliconTableType } satisfies BaktaInitRequest),
  })
  if (!resp.ok) {
    const detail = await resp.text().catch(() => '')
    throw new Error(`Bakta V1 init failed (${resp.status}): ${detail || resp.statusText}`)
  }
  return resp.json() as Promise<BaktaInitResponse>
}

/** POST /api/v1/job/start */
export async function startBaktaJob(
  job: BaktaJobRef,
  config: BaktaJobConfig,
): Promise<void> {
  const resp = await fetch(`${BAKTA_V1}/job/start`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({ job, config } satisfies BaktaStartRequest),
  })
  if (!resp.ok) {
    const detail = await resp.text().catch(() => '')
    throw new Error(`Bakta V1 start failed (${resp.status}): ${detail || resp.statusText}`)
  }
}

/** POST /api/v1/job/list – returns normalised status or null if not found yet. */
export async function listBaktaJob(job: BaktaJobRef): Promise<BaktaV1JobStatus | null> {
  const resp = await fetch(`${BAKTA_V1}/job/list`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({ jobs: [job] } satisfies BaktaListRequest),
  })
  if (!resp.ok) throw new Error(`Bakta V1 list failed (${resp.status}): ${resp.statusText}`)
  const data: BaktaListResponse = await resp.json()
  const failed = data.failedJobs?.find(f => f.jobID === job.jobID)
  if (failed) throw new Error(`Bakta V1 job ${job.jobID}: ${failed.jobStatus}`)
  return data.jobs?.[0] ?? null
}

/** POST /api/v1/job/result */
export async function getBaktaResult(job: BaktaJobRef): Promise<BaktaResultResponse> {
  const resp = await fetch(`${BAKTA_V1}/job/result`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify(job),
  })
  if (!resp.ok) {
    const detail = await resp.text().catch(() => '')
    throw new Error(`Bakta V1 result failed (${resp.status}): ${detail || resp.statusText}`)
  }
  return resp.json() as Promise<BaktaResultResponse>
}

/** GET /api/v1/job/logs?jobID=&secret= */
export async function getBaktaLogs(job: BaktaJobRef): Promise<string> {
  const url = new URL(`${BAKTA_V1}/job/logs`)
  url.searchParams.set('jobID', job.jobID)
  url.searchParams.set('secret', job.secret)
  const resp = await fetch(url.toString(), { headers: { Accept: 'text/plain, */*' } })
  if (!resp.ok) throw new Error(`Bakta V1 logs failed (${resp.status}): ${resp.statusText}`)
  return resp.text()
}

/** DELETE /api/v1/job/delete?jobID=&secret= */
export async function deleteBaktaJob(job: BaktaJobRef): Promise<void> {
  const url = new URL(`${BAKTA_V1}/job/delete`)
  url.searchParams.set('jobID', job.jobID)
  url.searchParams.set('secret', job.secret)
  const resp = await fetch(url.toString(), { method: 'DELETE' })
  if (!resp.ok) throw new Error(`Bakta V1 delete failed (${resp.status}): ${resp.statusText}`)
}

/** GET /api/v1/version */
export async function getBaktaVersion() {
  const resp = await fetch(`${BAKTA_V1}/version`, { headers: { Accept: 'application/json' } })
  if (!resp.ok) throw new Error(`Bakta V1 version check failed: ${resp.statusText}`)
  return resp.json()
}

// ═══════════════════════════════════════════════════════════════════════════
// V2 API calls  (protein annotation)
// ═══════════════════════════════════════════════════════════════════════════

/** POST /api/v2/job/init */
export async function initV2Job(name: string): Promise<V2InitResponse> {
  const resp = await fetch(`${BAKTA_V2}/job/init`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({ name, workflow_kind: 'bakta_proteins' } satisfies V2InitRequest),
  })
  if (!resp.ok) {
    const detail = await resp.text().catch(() => '')
    throw new Error(`Bakta V2 init failed (${resp.status}): ${detail || resp.statusText}`)
  }
  return resp.json() as Promise<V2InitResponse>
}

/** POST /api/v2/job/start – config is EmptyConfig ({}) for bakta_proteins */
export async function startV2Job(job: JobReference): Promise<void> {
  const resp = await fetch(`${BAKTA_V2}/job/start`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({ job, workflow_kind: 'bakta_proteins', config: {} }),
  })
  if (!resp.ok) {
    const detail = await resp.text().catch(() => '')
    throw new Error(`Bakta V2 start failed (${resp.status}): ${detail || resp.statusText}`)
  }
}

/**
 * POST /api/v2/job/list – returns normalised status (uppercase) or null.
 * V2 status values are lowercase; we normalise to uppercase for consistency.
 */
export async function listV2Job(job: JobReference): Promise<V2JobStatus | null> {
  const resp = await fetch(`${BAKTA_V2}/job/list`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({ jobs: [job] } satisfies V2ListRequest),
  })
  if (!resp.ok) throw new Error(`Bakta V2 list failed (${resp.status}): ${resp.statusText}`)
  const data: V2ListResponse = await resp.json()
  const failed = data.failed_jobs?.find(f => f.job_id === job.job_id)
  if (failed) throw new Error(`Bakta V2 job ${job.job_id}: ${failed.status}`)
  return data.jobs?.[0] ?? null
}

/** POST /api/v2/job/result */
export async function getV2Result(job: JobReference): Promise<V2ResultResponse> {
  const resp = await fetch(`${BAKTA_V2}/job/result`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify(job),
  })
  if (!resp.ok) {
    const detail = await resp.text().catch(() => '')
    throw new Error(`Bakta V2 result failed (${resp.status}): ${detail || resp.statusText}`)
  }
  return resp.json() as Promise<V2ResultResponse>
}

/** GET /api/v2/job/logs?job_id=&secret= – returns all stage logs concatenated */
export async function getV2Logs(job: JobReference): Promise<string> {
  const url = new URL(`${BAKTA_V2}/job/logs`)
  url.searchParams.set('job_id', job.job_id)
  url.searchParams.set('secret', job.secret)
  const resp = await fetch(url.toString(), { headers: { Accept: 'application/json' } })
  if (!resp.ok) throw new Error(`Bakta V2 logs failed (${resp.status}): ${resp.statusText}`)
  const data: V2LogsResponse = await resp.json()
  return data.stages
    .map(s => `[${s.stage} – ${s.status}]\n${s.content}`)
    .join('\n\n')
}

/** DELETE /api/v2/job/delete?job_id=&secret= */
export async function deleteV2Job(job: JobReference): Promise<void> {
  const url = new URL(`${BAKTA_V2}/job/delete`)
  url.searchParams.set('job_id', job.job_id)
  url.searchParams.set('secret', job.secret)
  const resp = await fetch(url.toString(), { method: 'DELETE' })
  if (!resp.ok) throw new Error(`Bakta V2 delete failed (${resp.status}): ${resp.statusText}`)
}

// ═══════════════════════════════════════════════════════════════════════════
// Full workflow functions
// ═══════════════════════════════════════════════════════════════════════════

export type BaktaProgressCallback = (stage: string, percent: number) => void

const POLL_INTERVAL_MS = 5_000
const MAX_WAIT_MS = 20 * 60 * 1_000    // 20 min

/**
 * V1 nucleotide workflow:
 * init → S3 upload → start → poll → result → parse JSON
 */
async function runNucleotideAnnotation(
  sequences: Array<{ id: string; sequence: string }>,
  options: BaktaJobOptions,
  onProgress: BaktaProgressCallback,
  signal?: AbortSignal,
  aidbJobId?: string,
): Promise<BaktaAnnotationSummary> {
  const fastaContent = sequences.map(s => `>${s.id}\n${s.sequence}`).join('\n')
  const jobName = `aidb-nucleotide-${Date.now()}`
  const config = buildJobConfig(options)

  console.group('[Bakta V1] Nucleotide annotation workflow')
  console.log('Sequences:', sequences.length, '| Job name:', jobName)
  console.log('Config:', config)

  // 1 – Init
  onProgress('Initializing Bakta job (nucleotide)…', 5)
  if (signal?.aborted) throw new Error('Aborted')
  const init = await initBaktaJob(jobName)
  const jobRef = init.job
  const webViewerUrl = `https://bakta.computational.bio/ui/result?jobID=${jobRef.jobID}&secret=${jobRef.secret}`

  console.log('[Bakta V1] Job initialized | ID:', jobRef.jobID)
  console.log('[Bakta V1] Web viewer:', webViewerUrl)

  // Persist initial state so the user can resume if they navigate away
  if (aidbJobId) await saveBaktaState(aidbJobId, {
    bakta_job_id: jobRef.jobID, bakta_secret: jobRef.secret,
    sequence_type: 'nucleotide', status: 'INIT',
    progress_label: 'Initializing…', progress_percent: 5,
  })

  // 2 – Upload
  onProgress('Uploading nucleotide sequences…', 15)
  if (signal?.aborted) throw new Error('Aborted')
  await uploadFastaToS3(init.uploadLinkFasta, fastaContent)
  console.log('[Bakta V1] FASTA uploaded | Size:', fastaContent.length, 'chars')

  // 3 – Start
  onProgress('Starting genome annotation…', 25)
  if (signal?.aborted) throw new Error('Aborted')
  await startBaktaJob(jobRef, config)
  console.log('[Bakta V1] Job started')

  if (aidbJobId) await saveBaktaState(aidbJobId, {
    bakta_job_id: jobRef.jobID, bakta_secret: jobRef.secret,
    sequence_type: 'nucleotide', status: 'RUNNING',
    progress_label: 'Genome annotation started', progress_percent: 25,
  })

  // 4 – Poll
  let jobStatus: BaktaJobStatusEnum = 'RUNNING'
  let elapsed = 0
  while (elapsed < MAX_WAIT_MS) {
    if (signal?.aborted) throw new Error('Aborted')
    await new Promise(r => setTimeout(r, POLL_INTERVAL_MS))
    elapsed += POLL_INTERVAL_MS
    const entry = await listBaktaJob(jobRef)
    if (!entry) continue
    jobStatus = entry.jobStatus
    console.log(`[Bakta V1] Poll +${Math.round(elapsed / 1000)}s | Status: ${jobStatus} | Updated: ${entry.updated}`)
    const pct = Math.min(85, 25 + (elapsed / MAX_WAIT_MS) * 60)
    onProgress(`Annotating genome… (${jobStatus})`, Math.round(pct))
    if (aidbJobId) await saveBaktaState(aidbJobId, {
      bakta_job_id: jobRef.jobID, bakta_secret: jobRef.secret,
      sequence_type: 'nucleotide', status: jobStatus,
      progress_label: `Annotating genome… (${jobStatus})`, progress_percent: Math.round(pct),
    })
    if (jobStatus === 'SUCCESSFUL' || jobStatus === 'ERROR') break
  }

  if (jobStatus !== 'SUCCESSFUL') {
    onProgress('Fetching error logs…', 87)
    let logs = ''
    try { logs = await getBaktaLogs(jobRef) } catch { /* non-critical */ }
    console.error('[Bakta V1] Job failed | Status:', jobStatus, '\nLogs:\n', logs)
    console.groupEnd()
    if (aidbJobId) await saveBaktaState(aidbJobId, {
      bakta_job_id: jobRef.jobID, bakta_secret: jobRef.secret,
      sequence_type: 'nucleotide', status: jobStatus,
      progress_label: 'Error', progress_percent: 87,
    })
    throw new Error(`Bakta job ended with status: ${jobStatus}${logs ? `\n\nBakta log:\n${logs}` : ''}`)
  }

  // 5 – Result URLs
  onProgress('Retrieving result URLs…', 88)
  const resultResp = await getBaktaResult(jobRef)
  console.log('[Bakta V1] Results ready | Files:', Object.keys(resultResp.ResultFiles).join(', '))

  // 6 – Parse JSON
  let stats: BaktaJsonResult['stats'] | undefined
  let features: BaktaFeature[] | undefined
  onProgress('Parsing annotation results…', 95)
  const json = await fetchBaktaJsonResult(resultResp.ResultFiles.JSON)
  if (json) {
    stats = json.stats
    features = json.features
    console.log('[Bakta V1] JSON parsed | CDSs:', stats?.no_cdss, '| Features:', features?.length, '| GC:', stats?.gc?.toFixed(3))
  } else {
    console.warn('[Bakta V1] JSON not available (likely CORS) – use web viewer for full results')
  }

  onProgress('Done', 100)
  console.log('[Bakta V1] Workflow complete')
  console.groupEnd()

  const summary: BaktaAnnotationSummary = {
    jobID: jobRef.jobID,
    secret: jobRef.secret,
    jobStatus,
    sequenceType: 'nucleotide',
    resultFilesNucleotide: resultResp.ResultFiles,
    stats,
    featureCount: features?.length,
    features: features?.slice(0, 200),
    webViewerUrl,
  }

  if (aidbJobId) await saveBaktaState(aidbJobId, {
    bakta_job_id: jobRef.jobID, bakta_secret: jobRef.secret,
    sequence_type: 'nucleotide', status: 'SUCCESSFUL',
    progress_label: 'Done', progress_percent: 100,
    result_files_json: JSON.stringify(resultResp.ResultFiles),
    result_json: JSON.stringify(summary),
  })

  return summary
}

/**
 * V2 protein workflow:
 * init → find protein_fasta upload slot → S3 upload → start → poll → result → parse JSON
 */
async function runProteinAnnotation(
  sequences: Array<{ id: string; sequence: string }>,
  onProgress: BaktaProgressCallback,
  signal?: AbortSignal,
  aidbJobId?: string,
): Promise<BaktaAnnotationSummary> {
  const fastaContent = sequences.map(s => `>${s.id}\n${s.sequence}`).join('\n')
  const jobName = `aidb-proteins-${Date.now()}`

  console.group('[Bakta V2] Protein annotation workflow')
  console.log('Sequences:', sequences.length, '| Job name:', jobName)

  // 1 – Init
  onProgress('Initializing Bakta protein job…', 5)
  if (signal?.aborted) throw new Error('Aborted')
  const init = await initV2Job(jobName)
  const jobRef = init.job
  const webViewerUrl = `https://bakta.computational.bio/ui/result?jobID=${jobRef.job_id}&secret=${jobRef.secret}`

  console.log('[Bakta V2] Job initialized | ID:', jobRef.job_id)
  console.log('[Bakta V2] Upload slots:', init.uploads.map(u => `${u.upload_kind} (required: ${u.required})`).join(', '))
  console.log('[Bakta V2] Web viewer:', webViewerUrl)

  if (aidbJobId) await saveBaktaState(aidbJobId, {
    bakta_job_id: jobRef.job_id, bakta_secret: jobRef.secret,
    sequence_type: 'protein', status: 'INIT',
    progress_label: 'Initializing…', progress_percent: 5,
  })

  // 2 – Find protein_fasta upload slot
  const proteinUpload = init.uploads.find(u => u.upload_kind === 'protein_fasta')
  if (!proteinUpload) throw new Error('Bakta V2 did not return a protein_fasta upload URL')

  onProgress('Uploading protein sequences…', 15)
  if (signal?.aborted) throw new Error('Aborted')
  await uploadFastaToS3(proteinUpload.url, fastaContent)
  console.log('[Bakta V2] Protein FASTA uploaded | Size:', fastaContent.length, 'chars')

  // 3 – Start
  onProgress('Starting protein annotation…', 25)
  if (signal?.aborted) throw new Error('Aborted')
  await startV2Job(jobRef)
  console.log('[Bakta V2] Job started')

  if (aidbJobId) await saveBaktaState(aidbJobId, {
    bakta_job_id: jobRef.job_id, bakta_secret: jobRef.secret,
    sequence_type: 'protein', status: 'RUNNING',
    progress_label: 'Protein annotation started', progress_percent: 25,
  })

  // 4 – Poll  (V2 status is lowercase – normalise for comparison)
  let rawStatus = 'running'
  let elapsed = 0
  while (elapsed < MAX_WAIT_MS) {
    if (signal?.aborted) throw new Error('Aborted')
    await new Promise(r => setTimeout(r, POLL_INTERVAL_MS))
    elapsed += POLL_INTERVAL_MS
    const entry = await listV2Job(jobRef)
    if (!entry) continue
    rawStatus = entry.status
    const normalised = rawStatus.toUpperCase() as BaktaJobStatusEnum
    console.log(`[Bakta V2] Poll +${Math.round(elapsed / 1000)}s | Status: ${rawStatus} | Updated: ${entry.updated}`)
    const pct = Math.min(85, 25 + (elapsed / MAX_WAIT_MS) * 60)
    onProgress(`Annotating proteins… (${rawStatus})`, Math.round(pct))
    if (aidbJobId) await saveBaktaState(aidbJobId, {
      bakta_job_id: jobRef.job_id, bakta_secret: jobRef.secret,
      sequence_type: 'protein', status: normalised,
      progress_label: `Annotating proteins… (${rawStatus})`, progress_percent: Math.round(pct),
    })
    if (normalised === 'SUCCESSFUL' || normalised === 'ERROR') break
  }

  const jobStatus = rawStatus.toUpperCase() as BaktaJobStatusEnum

  if (jobStatus !== 'SUCCESSFUL') {
    onProgress('Fetching error logs…', 87)
    let logs = ''
    try { logs = await getV2Logs(jobRef) } catch { /* non-critical */ }
    console.error('[Bakta V2] Job failed | Status:', rawStatus, '\nLogs:\n', logs)
    console.groupEnd()
    if (aidbJobId) await saveBaktaState(aidbJobId, {
      bakta_job_id: jobRef.job_id, bakta_secret: jobRef.secret,
      sequence_type: 'protein', status: jobStatus,
      progress_label: 'Error', progress_percent: 87,
    })
    throw new Error(`Bakta job ended with status: ${jobStatus}${logs ? `\n\nBakta log:\n${logs}` : ''}`)
  }

  // 5 – Result URLs
  onProgress('Retrieving result URLs…', 88)
  const resultResp = await getV2Result(jobRef)
  const files = resultResp.result.files
  console.log('[Bakta V2] Results ready | Files:', Object.keys(files).join(', '))

  // 6 – Parse JSON
  let stats: BaktaJsonResult['stats'] | undefined
  let features: BaktaFeature[] | undefined
  onProgress('Parsing annotation results…', 95)
  const json = await fetchBaktaJsonResult(files.json)
  if (json) {
    stats = json.stats
    features = json.features
    console.log('[Bakta V2] JSON parsed | CDSs:', stats?.no_cdss, '| Features:', features?.length)
  } else {
    console.warn('[Bakta V2] JSON not available (likely CORS) – use web viewer for full results')
  }

  onProgress('Done', 100)
  console.log('[Bakta V2] Workflow complete')
  console.groupEnd()

  const summary: BaktaAnnotationSummary = {
    jobID: jobRef.job_id,
    secret: jobRef.secret,
    jobStatus,
    sequenceType: 'protein',
    resultFilesProtein: files,
    stats,
    featureCount: features?.length,
    features: features?.slice(0, 200),
    webViewerUrl,
  }

  if (aidbJobId) await saveBaktaState(aidbJobId, {
    bakta_job_id: jobRef.job_id, bakta_secret: jobRef.secret,
    sequence_type: 'protein', status: 'SUCCESSFUL',
    progress_label: 'Done', progress_percent: 100,
    result_files_json: JSON.stringify(files),
    result_json: JSON.stringify(summary),
  })

  return summary
}

/**
 * Main entry point – auto-detects sequence type and routes to the correct workflow.
 *
 * @param sequences  Sequences to annotate (nucleotide or protein)
 * @param options    V1 job config options (ignored for protein sequences)
 * @param onProgress Progress callback (stage label, 0–100 %)
 * @param signal     AbortSignal to cancel
 */
export async function runBaktaAnnotation(
  sequences: Array<{ id: string; sequence: string }>,
  options: BaktaJobOptions,
  onProgress: BaktaProgressCallback,
  signal?: AbortSignal,
  aidbJobId?: string,
): Promise<BaktaAnnotationSummary> {
  const seqType = detectSequenceType(sequences)
  console.log('[Bakta] Detected sequence type:', seqType)

  if (seqType === 'protein') {
    return runProteinAnnotation(sequences, onProgress, signal, aidbJobId)
  } else {
    return runNucleotideAnnotation(sequences, options, onProgress, signal, aidbJobId)
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

/** Count features by type for display. */
export function groupFeaturesByType(features: BaktaFeature[]): Record<string, number> {
  const counts: Record<string, number> = {}
  for (const f of features) counts[f.type] = (counts[f.type] ?? 0) + 1
  return counts
}

// ═══════════════════════════════════════════════════════════════════════════
// Backend persistence  (mirrors the Psos persistence pattern)
// API: POST/GET/DELETE /api/job/{aidbJobId}/bakta
// ═══════════════════════════════════════════════════════════════════════════

const API_BASE = '/api'

export interface BaktaPersistedState {
  bakta_job_id: string
  bakta_secret: string
  sequence_type: SequenceType
  status: BaktaJobStatusEnum
  progress_label: string
  progress_percent: number
  /** JSON of BaktaResultFiles | BaktaProteinsResultFiles – all S3 URLs.
   *  Refreshed on every reload of a completed job. */
  result_files_json?: string | null
  /** Full BaktaAnnotationSummary JSON (stats + features + files).
   *  Set once on first SUCCESSFUL completion. */
  result_json?: string | null
}

/**
 * Upsert Bakta job state on the AI-DB backend.
 * Called at every meaningful progress step so the user can resume after navigation.
 * Silently swallows errors – persistence is best-effort, the workflow continues either way.
 */
export async function saveBaktaState(
  aidbJobId: string,
  state: BaktaPersistedState,
): Promise<void> {
  try {
    const resp = await fetch(`${API_BASE}/job/${aidbJobId}/bakta`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(state),
    })
    if (!resp.ok) {
      console.warn(`[Bakta] State save failed (${resp.status}) for job ${aidbJobId}`)
    } else {
      console.log(`[Bakta] State saved | aidb=${aidbJobId} | status=${state.status} | ${state.progress_percent}%`)
    }
  } catch (e) {
    console.warn('[Bakta] State save error:', e)
  }
}

/**
 * Load persisted Bakta state from the AI-DB backend.
 * Returns null when no Bakta job has been started for this AI-DB job.
 */
export async function loadBaktaState(
  aidbJobId: string,
): Promise<BaktaPersistedState | null> {
  try {
    const resp = await fetch(`${API_BASE}/job/${aidbJobId}/bakta`)
    if (resp.status === 404) return null
    if (!resp.ok) {
      console.warn(`[Bakta] State load failed (${resp.status}) for job ${aidbJobId}`)
      return null
    }
    const data = await resp.json()
    return data.state as BaktaPersistedState
  } catch (e) {
    console.warn('[Bakta] State load error:', e)
    return null
  }
}

/**
 * Delete persisted Bakta state from the AI-DB backend.
 */
export async function deleteBaktaState(aidbJobId: string): Promise<void> {
  try {
    await fetch(`${API_BASE}/job/${aidbJobId}/bakta`, { method: 'DELETE' })
  } catch (e) {
    console.warn('[Bakta] State delete error:', e)
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Resume helpers
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Resume polling a Bakta job that is already running on the Bakta server.
 * Called when the user navigates back to a job page that has a persisted
 * RUNNING state. Re-uses the existing jobID + secret to poll until done,
 * then fetches result URLs and (optionally) the JSON result.
 *
 * @param aidbJobId  AI-DB job ID (for saving state)
 * @param persisted  The state loaded from the backend
 * @param onProgress Progress callback (stage label, 0–100 %)
 * @param signal     AbortSignal to cancel
 */
export async function resumeBaktaAnnotation(
  aidbJobId: string,
  persisted: BaktaPersistedState,
  onProgress: BaktaProgressCallback,
  signal?: AbortSignal,
): Promise<BaktaAnnotationSummary> {
  const jobRef = { jobID: persisted.bakta_job_id, secret: persisted.bakta_secret }
  const v2Ref  = { job_id: persisted.bakta_job_id, secret: persisted.bakta_secret }
  const isProtein = persisted.sequence_type === 'protein'
  const webViewerUrl = `https://bakta.computational.bio/ui/result?jobID=${persisted.bakta_job_id}&secret=${persisted.bakta_secret}`

  console.group(`[Bakta] Resuming ${isProtein ? 'V2 protein' : 'V1 nucleotide'} job`)
  console.log(`[Bakta] ID: ${persisted.bakta_job_id}`)
  console.log(`[Bakta] Last known status: ${persisted.status} @ ${persisted.progress_percent}%`)

  // If already done – re-fetch fresh S3 URLs (presigned URLs expire),
  // update result_files_json in storage, then return the full summary.
  if (persisted.status === 'SUCCESSFUL') {
    console.log('[Bakta] Already SUCCESSFUL – refreshing S3 result URLs')

    try {
      // Get fresh presigned URLs from Bakta
      const freshFiles = isProtein
        ? (await getV2Result(v2Ref)).result.files
        : (await getBaktaResult(jobRef)).ResultFiles

      console.log('[Bakta] Fresh URLs obtained | Files:', Object.keys(freshFiles).join(', '))

      // Rebuild summary: restore stats/features from result_json, use fresh URLs
      let summary: BaktaAnnotationSummary
      if (persisted.result_json) {
        summary = JSON.parse(persisted.result_json) as BaktaAnnotationSummary
        // Replace stored (possibly expired) URLs with fresh ones
        if (isProtein) {
          summary.resultFilesProtein = freshFiles as BaktaProteinsResultFiles
        } else {
          summary.resultFilesNucleotide = freshFiles as BaktaResultFiles
        }
      } else {
        // No cached summary – build minimal one from fresh URLs alone
        summary = {
          jobID: persisted.bakta_job_id,
          secret: persisted.bakta_secret,
          jobStatus: 'SUCCESSFUL',
          sequenceType: persisted.sequence_type,
          webViewerUrl,
          ...(isProtein
            ? { resultFilesProtein: freshFiles as BaktaProteinsResultFiles }
            : { resultFilesNucleotide: freshFiles as BaktaResultFiles }),
        }
      }

      // Persist fresh URLs back to storage so next reload gets them too
      await saveBaktaState(aidbJobId, {
        bakta_job_id: persisted.bakta_job_id,
        bakta_secret: persisted.bakta_secret,
        sequence_type: persisted.sequence_type,
        status: 'SUCCESSFUL',
        progress_label: 'Done',
        progress_percent: 100,
        result_files_json: JSON.stringify(freshFiles),
        result_json: JSON.stringify(summary),
      })

      console.groupEnd()
      return summary
    } catch (e) {
      // Bakta result endpoint failed (job expired?) – fall back to cached data
      console.warn('[Bakta] Could not refresh URLs, falling back to cached result:', e)
      if (persisted.result_json) {
        console.groupEnd()
        return JSON.parse(persisted.result_json) as BaktaAnnotationSummary
      }
      // No cached data at all – show cached file URLs from result_files_json if available
      if (persisted.result_files_json) {
        const cachedFiles = JSON.parse(persisted.result_files_json)
        console.warn('[Bakta] Serving potentially expired URLs from cache')
        console.groupEnd()
        return {
          jobID: persisted.bakta_job_id,
          secret: persisted.bakta_secret,
          jobStatus: 'SUCCESSFUL',
          sequenceType: persisted.sequence_type,
          webViewerUrl,
          ...(isProtein
            ? { resultFilesProtein: cachedFiles as BaktaProteinsResultFiles }
            : { resultFilesNucleotide: cachedFiles as BaktaResultFiles }),
        }
      }
      console.groupEnd()
      throw new Error('Bakta results are no longer available (job may have expired on the Bakta server)')
    }
  }

  // If ERROR, throw so the UI shows the error state
  if (persisted.status === 'ERROR') {
    console.groupEnd()
    throw new Error('Bakta job had previously ended with status: ERROR')
  }

  // Otherwise resume polling
  let jobStatus: BaktaJobStatusEnum = persisted.status
  let elapsed = 0
  const startPct = persisted.progress_percent

  onProgress(`Resuming… (${jobStatus})`, startPct)

  while (elapsed < MAX_WAIT_MS) {
    if (signal?.aborted) throw new Error('Aborted')

    await new Promise(r => setTimeout(r, POLL_INTERVAL_MS))
    elapsed += POLL_INTERVAL_MS

    const entry = isProtein
      ? await listV2Job(v2Ref)
      : await listBaktaJob(jobRef)

    if (!entry) continue

    // Normalise V2 lowercase status
    jobStatus = (isProtein
      ? (entry as V2JobStatus).status.toUpperCase()
      : (entry as BaktaV1JobStatus).jobStatus) as BaktaJobStatusEnum

    const updatedAt = isProtein
      ? (entry as V2JobStatus).updated
      : (entry as BaktaV1JobStatus).updated

    // Scale remaining progress (startPct → 85 %)
    const remaining = Math.max(0, 85 - startPct)
    const pct = Math.min(85, startPct + (elapsed / MAX_WAIT_MS) * remaining)
    const label = `Bakta annotating… (${jobStatus})`

    console.log(`[Bakta] Resume poll +${Math.round(elapsed / 1000)}s | Status: ${jobStatus} | Updated: ${updatedAt}`)
    onProgress(label, Math.round(pct))

    // Save progress update
    await saveBaktaState(aidbJobId, {
      bakta_job_id: persisted.bakta_job_id,
      bakta_secret: persisted.bakta_secret,
      sequence_type: persisted.sequence_type,
      status: jobStatus,
      progress_label: label,
      progress_percent: Math.round(pct),
    })

    if (jobStatus === 'SUCCESSFUL' || jobStatus === 'ERROR') break
  }

  if (jobStatus !== 'SUCCESSFUL') {
    // Fetch logs for error detail
    onProgress('Fetching error logs…', 87)
    let logs = ''
    try {
      logs = isProtein ? await getV2Logs(v2Ref) : await getBaktaLogs(jobRef)
    } catch { /* non-critical */ }
    console.error('[Bakta] Resume: job failed | Status:', jobStatus, '\nLogs:\n', logs)
    console.groupEnd()

    await saveBaktaState(aidbJobId, {
      bakta_job_id: persisted.bakta_job_id,
      bakta_secret: persisted.bakta_secret,
      sequence_type: persisted.sequence_type,
      status: jobStatus,
      progress_label: 'Error',
      progress_percent: 87,
    })

    throw new Error(`Bakta job ended with status: ${jobStatus}${logs ? `\n\nBakta log:\n${logs}` : ''}`)
  }

  // Fetch results
  onProgress('Retrieving result URLs…', 88)
  let summary: BaktaAnnotationSummary

  if (isProtein) {
    const resultResp = await getV2Result(v2Ref)
    const files = resultResp.result.files
    console.log('[Bakta] Resume V2 results ready')

    let stats: BaktaJsonResult['stats'] | undefined
    let features: BaktaFeature[] | undefined
    onProgress('Parsing annotation results…', 95)
    const json = await fetchBaktaJsonResult(files.json)
    if (json) { stats = json.stats; features = json.features }

    summary = {
      jobID: persisted.bakta_job_id,
      secret: persisted.bakta_secret,
      jobStatus,
      sequenceType: 'protein',
      resultFilesProtein: files,
      stats,
      featureCount: features?.length,
      features: features?.slice(0, 200),
      webViewerUrl,
    }
  } else {
    const resultResp = await getBaktaResult(jobRef)
    console.log('[Bakta] Resume V1 results ready')

    let stats: BaktaJsonResult['stats'] | undefined
    let features: BaktaFeature[] | undefined
    onProgress('Parsing annotation results…', 95)
    const json = await fetchBaktaJsonResult(resultResp.ResultFiles.JSON)
    if (json) { stats = json.stats; features = json.features }

    summary = {
      jobID: persisted.bakta_job_id,
      secret: persisted.bakta_secret,
      jobStatus,
      sequenceType: 'nucleotide',
      resultFilesNucleotide: resultResp.ResultFiles,
      stats,
      featureCount: features?.length,
      features: features?.slice(0, 200),
      webViewerUrl,
    }
  }

  onProgress('Done', 100)
  console.log('[Bakta] Resume complete')
  console.groupEnd()

  // Determine which files object to serialise
  const freshFiles = isProtein ? summary.resultFilesProtein : summary.resultFilesNucleotide

  // Persist the completed result with fresh URLs
  await saveBaktaState(aidbJobId, {
    bakta_job_id: persisted.bakta_job_id,
    bakta_secret: persisted.bakta_secret,
    sequence_type: persisted.sequence_type,
    status: 'SUCCESSFUL',
    progress_label: 'Done',
    progress_percent: 100,
    result_files_json: freshFiles ? JSON.stringify(freshFiles) : undefined,
    result_json: JSON.stringify(summary),
  })

  return summary
}

// ═══════════════════════════════════════════════════════════════════════════
// AI-DB Annotations DB Ingest
// API: POST /api/job/{aidbJobId}/bakta/ingest
// ═══════════════════════════════════════════════════════════════════════════

/**
 * PSC (Protein Sequence Cluster) data from a Bakta protein JSON feature.
 * Present on annotated CDS features; absent on hypothetical proteins.
 */
export interface BaktaPscData {
  uniref90_id?: string
  uniref50_id?: string
  gene?: string
  product?: string
  ec_ids?: string[]
  go_ids?: string[]
  cog_id?: string
  cog_category?: string
  kegg_orthology_id?: string
  identity?: number
  score?: number
  evalue?: number
  query_cov?: number
  subject_cov?: number
  valid?: boolean
}

/**
 * A single CDS feature from the Bakta protein workflow JSON result.
 *
 * Key field: `aa_hexdigest` – the MD5 hex digest of the protein sequence.
 * This matches exactly what AI-DB computes via `compute_md5`, so no
 * sequence matching by ID is needed for ingestion.
 */
export interface BaktaProteinFeature {
  id: string
  description?: string
  aa?: string                    // Amino acid sequence
  length: number
  type: string                   // "cds" for protein coding sequences
  locus?: string
  /** MD5 hex digest of the amino acid sequence – direct lookup key for ups table */
  aa_hexdigest?: string
  hypothetical?: boolean
  gene?: string | null
  genes?: string[]
  product?: string
  db_xrefs?: string[]
  psc?: BaktaPscData             // Present when Bakta found a PSC match
  pscc?: {
    uniref50_id?: string
    db_xrefs?: string[]
    product?: string
  }
  seq_stats?: {
    molecular_weight?: number
    isoelectric_point?: number
  }
}

export interface CustomAnnotationEntry {
  /** MD5 hex digest (aa_hexdigest from Bakta JSON) – direct key for ups table */
  md5_hash: string
  length: number
  uniparc_id?: string | null
  ncbi_nrp_id?: string | null
  /** UniRef90 ID stored as lookup-chain key (Bakta protein workflow has no UniRef100) */
  uniref100_id?: string | null
  uniref90_id?: string | null
  gene?: string | null
  product?: string | null
  ec_ids?: string | null
  go_ids?: string | null
  cog_category?: string | null
}

export interface IngestResponse {
  ingested: number
  skipped: number
  total: number
}

/**
 * POST /api/job/{aidbJobId}/bakta/ingest
 * Sends annotation entries to the backend for insertion into the AI-DB annotations DB.
 */
export async function ingestBaktaResults(
  aidbJobId: string,
  entries: CustomAnnotationEntry[],
): Promise<IngestResponse> {
  const resp = await fetch(`${API_BASE}/job/${aidbJobId}/bakta/ingest`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ entries }),
  })
  if (!resp.ok) {
    const detail = await resp.text().catch(() => '')
    throw new Error(`Ingest failed (${resp.status}): ${detail || resp.statusText}`)
  }
  return resp.json() as Promise<IngestResponse>
}

/**
 * Build ingest entries directly from Bakta protein JSON features.
 *
 * Uses `aa_hexdigest` as the direct MD5 lookup key – no sequence ID matching needed.
 *
 * Lookup chain built for each annotated CDS:
 *   ups  : aa_hexdigest → uniref90_id (stored as uniref100_id)
 *   ips  : uniref90_id  → gene, product, ec_ids, go_ids
 *   psc  : uniref90_id  → cog_category, gene, product
 *
 * Hypothetical proteins (no psc) are stored in ups only – ensuring they are
 * found as "known" on the next lookup and won't be re-submitted to Bakta.
 *
 * Data sources (in priority order):
 *   Gene / product  → feature.gene / feature.product  (top-level, already resolved)
 *   EC numbers      → psc.ec_ids  (array → comma-separated)
 *   GO terms        → psc.go_ids  (array → comma-separated)
 *   COG category    → psc.cog_category  OR  db_xrefs "COG:X" (single letter)
 *   UniRef90 ID     → psc.uniref90_id
 */
export function buildIngestEntries(
  features: BaktaProteinFeature[],
): CustomAnnotationEntry[] {
  const entries: CustomAnnotationEntry[] = []
  let annotated = 0
  let hypothetical = 0

  for (const feature of features) {
    if (feature.type !== 'cds') continue
    if (!feature.aa_hexdigest) {
      console.warn('[AI-DB Ingest] Feature missing aa_hexdigest, skipping:', feature.id)
      continue
    }

    const psc = feature.psc

    // UniRef90 ID – used as the lookup-chain key in both ups and ips tables
    const uniref90_id = psc?.uniref90_id ?? null

    // EC numbers: prefer structured psc.ec_ids, fall back to db_xrefs parsing
    let ec_ids: string | null = null
    if (psc?.ec_ids?.length) {
      ec_ids = psc.ec_ids.join(',')
    } else if (feature.db_xrefs?.length) {
      const ecRefs = feature.db_xrefs.filter(x => x.startsWith('EC:'))
      if (ecRefs.length) ec_ids = ecRefs.map(x => x.slice(3)).join(',')
    }

    // GO terms: prefer structured psc.go_ids, fall back to db_xrefs parsing
    let go_ids: string | null = null
    if (psc?.go_ids?.length) {
      go_ids = psc.go_ids.join(',')
    } else if (feature.db_xrefs?.length) {
      const goRefs = feature.db_xrefs.filter(x => x.startsWith('GO:'))
      if (goRefs.length) go_ids = goRefs.join(',')
    }

    // COG functional category: single letter (e.g. "H", "J", "K")
    // Prefer psc.cog_category; fall back to db_xrefs "COG:X" (exactly 5 chars → single letter)
    let cog_category: string | null = psc?.cog_category ?? null
    if (!cog_category && feature.db_xrefs?.length) {
      const cogRef = feature.db_xrefs.find(x => x.startsWith('COG:') && x.length === 5)
      if (cogRef) cog_category = cogRef.slice(4)
    }

    if (uniref90_id) annotated++
    else hypothetical++

    entries.push({
      md5_hash:     feature.aa_hexdigest,
      length:       feature.length,
      // Store UniRef90 ID as uniref100_id for the ups→ips lookup chain
      uniref100_id: uniref90_id,
      uniref90_id,
      // Use top-level gene/product (Bakta already resolves the best name)
      // Hypotheticals have product = "hypothetical protein" → store as null
      gene:         feature.hypothetical ? null : (feature.gene ?? null),
      product:      feature.hypothetical ? null : (feature.product ?? null),
      ec_ids,
      go_ids,
      cog_category,
    })
  }

  console.log(
    `[AI-DB Ingest] ${entries.length} entries built:`,
    `${annotated} annotated (ups+ips+psc),`,
    `${hypothetical} hypothetical (ups only)`,
  )

  return entries
}
