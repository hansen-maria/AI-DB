import { ref, type Ref } from 'vue'
import {
  getJob, getJobStats,
  type PaginatedJobResponse, type FunctionalStats,
} from '../api/jobs.ts'

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

  // ── Public API ─────────────────────────────────────────────────────────────

  async function loadJob(onCompleted?: () => Promise<void>) {
    loading.value = true
    error.value   = ''
    try {
      // NOTE: intentionally NOT passing includeSequences here – the raw
      // `sequence` text field is by far the largest part of the payload
      // (full protein/DNA text × up to 10,000 entries) and isn't used
      // anywhere in the table/filtering UI. For a completed job, unmatched
      // sequences' text is fetched separately below (much smaller subset,
      // filtered server-side) only once it's actually needed.
      const response    = await getJob(jobId.value, 1, 10_000, 'all')
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
      const response = await getJob(jobId.value, 1, 10_000, 'none', undefined, true)
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

  function startPolling(onCompleted?: () => Promise<void>) {
    if (pollInterval) return
    pollInterval = window.setInterval(async () => {
      try {
        const response     = await getJob(jobId.value, 1, 10_000, 'all')
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
      }
    }, 1_000)
  }

  function stopPolling() {
    if (pollInterval) {
      clearInterval(pollInterval)
      pollInterval = null
    }
  }

  return { job, allSequences, stats, loading, error, loadJob, loadStats, stopPolling }
}
