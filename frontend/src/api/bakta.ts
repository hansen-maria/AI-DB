/**
 * Bakta Web API Client
 * Spec: https://api.bakta.computational.bio/swagger-ui/
 * Flow: init → upload FASTA to S3 → start → poll (list) → result → fetch JSON
 */

const BAKTA_API_BASE = 'https://api.bakta.computational.bio/api/v1'

export type RepliconTableType = 'CSV' | 'TSV'
export type DermType = 'UNKNOWN' | 'MONODERM' | 'DIDERM'
export type BaktaJobStatusEnum = 'INIT' | 'RUNNING' | 'SUCCESSFUL' | 'ERROR'
export type FailedJobStatusEnum = 'NOT_FOUND' | 'UNAUTHORIZED'

/** Schema: Job */
export interface BaktaJobRef {
  jobID: string   // uuid
  secret: string
}

/** Schema: JobConfig – required fields + optional nullable fields */
export interface BaktaJobConfig {
  // Required by spec
  translationTable: number        // int32, >= 0; typically 11 (Bacteria/Archaea)
  completeGenome: boolean
  keepContigHeaders: boolean
  minContigLength: number         // int64, >= 0
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

/** Schema: InitRequest */
export interface BaktaInitRequest {
  name: string
  repliconTableType: RepliconTableType
}

/** Schema: InitResponse – all fields required */
export interface BaktaInitResponse {
  job: BaktaJobRef
  uploadLinkFasta: string
  uploadLinkProdigal: string
  uploadLinkReplicons: string
}

/** Schema: JobStatus – all fields required */
export interface BaktaJobStatus {
  jobID: string
  jobStatus: BaktaJobStatusEnum
  started: string       // date-time
  updated: string       // date-time
  name: string
}

/** Schema: FailedJobStatus */
export interface BaktaFailedJobStatus {
  jobID: string
  jobStatus: FailedJobStatusEnum
}

/** Schema: ListRequest */
export interface BaktaListRequest {
  jobs: BaktaJobRef[]
}

/** Schema: ListResponse */
export interface BaktaListResponse {
  jobs: BaktaJobStatus[]
  failedJobs: BaktaFailedJobStatus[]
}

/** Schema: StartRequest */
export interface BaktaStartRequest {
  job: BaktaJobRef
  config: BaktaJobConfig
}

/** Schema: ResultFiles – all fields required */
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

/** Schema: ResultResponse – all fields required */
export interface BaktaResultResponse {
  ResultFiles: BaktaResultFiles
  jobID: string
  name: string
  started: string       // date-time
  updated: string       // date-time
}

/** Schema: VersionResponse */
export interface BaktaVersionResponse {
  toolVersion: string
  dbVersion: string
  backendVersion: string
}

// ── Result JSON types (Bakta annotation output) ───────────

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

// ── High-level summary returned by runBaktaAnnotation ─────────────────────

export interface BaktaAnnotationSummary {
  jobID: string
  secret: string
  jobStatus: BaktaJobStatusEnum
  resultFiles: BaktaResultFiles
  stats?: BaktaJsonResult['stats']
  featureCount?: number
  features?: BaktaFeature[]   // first 200 for display
  webViewerUrl: string
}

// ── User-facing config options (all optional, merged with defaults) ────────

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

/** Default values for all required JobConfig fields */
const DEFAULT_CONFIG: BaktaJobConfig = {
  translationTable: 11,
  completeGenome: false,
  keepContigHeaders: true,
  minContigLength: 0,
  compliant: false,
}

/** Merge user options with defaults to produce a fully valid JobConfig */
export function buildJobConfig(options: BaktaJobOptions = {}): BaktaJobConfig {
  return {
    ...DEFAULT_CONFIG,
    completeGenome: options.completeGenome ?? DEFAULT_CONFIG.completeGenome,
    compliant: options.compliant ?? DEFAULT_CONFIG.compliant,
    keepContigHeaders: options.keepContigHeaders ?? DEFAULT_CONFIG.keepContigHeaders,
    minContigLength: options.minContigLength ?? DEFAULT_CONFIG.minContigLength,
    translationTable: options.translationTable ?? DEFAULT_CONFIG.translationTable,
    // Optional / nullable fields – only include when provided
    ...(options.genus !== undefined && { genus: options.genus }),
    ...(options.species !== undefined && { species: options.species }),
    ...(options.strain !== undefined && { strain: options.strain }),
    ...(options.dermType !== undefined && { dermType: options.dermType }),
    ...(options.locus !== undefined && { locus: options.locus }),
    ...(options.locusTag !== undefined && { locusTag: options.locusTag }),
    ...(options.plasmid !== undefined && { plasmid: options.plasmid }),
  }
}

// ── API calls ──────────────────────────────────────────────────────────────

/**
 * Step 1 – POST /api/v1/job/init
 * Creates a new job and returns presigned S3 upload URLs.
 */
export async function initBaktaJob(
  name: string,
  repliconTableType: RepliconTableType = 'TSV',
): Promise<BaktaInitResponse> {
  const body: BaktaInitRequest = { name, repliconTableType }
  const resp = await fetch(`${BAKTA_API_BASE}/job/init`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify(body),
  })
  if (!resp.ok) {
    const detail = await resp.text().catch(() => '')
    throw new Error(`Bakta init failed (${resp.status}): ${detail || resp.statusText}`)
  }
  return resp.json() as Promise<BaktaInitResponse>
}

/**
 * Step 2 – PUT <presigned S3 URL>
 * Uploads the FASTA content directly to the S3 presigned URL.
 */
export async function uploadFastaToS3(uploadUrl: string, fastaContent: string): Promise<void> {
  const resp = await fetch(uploadUrl, {
    method: 'PUT',
    body: new Blob([fastaContent], { type: 'text/plain' }),
  })
  if (!resp.ok) {
    throw new Error(`S3 upload failed (${resp.status}): ${resp.statusText}`)
  }
}

/**
 * Step 3 – POST /api/v1/job/start
 * Starts the annotation job. Config must contain all required fields;
 * use buildJobConfig() to construct it safely.
 */
export async function startBaktaJob(
  job: BaktaJobRef,
  config: BaktaJobConfig,
): Promise<void> {
  const body: BaktaStartRequest = { job, config }
  const resp = await fetch(`${BAKTA_API_BASE}/job/start`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify(body),
  })
  if (!resp.ok) {
    const detail = await resp.text().catch(() => '')
    throw new Error(`Bakta start failed (${resp.status}): ${detail || resp.statusText}`)
  }
}

/**
 * Step 4 – POST /api/v1/job/list
 * Returns the current status of the given job.
 * Throws if the job appears in failedJobs (NOT_FOUND / UNAUTHORIZED).
 */
export async function listBaktaJob(job: BaktaJobRef): Promise<BaktaJobStatus | null> {
  const body: BaktaListRequest = { jobs: [job] }
  const resp = await fetch(`${BAKTA_API_BASE}/job/list`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify(body),
  })
  if (!resp.ok) {
    throw new Error(`Bakta list failed (${resp.status}): ${resp.statusText}`)
  }
  const data: BaktaListResponse = await resp.json()

  const failed = data.failedJobs?.find(f => f.jobID === job.jobID)
  if (failed) {
    throw new Error(`Bakta job ${job.jobID} could not be found: ${failed.jobStatus}`)
  }

  return data.jobs?.[0] ?? null
}

/**
 * Step 5 – POST /api/v1/job/result
 * Returns presigned S3 URLs for all result files.
 */
export async function getBaktaResult(job: BaktaJobRef): Promise<BaktaResultResponse> {
  const resp = await fetch(`${BAKTA_API_BASE}/job/result`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify(job),
  })
  if (!resp.ok) {
    const detail = await resp.text().catch(() => '')
    throw new Error(`Bakta result fetch failed (${resp.status}): ${detail || resp.statusText}`)
  }
  return resp.json() as Promise<BaktaResultResponse>
}

/**
 * Step 6 (optional) – GET <ResultFiles.JSON>
 * Fetches and parses the Bakta JSON result from S3 for inline display.
 * Returns null on any error (e.g. CORS); caller handles gracefully.
 */
export async function fetchBaktaJsonResult(jsonUrl: string): Promise<BaktaJsonResult | null> {
  try {
    const resp = await fetch(jsonUrl)
    if (!resp.ok) return null
    return resp.json() as Promise<BaktaJsonResult>
  } catch {
    return null
  }
}

/**
 * GET /api/v1/job/logs?jobID=&secret=
 * Returns stdout/stderr logs of a job. Useful for diagnosing ERROR status.
 */
export async function getBaktaLogs(job: BaktaJobRef): Promise<string> {
  const url = new URL(`${BAKTA_API_BASE}/job/logs`)
  url.searchParams.set('jobID', job.jobID)
  url.searchParams.set('secret', job.secret)
  const resp = await fetch(url.toString(), {
    headers: { Accept: 'application/json, text/plain, */*' },
  })
  if (!resp.ok) {
    throw new Error(`Bakta logs fetch failed (${resp.status}): ${resp.statusText}`)
  }
  // API returns JSON (schema: default: null) – stringify if object, else plain text
  const text = await resp.text()
  try {
    const parsed = JSON.parse(text)
    if (parsed === null) return ''
    return typeof parsed === 'string' ? parsed : JSON.stringify(parsed, null, 2)
  } catch {
    return text
  }
}

/**
 * DELETE /api/v1/job/delete?jobID=&secret=
 * Optional cleanup after results have been retrieved.
 */
export async function deleteBaktaJob(job: BaktaJobRef): Promise<void> {
  const url = new URL(`${BAKTA_API_BASE}/job/delete`)
  url.searchParams.set('jobID', job.jobID)
  url.searchParams.set('secret', job.secret)
  const resp = await fetch(url.toString(), { method: 'DELETE' })
  if (!resp.ok) {
    throw new Error(`Bakta delete failed (${resp.status}): ${resp.statusText}`)
  }
}

/**
 * GET /api/v1/version
 */
export async function getBaktaVersion(): Promise<BaktaVersionResponse> {
  const resp = await fetch(`${BAKTA_API_BASE}/version`, {
    headers: { Accept: 'application/json' },
  })
  if (!resp.ok) throw new Error(`Bakta version check failed: ${resp.statusText}`)
  return resp.json() as Promise<BaktaVersionResponse>
}

// ── Full annotation workflow ───────────────────────────────────────────────

export type BaktaProgressCallback = (stage: string, percent: number) => void

const POLL_INTERVAL_MS = 5_000
const MAX_WAIT_MS = 20 * 60 * 1_000   // 20 min

/**
 * Full Bakta annotation workflow:
 * init → S3 upload → start → poll → result → (optional) parse JSON
 *
 * @param sequences  Nucleotide sequences to annotate
 * @param options    Optional job config merged with safe defaults
 * @param onProgress Progress callback (stage label, 0–100 %)
 * @param signal     AbortSignal to cancel
 */
export async function runBaktaAnnotation(
  sequences: Array<{ id: string; sequence: string }>,
  options: BaktaJobOptions,
  onProgress: BaktaProgressCallback,
  signal?: AbortSignal,
): Promise<BaktaAnnotationSummary> {
  const fastaContent = sequences.map(s => `>${s.id}\n${s.sequence}`).join('\n')
  const jobName = `aidb-unmatched-${Date.now()}`
  const config = buildJobConfig(options)

  console.group('[Bakta] Starting annotation workflow')
  console.log('Sequences:', sequences.length, '| Job name:', jobName)
  console.log('Config:', config)

  // 1 – Init
  onProgress('Initializing Bakta job…', 5)
  if (signal?.aborted) throw new Error('Aborted')
  const init = await initBaktaJob(jobName)
  const jobRef = init.job
  const webViewerUrl =
    `https://bakta.computational.bio/ui/result?jobID=${jobRef.jobID}&secret=${jobRef.secret}`

  console.log('[Bakta] Job initialized | ID:', jobRef.jobID)
  console.log('[Bakta] Web viewer:', webViewerUrl)

  // 2 – Upload FASTA
  onProgress('Uploading sequences…', 15)
  if (signal?.aborted) throw new Error('Aborted')
  await uploadFastaToS3(init.uploadLinkFasta, fastaContent)

  console.log('[Bakta] FASTA uploaded | Size:', fastaContent.length, 'chars')

  // 3 – Start job
  onProgress('Starting Bakta annotation…', 25)
  if (signal?.aborted) throw new Error('Aborted')
  await startBaktaJob(jobRef, config)

  console.log('[Bakta] Job started')

  // 4 – Poll until SUCCESSFUL or ERROR
  let jobStatus: BaktaJobStatusEnum = 'RUNNING'
  let elapsed = 0

  while (elapsed < MAX_WAIT_MS) {
    if (signal?.aborted) throw new Error('Aborted')

    await new Promise(r => setTimeout(r, POLL_INTERVAL_MS))
    elapsed += POLL_INTERVAL_MS

    const entry = await listBaktaJob(jobRef)
    if (!entry) continue

    jobStatus = entry.jobStatus

    console.log(`[Bakta] Poll +${Math.round(elapsed / 1000)}s | Status: ${jobStatus} | Updated: ${entry.updated}`)

    // Scale 25 → 85 % over the polling window
    const pollPct = Math.min(85, 25 + (elapsed / MAX_WAIT_MS) * 60)
    onProgress(`Bakta annotating… (${jobStatus})`, Math.round(pollPct))

    if (jobStatus === 'SUCCESSFUL' || jobStatus === 'ERROR') break
  }

  if (jobStatus !== 'SUCCESSFUL') {
    // Fetch logs so the user sees the actual error from Bakta
    onProgress('Fetching error logs…', 87)
    let logs = ''
    try {
      logs = await getBaktaLogs(jobRef)
    } catch {
      // Logs not critical – proceed with generic message if unavailable
    }
    console.error('[Bakta] Job failed | Status:', jobStatus, '\nLogs:\n', logs)
    console.groupEnd()
    const detail = logs ? `\n\nBakta log:\n${logs}` : ''
    throw new Error(`Bakta job ended with status: ${jobStatus}${detail}`)
  }

  // 5 – Get result URLs
  onProgress('Retrieving result URLs…', 88)
  const resultResp = await getBaktaResult(jobRef)

  console.log('[Bakta] Results ready | Files:', Object.keys(resultResp.ResultFiles).join(', '))

  // 6 – Parse JSON result for inline stats (may fail silently on CORS)
  let stats: BaktaJsonResult['stats'] | undefined
  let features: BaktaFeature[] | undefined

  onProgress('Parsing annotation results…', 95)
  const json = await fetchBaktaJsonResult(resultResp.ResultFiles.JSON)
  if (json) {
    stats = json.stats
    features = json.features
    console.log('[Bakta] JSON parsed | CDSs:', stats?.no_cdss, '| Features:', features?.length, '| GC:', stats?.gc?.toFixed(3))
  } else {
    console.warn('[Bakta] JSON result could not be fetched (likely CORS) – results available via web viewer')
  }

  onProgress('Done', 100)
  console.log('[Bakta] Workflow complete')
  console.groupEnd()

  return {
    jobID: jobRef.jobID,
    secret: jobRef.secret,
    jobStatus,
    resultFiles: resultResp.ResultFiles,
    stats,
    featureCount: features?.length,
    features: features?.slice(0, 200),
    webViewerUrl,
  }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/** Count features by type for display */
export function groupFeaturesByType(features: BaktaFeature[]): Record<string, number> {
  const counts: Record<string, number> = {}
  for (const f of features) {
    counts[f.type] = (counts[f.type] ?? 0) + 1
  }
  return counts
}
