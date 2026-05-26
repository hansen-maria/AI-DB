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
      const response    = await getJob(jobId.value, 1, 10_000, 'all')
      job.value         = response
      allSequences.value = response.sequences || []

      if (response.status === 'pending' || response.status === 'processing') {
        startPolling(onCompleted)
      } else if (response.status === 'completed') {
        if (!stats.value) await loadStats()
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
