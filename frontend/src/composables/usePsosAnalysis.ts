import { ref, type Ref, type ComputedRef } from 'vue'
import {
  submitToPsos, pollPsosJob, getPsosJobUrl, getPsosFile, parsePsosResult,
  openInPsos, downloadForPsos,
  psosProfiles, type PsosProfile, type PsosAnnotation,
  savePsosResults, loadPsosResults,
} from '../api/psos.ts'

/**
 * Encapsulates all Psos-related state and actions.
 * Accepts reactive references so it stays in sync with the job data.
 */
export function usePsosAnalysis(
  jobId: Ref<string>,
  unmatchedSequences: ComputedRef<any[]>,
  jobFilename: ComputedRef<string | undefined>,
) {
  const selectedPsosProfile = ref<PsosProfile>('bacteria-gram-')
  const showPsosPanel       = ref(false)
  const psosAnalyzing       = ref(false)
  const psosProgress        = ref(0)
  const psosTotal           = ref(0)
  const psosError           = ref('')
  const psosResults         = ref<Map<string, PsosAnnotation>>(new Map())
  const psosCopied          = ref(false)

  // ── Load persisted results ─────────────────────────────────────────────────

  async function loadExistingResults() {
    try {
      const results = await loadPsosResults(jobId.value)
      if (results.length > 0) {
        psosResults.value = new Map(results.map(r => [r.sequenceId, r]))
        showPsosPanel.value = true
        console.log(`[Psos] Loaded ${results.length} existing results`)
      }
    } catch (e) {
      console.error('[Psos] Failed to load persisted results:', e)
    }
  }

  // ── Analyze ────────────────────────────────────────────────────────────────

  async function analyzeWithPsos() {
    if (!unmatchedSequences.value.length) return

    psosAnalyzing.value = true
    psosError.value     = ''
    psosProgress.value  = 0
    psosTotal.value     = unmatchedSequences.value.length
    psosResults.value.clear()

    try {
      for (let i = 0; i < unmatchedSequences.value.length; i++) {
        const seq = unmatchedSequences.value[i]
        if (!seq.sequence) { psosProgress.value = i + 1; continue }

        try {
          const psosJob      = await submitToPsos(seq.id, seq.sequence, selectedPsosProfile.value)
          const completedJob = await pollPsosJob(psosJob.id, undefined, 60, 3_000)
          const jobState     = completedJob.state?.value?.toLowerCase() || ''

          if (jobState === 'succeeded' && completedJob.data?.files) {
            const resultFile = completedJob.data.files.find(
              (f: any) => f.type === 'result' && f.name.endsWith('.json') && f.name !== 'config.json',
            )
            if (resultFile) {
              const data   = JSON.parse(await getPsosFile(psosJob.id, resultFile.name))
              const parsed = parsePsosResult(data)
              psosResults.value.set(seq.id, { sequenceId: seq.id, psosJobId: psosJob.id, ...parsed })
            } else {
              psosResults.value.set(seq.id, { sequenceId: seq.id, psosJobId: psosJob.id })
            }
          }
        } catch (e) {
          console.error(`[Psos] Analysis failed for ${seq.id}:`, e)
        }

        psosProgress.value = i + 1
      }

      // Persist results
      if (psosResults.value.size > 0) {
        try {
          const arr = Array.from(psosResults.value.values())
          const { savedCount, totalCount } = await savePsosResults(jobId.value, arr)
          console.log(`[Psos] Saved ${savedCount}/${totalCount} results`)
        } catch (e) {
          console.error('[Psos] Failed to save results:', e)
          psosError.value = 'Results could not be saved and will be lost on reload.'
        }
      }
    } catch (e) {
      psosError.value = e instanceof Error ? e.message : 'Psos analysis failed'
    } finally {
      psosAnalyzing.value = false
    }
  }

  // ── Fallback helpers ───────────────────────────────────────────────────────

  async function handleOpenInPsos() {
    const sequences = unmatchedSequences.value
      .filter(s => s.sequence)
      .map(s => ({ id: s.id, sequence: s.sequence }))
    if (!sequences.length) return

    await openInPsos(sequences)
    psosCopied.value = true
    setTimeout(() => { psosCopied.value = false }, 3_000)
  }

  function handleDownloadForPsos() {
    const sequences = unmatchedSequences.value
      .filter(s => s.sequence)
      .map(s => ({ id: s.id, sequence: s.sequence }))
    if (!sequences.length) return

    const filename = jobFilename.value
      ? `${jobFilename.value.replace(/\.[^.]+$/, '')}_unmatched.fasta`
      : 'unmatched_sequences.fasta'

    downloadForPsos(sequences, filename)
  }

  // ── Download results TSV ───────────────────────────────────────────────────

  function downloadPsosResults() {
    if (!psosResults.value.size) return

    const header = ['Sequence ID', 'Protein Name', 'Best Hit (dbxref)', 'E-value', 'Identity (%)', 'Signal Peptide', 'TM Domains', 'Psos URL']
    const rows   = [header.join('\t')]

    for (const [seqId, result] of psosResults.value) {
      rows.push([
        seqId,
        result.proteinName || '',
        result.bestHit?.dbxref || '',
        result.bestHit?.evalue?.toExponential(2) || '',
        result.bestHit?.percentIdentity?.toFixed(1) || '',
        result.hasSignalPeptide ? 'Yes' : 'No',
        result.transmembraneCount || '0',
        getPsosJobUrl(result.psosJobId),
      ].join('\t'))
    }

    const filename = jobFilename.value
      ? `${jobFilename.value.replace(/\.[^.]+$/, '')}_psos_results.tsv`
      : 'psos_results.tsv'

    const blob = new Blob([rows.join('\n')], { type: 'text/tab-separated-values' })
    const url  = URL.createObjectURL(blob)
    const a    = Object.assign(document.createElement('a'), { href: url, download: filename })
    a.click()
    URL.revokeObjectURL(url)
  }

  return {
    // State
    selectedPsosProfile, showPsosPanel, psosAnalyzing,
    psosProgress, psosTotal, psosError, psosResults, psosCopied,
    // Re-exported helpers used in the results table template
    getPsosJobUrl,
    psosProfiles,
    // Actions
    loadExistingResults,
    analyzeWithPsos,
    handleOpenInPsos,
    handleDownloadForPsos,
    downloadPsosResults,
  }
}
