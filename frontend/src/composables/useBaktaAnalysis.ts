import { ref, type Ref, type ComputedRef } from 'vue'
import {
  runBaktaAnnotation, resumeBaktaAnnotation,
  loadBaktaState, deleteBaktaState, groupFeaturesByType,
  ingestBaktaResults, buildIngestEntries,
  type BaktaJobOptions, type BaktaAnnotationSummary,
  type IngestResponse, type BaktaProteinFeature,
} from '../api/bakta.ts'

/**
 * Encapsulates all Bakta-related state and actions.
 * Re-exports `groupFeaturesByType` and `deleteBaktaState` for use in the
 * results template.
 */
export function useBaktaAnalysis(
  jobId: Ref<string>,
  unmatchedSequences: ComputedRef<any[]>,
) {
  // ── State ──────────────────────────────────────────────────────────────────

  const showBaktaPanel       = ref(false)
  const baktaAnalyzing       = ref(false)
  const baktaProgressLabel   = ref('')
  const baktaProgressPercent = ref(0)
  const baktaError           = ref('')
  const baktaResult          = ref<BaktaAnnotationSummary | null>(null)
  const baktaAbortController = ref<AbortController | null>(null)

  // Config form
  const baktaGenus          = ref('')
  const baktaSpecies        = ref('')
  const baktaCompleteGenome = ref(false)

  // Ingest state
  const baktaIngesting      = ref(false)
  const baktaIngestResult   = ref<IngestResponse | null>(null)
  const baktaIngestError    = ref('')

  // ── Restore persisted state on job load ────────────────────────────────────

  async function loadExistingState() {
    const persisted = await loadBaktaState(jobId.value)
    if (!persisted) return

    console.log('[Bakta] Persisted state | status:', persisted.status, '| type:', persisted.sequence_type)
    showBaktaPanel.value = true

    if (persisted.status === 'SUCCESSFUL') {
      baktaProgressPercent.value = 100
      baktaProgressLabel.value   = 'Done'
      baktaAnalyzing.value       = true
      baktaAbortController.value = new AbortController()

      resumeBaktaAnnotation(jobId.value, persisted, () => {}, baktaAbortController.value.signal)
        .then(summary => {
          baktaResult.value = summary
          console.log('[Bakta] Restored completed result with fresh URLs')
        })
        .catch(e => {
          baktaError.value = e instanceof Error ? e.message : 'Failed to restore Bakta results.'
        })
        .finally(() => {
          baktaAnalyzing.value       = false
          baktaAbortController.value = null
        })
      return
    }

    if (persisted.status === 'ERROR') {
      baktaError.value           = 'Bakta job previously ended with status: ERROR'
      baktaProgressPercent.value = persisted.progress_percent
      baktaProgressLabel.value   = persisted.progress_label
      return
    }

    // Still running – resume polling in background
    baktaProgressPercent.value = persisted.progress_percent
    baktaProgressLabel.value   = persisted.progress_label + ' (resuming…)'
    baktaAnalyzing.value       = true
    baktaError.value           = ''
    baktaAbortController.value = new AbortController()

    console.log('[Bakta] Resuming background polling…')

    resumeBaktaAnnotation(
      jobId.value,
      persisted,
      (stage, pct) => { baktaProgressLabel.value = stage; baktaProgressPercent.value = pct },
      baktaAbortController.value.signal,
    )
      .then(summary => { baktaResult.value = summary })
      .catch(e => {
        baktaError.value = e instanceof Error && e.message === 'Aborted'
          ? 'Analysis cancelled.'
          : (e instanceof Error ? e.message : 'Bakta analysis failed.')
      })
      .finally(() => {
        baktaAnalyzing.value       = false
        baktaAbortController.value = null
      })
  }

  // ── Run annotation ─────────────────────────────────────────────────────────

  async function analyzeWithBakta() {
    const sequences = unmatchedSequences.value.filter(s => s.sequence)
    if (!sequences.length) return

    baktaAnalyzing.value       = true
    baktaError.value           = ''
    baktaResult.value          = null
    baktaProgressPercent.value = 0
    baktaProgressLabel.value   = 'Starting…'
    baktaAbortController.value = new AbortController()

    const config: BaktaJobOptions = { completeGenome: baktaCompleteGenome.value }
    if (baktaGenus.value.trim())   config.genus   = baktaGenus.value.trim()
    if (baktaSpecies.value.trim()) config.species = baktaSpecies.value.trim()

    try {
      baktaResult.value = await runBaktaAnnotation(
        sequences.map(s => ({ id: s.id, sequence: s.sequence })),
        config,
        (stage, pct) => { baktaProgressLabel.value = stage; baktaProgressPercent.value = pct },
        baktaAbortController.value.signal,
        jobId.value,
      )
    } catch (e) {
      baktaError.value = e instanceof Error && e.message === 'Aborted'
        ? 'Analysis cancelled.'
        : (e instanceof Error ? e.message : 'Bakta analysis failed.')
    } finally {
      baktaAnalyzing.value       = false
      baktaAbortController.value = null
    }
  }

  // ── Ingest results into AI-DB DB ───────────────────────────────────────────

  async function ingestBaktaAnnotations() {
    if (!baktaResult.value || baktaIngesting.value) return

    baktaIngesting.value    = true
    baktaIngestError.value  = ''
    baktaIngestResult.value = null

    try {
      let features   = (baktaResult.value.features as unknown as BaktaProteinFeature[]) ?? []
      const cached   = features.length
      const total    = baktaResult.value.featureCount ?? cached

      if (total > cached) {
        const jsonUrl = baktaResult.value.resultFilesProtein?.json ?? baktaResult.value.resultFilesNucleotide?.JSON
        if (jsonUrl) {
          try {
            const resp = await fetch(jsonUrl)
            if (resp.ok) {
              const full = await resp.json()
              features   = (full.features ?? features) as BaktaProteinFeature[]
              console.log(`[AI-DB Ingest] Fetched full JSON: ${features.length} features (cached: ${cached})`)
            }
          } catch {
            console.warn('[AI-DB Ingest] Could not fetch full JSON from S3, using cache')
          }
        }
      }

      if (!features.length)       { baktaIngestError.value = 'No features available to ingest.'; return }
      const entries = buildIngestEntries(features)
      if (!entries.length)        { baktaIngestError.value = 'No CDS features with aa_hexdigest found.'; return }

      baktaIngestResult.value = await ingestBaktaResults(jobId.value, entries)
      console.log('[AI-DB Ingest] Result:', baktaIngestResult.value)
    } catch (e) {
      baktaIngestError.value = e instanceof Error ? e.message : 'Ingest failed.'
    } finally {
      baktaIngesting.value = false
    }
  }

  function resetBakta() {
    deleteBaktaState(jobId.value)
    baktaResult.value      = null
    baktaError.value       = ''
    baktaIngestResult.value = null
  }

  return {
    // State
    showBaktaPanel, baktaAnalyzing, baktaProgressLabel, baktaProgressPercent,
    baktaError, baktaResult, baktaAbortController,
    baktaGenus, baktaSpecies, baktaCompleteGenome,
    baktaIngesting, baktaIngestResult, baktaIngestError,
    // Re-exported helpers needed by the template
    groupFeaturesByType,
    // Actions
    loadExistingState,
    analyzeWithBakta,
    ingestBaktaAnnotations,
    resetBakta,
  }
}
