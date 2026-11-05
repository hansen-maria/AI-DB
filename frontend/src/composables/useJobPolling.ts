import { ref, type Ref } from 'vue'
import {
  getJob, getJobStats,
  type PaginatedJobResponse, type FunctionalStats, type SequenceFilter, type SequenceInfo,
} from '../api/jobs.ts'

/** Must match MAX_PER_PAGE in crates/aidb-types/src/pagination.rs (server clamps to it). */
const FETCH_PAGE_SIZE = 10_000

/**
 * Fetches ALL sequences of a job by walking through every page.
 * The backend caps `per_page` at 10,000, so a single request silently
 * truncates larger jobs. Returns the first page's response (job metadata)
 * with `sequences` replaced by the full, concatenated list.
 *
 * @param stopWhen Optional early-exit check, evaluated after each page.
 */
async function getJobAllSequences(
    jobId: string,
    filter: SequenceFilter,
    includeSequences = false,
    stopWhen?: (seqs: SequenceInfo[]) => boolean,
): Promise<PaginatedJobResponse> {
  const first = await getJob(jobId, 1, FETCH_PAGE_SIZE, filter, undefined, includeSequences)
  const sequences: SequenceInfo[] = [...(first.sequences || [])]
  const totalPages = first.pagination?.total_pages ?? 1

  for (let page = 2; page <= totalPages; page++) {
    if (stopWhen?.(sequences)) break
    const res = await getJob(jobId, page, FETCH_PAGE_SIZE, filter, undefined, includeSequences)
    sequences.push(...(res.sequences || []))
  }

  return { ...first, sequences }
}

/**
 * Manages fetching job data, background polling while the job is running,
 * and loading functional stats once the job completes.
 *
 * All Psos / Bakta state restoration is delegated back to the caller via
 * the returned `onJobCompleted` callback array so this composable stays
 * focused on a single concern.
 */
export function useJobPolling(jobId: Ref<string>) {
  const job        = ref<PaginatedJobResponse | null>(null)
  const allSequences = ref<any[]>([])
  const stats      = ref<FunctionalStats | null>(null)
  const loading    = ref(true)
  const error      = ref('')

  let pollInterval: number | null = null
  let pollInFlight = false

  // ── Public API ─────────────────────────────────────────────────────────────

  async function loadJob(onCompleted?: () => Promise<void>) {
    loading.value = true
    error.value   = ''
    try {
      // NOTE: intentionally NOT passing includeSequences here – the raw
      // `sequence` text field is by far the largest part of the payload
      // (full protein/DNA text × every entry) and isn't used
      // anywhere in the table/filtering UI. For a completed job, unmatched
      // sequences' text is fetched separately below (much smaller subset,
      // filtered server-side) only once it's actually needed.
      const response    = await getJobAllSequences(jobId.value, 'all')
      job.value         = response
      allSequences.value = response.sequences || []

      if (response.status === 'pending' || response.status === 'processing') {
        startPolling(onCompleted)
      } else if (response.status === 'completed') {
        if (!stats.value) await loadStats()
        await loadUnmatchedSequenceText()
        await onCompleted?.()
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to load job'
    } finally {
      loading.value = false
    }
  }

  async function loadStats() {
    if (!job.value || job.value.status !== 'completed') return
    try {
      stats.value = await getJobStats(jobId.value)
    } catch (e) {
      console.error('Failed to load stats:', e)
    }
  }

  /**
   * Fetches the raw `sequence` text for unmatched sequences only (server-side
   * `filter=none`, so this is typically a much smaller response than the full
   * job) and merges it into `allSequences` by id. The Bakta and Psos
   * "unmatched sequences" workflows both need the actual sequence text
   * (to build FASTA uploads); everything else in the UI works fine without it.
   * Safe/cheap to call repeatedly – merges by id, never duplicates entries.
   */
  async function loadUnmatchedSequenceText() {
    try {
      const response = await getJobAllSequences(jobId.value, 'none', true)
      const textById = new Map((response.sequences || []).map(s => [s.id, s.sequence]))
      if (textById.size === 0) return
      allSequences.value = allSequences.value.map(s =>
          textById.has(s.id) ? { ...s, sequence: textById.get(s.id) } : s,
      )
    } catch (e) {
      // Non-critical: Bakta/Psos will just show "no sequence text available"
      // for affected entries rather than blocking the whole page load.
      console.warn('Failed to load unmatched sequence text:', e)
    }
  }

  /**
   * Fetches the raw sequence text for specific sequence ids (e.g. community
   * entries that are re-checked with Bakta) and merges it into `allSequences`.
   * Needs the full job payload with sequences, so it is only called on demand.
   */
  async function loadSequenceText(ids: string[]) {
    if (ids.length === 0) return
    const wanted = new Set(ids)
    try {
      // Stop paging as soon as every requested id has been seen.
      const response = await getJobAllSequences(
          jobId.value, 'all', true,
          seqs => seqs.reduce((n, s) => n + (wanted.has(s.id) ? 1 : 0), 0) >= wanted.size,
      )
      const textById = new Map(
        (response.sequences || []).filter(s => wanted.has(s.id)).map(s => [s.id, s.sequence]),
      )
      allSequences.value = allSequences.value.map(s =>
          textById.has(s.id) ? { ...s, sequence: textById.get(s.id) } : s,
      )
    } catch (e) {
      console.warn('Failed to load sequence text for re-check:', e)
    }
  }

  function startPolling(onCompleted?: () => Promise<void>) {
    if (pollInterval) return
    pollInterval = window.setInterval(async () => {
      // Fetching all pages of a large job can take longer than the interval –
      // skip ticks instead of stacking overlapping requests.
      if (pollInFlight) return
      pollInFlight = true
      try {
        const response     = await getJobAllSequences(jobId.value, 'all')
        job.value          = response
        allSequences.value = response.sequences || []
        if (response.status === 'completed' || response.status === 'failed') {
          stopPolling()
          if (response.status === 'completed') {
            await loadStats()
            await loadUnmatchedSequenceText()
            await onCompleted?.()
          }
        }
      } catch {
        stopPolling()
      } finally {
        pollInFlight = false
      }
    }, 1_000)
  }

  function stopPolling() {
    if (pollInterval) {
      clearInterval(pollInterval)
      pollInterval = null
    }
  }

  return { job, allSequences, stats, loading, error, loadJob, loadStats, loadSequenceText, stopPolling }
}
