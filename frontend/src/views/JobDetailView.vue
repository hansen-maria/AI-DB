<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter, RouterLink } from 'vue-router'
import { detectSequenceType } from '../api/bakta.ts'
import { downloadOptions, deleteJob, type DownloadFormat } from '../api/jobs.ts'
import { downloadJobResults } from '../api/jobs.ts'
import { psosProfiles } from '../api/psos.ts'

// ── Composables ───────────────────────────────────────────────────────────────
import { useJobPolling }      from '../composables/useJobPolling.ts'
import { useSequenceFilters } from '../composables/useSequenceFilters.ts'
import { usePsosAnalysis }    from '../composables/usePsosAnalysis.ts'
import { useBaktaAnalysis }   from '../composables/useBaktaAnalysis.ts'

// ── Child components ──────────────────────────────────────────────────────────
import SequencesTab from '../components/job/SequencesTab.vue'
import AnalysisTab  from '../components/job/AnalysisTab.vue'

import { statusColors, statusLabels, formatDate } from '../constants/sequences.ts'

// ── Route ─────────────────────────────────────────────────────────────────────
const route  = useRoute()
const router = useRouter()
const jobId  = computed(() => route.params.id as string)

// ── Polling ───────────────────────────────────────────────────────────────────
const { job, allSequences, stats, loading, error, loadJob, stopPolling } = useJobPolling(jobId)

// ── Derived cross-composable data ─────────────────────────────────────────────
const unmatchedSequences   = computed(() => allSequences.value.filter(s => !s.annotation_source))
const detectedSequenceType = computed(() => {
  const seqs = unmatchedSequences.value.filter(s => s.sequence)
  return seqs.length === 0 ? 'protein' : detectSequenceType(seqs.map(s => ({ sequence: s.sequence })))
})

// ── Sequence filters ──────────────────────────────────────────────────────────
const filters = useSequenceFilters(allSequences, job)

// ── Psos ──────────────────────────────────────────────────────────────────────
const psos = usePsosAnalysis(jobId, unmatchedSequences, computed(() => job.value?.filename ?? undefined))

// ── Bakta ─────────────────────────────────────────────────────────────────────
const bakta = useBaktaAnalysis(jobId, unmatchedSequences)

// ── UI state ──────────────────────────────────────────────────────────────────
const TABS = ['overview', 'sequences', 'analysis'] as const
type Tab = typeof TABS[number]

/** Active tab is driven by the URL hash so browser Back/Forward works. */
const activeTab = computed<Tab>(() => {
  const hash = route.hash.replace('#', '') as Tab
  return TABS.includes(hash) ? hash : 'overview'
})

/**
 * Navigate to a tab.
 * @param replace – use router.replace instead of push (no history entry).
 *   Use `true` for programmatic/internal switches, `false` (default) for
 *   explicit user interactions so Back returns to the previous tab.
 */
function setTab(tab: Tab, replace = false) {
  const nav = replace ? router.replace : router.push
  nav({ hash: `#${tab}` })
}
const deleting   = ref(false)
const downloading = ref(false)
const downloadError = ref('')

const progressPercent = computed(() => {
  if (!job.value || job.value.sequence_count === 0) return 0
  return Math.min(100, (job.value.processed_count / job.value.sequence_count) * 100)
})

// ── Lifecycle ─────────────────────────────────────────────────────────────────
onMounted(() => {
  loadJob(async () => {
    await psos.loadExistingResults()
    await bakta.loadExistingState()
  })
})
onUnmounted(() => stopPolling())

// ── Actions ───────────────────────────────────────────────────────────────────
async function handleDelete() {
  if (!confirm('Are you sure you want to delete this job?')) return
  deleting.value = true
  try {
    await deleteJob(jobId.value)
    await router.push({ name: 'jobs' })
  } catch (e) {
    error.value   = e instanceof Error ? e.message : 'Failed to delete job'
    deleting.value = false
  }
}

function handleDownload(format: DownloadFormat) {
  downloading.value  = true
  downloadError.value = ''
  downloadJobResults(jobId.value, format)
      .catch(e => { downloadError.value = e instanceof Error ? e.message : 'Download failed' })
      .finally(() => { downloading.value = false })
}

function startAnnotateFromOverview() {
  bakta.showBaktaPanel.value = true
  bakta.analyzeWithBakta()
}

function openBaktaConfig() {
  setTab('sequences', true)
  bakta.showBaktaPanel.value = true
  setTimeout(() => document.querySelector('.tab-content')?.scrollIntoView({ behavior: 'smooth', block: 'start' }), 50)
}
</script>

<template>
  <div class="job-detail">
    <!-- Loading -->
    <div v-if="loading" class="loading-state">
      <div class="spinner-large"></div>
      <p>Loading job details...</p>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="error-state">
      <h3>Error</h3>
      <p>{{ error }}</p>
      <RouterLink to="/jobs" class="btn btn-secondary">Back to Jobs</RouterLink>
    </div>

    <!-- Job Details -->
    <template v-else-if="job">
      <!-- Header -->
      <div class="job-header">
        <div class="header-left">
          <RouterLink to="/jobs" class="back-link">← Back to Jobs</RouterLink>
          <h2>{{ job.filename || 'Direct Input' }}</h2>
        </div>
        <button class="delete-btn" :disabled="deleting" @click="handleDelete">
          {{ deleting ? 'Deleting…' : 'Delete' }}
        </button>
      </div>

      <!-- Status card -->
      <div class="status-card">
        <div class="status-indicator" :style="{ background: statusColors[job.status] }">
          <div v-if="job.status === 'processing'" class="pulse" :style="{ background: statusColors[job.status] }"></div>
        </div>
        <div class="status-info">
          <span class="status-label">{{ statusLabels[job.status] }}</span>
          <span class="job-id">{{ job.job_id }}</span>
        </div>
        <div v-if="job.status === 'processing'" class="processing-spinner">
          <div class="spinner"></div>
        </div>
      </div>

      <!-- Progress bar -->
      <div v-if="job.status === 'processing'" class="progress-section">
        <div class="progress-bar">
          <div class="progress-fill" :class="{ indeterminate: job.sequence_count === 0 }"
               :style="{ width: job.sequence_count > 0 ? `${progressPercent}%` : '100%' }"></div>
        </div>
        <span class="progress-text" v-if="job.sequence_count > 0">
          {{ job.processed_count.toLocaleString() }} / {{ job.sequence_count.toLocaleString() }}
          ({{ progressPercent.toFixed(1) }}%)
        </span>
        <span class="progress-text" v-else>Counting sequences...</span>
      </div>

      <!-- Tabs -->
      <div v-if="job.status === 'completed'" class="tab-navigation">
        <button class="tab-btn" :class="{ active: activeTab === 'overview'  }" @click="setTab('overview')">Overview</button>
        <button class="tab-btn" :class="{ active: activeTab === 'sequences' }" @click="setTab('sequences')">Sequences</button>
        <button class="tab-btn" :class="{ active: activeTab === 'analysis'  }" @click="setTab('analysis')">Functional Analysis</button>
      </div>

      <!-- Tab content -->
      <div class="tab-content">

        <!-- ── Overview ─────────────────────────────────────────────────── -->
        <div v-if="activeTab === 'overview' || job.status !== 'completed'" class="tab-panel">
          <div class="info-grid">
            <div class="info-item"><span class="info-label">Filename</span>  <span class="info-value">{{ job.filename || 'Direct Input' }}</span></div>
            <div class="info-item"><span class="info-label">Sequences</span> <span class="info-value">{{ job.sequence_count.toLocaleString() }}</span></div>
            <div class="info-item"><span class="info-label">Created</span>   <span class="info-value">{{ formatDate(job.created_at) }}</span></div>
            <div class="info-item"><span class="info-label">Updated</span>   <span class="info-value">{{ formatDate(job.updated_at) }}</span></div>
          </div>

          <div v-if="job.status === 'completed'" class="results-section">
            <h3>Results Summary</h3>
            <div class="results-stats">
              <div class="stat-card stat-hash">
                <span class="stat-value">{{ job.hash_matches.toLocaleString() }}</span>
                <span class="stat-label">Hash Matches</span>
                <span class="stat-percent">{{ Math.round((job.hash_matches / job.sequence_count) * 100) }}%</span>
              </div>
              <div class="stat-card stat-none">
                <span class="stat-value">{{ (job.sequence_count - job.hash_matches).toLocaleString() }}</span>
                <span class="stat-label">No Match</span>
                <span class="stat-percent">{{ Math.round(((job.sequence_count - job.hash_matches) / job.sequence_count) * 100) }}%</span>
              </div>
            </div>

            <!-- Action cards -->
            <div class="action-cards">
              <button class="action-card action-card--sequences" @click="activeTab = 'sequences'">
                <div class="action-card__icon">
                  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M3 15h18M9 3v18"/></svg>
                </div>
                <div class="action-card__body">
                  <span class="action-card__title">Sequences</span>
                  <span class="action-card__desc">Browse, search and filter all sequences. Download as TSV, FASTA or JSON.</span>
                </div>
                <div class="action-card__meta">
                  <span class="action-card__badge action-card__badge--green">{{ job.sequence_count.toLocaleString() }} seqs</span>
                  <svg class="action-card__arrow" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14M12 5l7 7-7 7"/></svg>
                </div>
              </button>

              <button class="action-card action-card--analysis" @click="activeTab = 'analysis'">
                <div class="action-card__icon">
                  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/><path d="M2 20h20"/></svg>
                </div>
                <div class="action-card__body">
                  <span class="action-card__title">Functional Analysis</span>
                  <span class="action-card__desc">Explore COG categories, enzyme classes, top genes and products.</span>
                </div>
                <div class="action-card__meta">
                  <span class="action-card__badge action-card__badge--teal">{{ Math.round((job.hash_matches / job.sequence_count) * 100) }}% annotated</span>
                  <svg class="action-card__arrow" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14M12 5l7 7-7 7"/></svg>
                </div>
              </button>

              <!-- Annotate card -->
              <div class="action-card action-card--annotate"
                   :class="{ 'action-card--disabled': unmatchedSequences.length === 0, 'action-card--analyzing': bakta.baktaAnalyzing.value, 'action-card--done': !!bakta.baktaResult.value && !bakta.baktaAnalyzing.value }">
                <div class="action-card__icon">
                  <svg v-if="!bakta.baktaAnalyzing.value && !bakta.baktaResult.value" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M3 3c9 0 9 18 18 18"/><path d="M21 3C12 3 12 21 3 21"/><path d="M7 8h4"/><path d="M13 16h4"/><path d="M7.5 12H10"/><path d="M14 12h2.5"/></svg>
                  <svg v-else-if="bakta.baktaAnalyzing.value" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" class="spin-icon"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>
                  <svg v-else width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
                </div>
                <div class="action-card__body">
                  <template v-if="!bakta.baktaAnalyzing.value && !bakta.baktaResult.value">
                    <span class="action-card__title">Annotate Unmatched</span>
                    <span class="action-card__desc" v-if="unmatchedSequences.length > 0">Run Bakta on {{ unmatchedSequences.length.toLocaleString() }} sequences with no database match.</span>
                    <span class="action-card__desc" v-else>All sequences are annotated.</span>
                    <div v-if="bakta.baktaError.value" class="annotate-card-error">{{ bakta.baktaError.value }}</div>
                    <div v-if="unmatchedSequences.length > 0" class="annotate-card-actions">
                      <button class="annotate-card-btn" :disabled="bakta.baktaAnalyzing.value" @click.stop="startAnnotateFromOverview">
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polygon points="5 3 19 12 5 21 5 3"/></svg>
                        Start with defaults
                      </button>
                      <button v-if="detectedSequenceType === 'nucleotide'" class="annotate-card-configure" @click.stop="openBaktaConfig">Configure →</button>
                    </div>
                  </template>
                  <template v-else-if="bakta.baktaAnalyzing.value">
                    <span class="action-card__title">Annotating…</span>
                    <span class="action-card__desc annotate-stage">{{ bakta.baktaProgressLabel.value }}</span>
                    <div class="annotate-progress-bar"><div class="annotate-progress-fill" :style="{ width: `${bakta.baktaProgressPercent.value}%` }"></div></div>
                    <div class="annotate-card-actions">
                      <button class="annotate-card-configure" @click.stop="bakta.baktaAbortController.value?.abort()">Cancel</button>
                      <button v-if="detectedSequenceType === 'nucleotide'" class="annotate-card-configure" @click.stop="openBaktaConfig">View details →</button>
                    </div>
                  </template>
                  <template v-else-if="bakta.baktaResult.value">
                    <span class="action-card__title">Annotation complete</span>
                    <span class="action-card__desc">{{ bakta.baktaResult.value.featureCount ?? '?' }} features found.<span v-if="bakta.baktaIngestResult.value"> {{ bakta.baktaIngestResult.value.updated }} sequences updated.</span></span>
                    <div class="annotate-card-actions">
                      <button v-if="detectedSequenceType === 'nucleotide'" class="annotate-card-configure" @click.stop="openBaktaConfig">View results →</button>
                    </div>
                  </template>
                </div>
                <div v-if="!bakta.baktaAnalyzing.value && !bakta.baktaResult.value" class="action-card__meta">
                  <span class="action-card__badge" :class="unmatchedSequences.length > 0 ? 'action-card__badge--amber' : 'action-card__badge--muted'">{{ unmatchedSequences.length.toLocaleString() }} unmatched</span>
                </div>
                <div v-else-if="bakta.baktaAnalyzing.value" class="action-card__meta">
                  <span class="action-card__badge action-card__badge--amber">{{ Math.round(bakta.baktaProgressPercent.value) }}%</span>
                </div>
              </div>
            </div>

            <!-- Download section -->
            <div class="download-section">
              <h4>Download Results</h4>
              <div v-if="downloadError" class="download-error">{{ downloadError }}</div>
              <div class="download-buttons">
                <button v-for="opt in downloadOptions" :key="opt.format" class="download-btn"
                        :disabled="downloading" @click="handleDownload(opt.format)">
                  <span class="download-label">{{ opt.label }}</span>
                  <span class="download-desc">{{ opt.description }}</span>
                </button>
              </div>
              <div v-if="downloading" class="download-progress">
                <div class="spinner"></div> Preparing download...
              </div>
            </div>
          </div>
        </div>

        <!-- ── Sequences Tab ─────────────────────────────────────────────── -->
        <SequencesTab
            v-if="activeTab === 'sequences' && job.status === 'completed'"
            :allSequences="allSequences"
            :filteredSequences="filters.filteredSequences.value"
            :paginatedSequences="filters.paginatedSequences.value"
            :unmatchedSequences="unmatchedSequences"
            :detectedSequenceType="detectedSequenceType"
            :currentFilter="filters.currentFilter.value"
            :searchText="filters.searchText.value"
            :debouncedSearch="filters.debouncedSearch.value"
            :minLength="filters.minLength.value"
            :maxLength="filters.maxLength.value"
            :selectedCog="filters.selectedCog.value"
            :selectedEcClass="filters.selectedEcClass.value"
            :hasGeneOnly="filters.hasGeneOnly.value"
            :hasProductOnly="filters.hasProductOnly.value"
            :showAdvancedFilters="filters.showAdvancedFilters.value"
            :hasActiveFilters="filters.hasActiveFilters.value"
            :activeFilterBadges="filters.activeFilterBadges.value"
            :pagination="filters.pagination.value"
            :pageNumbers="filters.pageNumbers.value"
            :psosShow="psos.showPsosPanel.value"
            :psosProfile="psos.selectedPsosProfile.value"
            :psosProfiles="psosProfiles"
            :psosAnalyzing="psos.psosAnalyzing.value"
            :psosProgress="psos.psosProgress.value"
            :psosTotal="psos.psosTotal.value"
            :psosError="psos.psosError.value"
            :psosResults="psos.psosResults.value"
            :psosCopied="psos.psosCopied.value"
            :baktaShow="bakta.showBaktaPanel.value"
            :baktaAnalyzing="bakta.baktaAnalyzing.value"
            :baktaProgressLabel="bakta.baktaProgressLabel.value"
            :baktaProgressPercent="bakta.baktaProgressPercent.value"
            :baktaError="bakta.baktaError.value"
            :baktaResult="bakta.baktaResult.value"
            :baktaAbortController="bakta.baktaAbortController.value"
            :baktaGenus="bakta.baktaGenus.value"
            :baktaSpecies="bakta.baktaSpecies.value"
            :baktaCompleteGenome="bakta.baktaCompleteGenome.value"
            :baktaIngesting="bakta.baktaIngesting.value"
            :baktaIngestResult="bakta.baktaIngestResult.value"
            :baktaIngestError="bakta.baktaIngestError.value"
            :groupFeaturesByType="bakta.groupFeaturesByType"
            @update:currentFilter="filters.currentFilter.value = $event"
            @update:searchText="filters.searchText.value = $event"
            @update:minLength="filters.minLength.value = $event"
            @update:maxLength="filters.maxLength.value = $event"
            @update:selectedCog="filters.selectedCog.value = $event"
            @update:selectedEcClass="filters.selectedEcClass.value = $event"
            @update:hasGeneOnly="filters.hasGeneOnly.value = $event"
            @update:hasProductOnly="filters.hasProductOnly.value = $event"
            @update:showAdvancedFilters="filters.showAdvancedFilters.value = $event"
            @clear-filters="filters.clearFilters()"
            @download-filtered="filters.downloadFilteredSequences($event)"
            @go-to-page="filters.goToPage($event)"
            @update:psosShow="psos.showPsosPanel.value = $event"
            @update:psosProfile="psos.selectedPsosProfile.value = $event"
            @psos-analyze="psos.analyzeWithPsos()"
            @psos-open="psos.handleOpenInPsos()"
            @psos-download-fasta="psos.handleDownloadForPsos()"
            @psos-download-tsv="psos.downloadPsosResults()"
            @update:baktaShow="bakta.showBaktaPanel.value = $event"
            @update:baktaGenus="bakta.baktaGenus.value = $event"
            @update:baktaSpecies="bakta.baktaSpecies.value = $event"
            @update:baktaCompleteGenome="bakta.baktaCompleteGenome.value = $event"
            @bakta-analyze="bakta.analyzeWithBakta()"
            @bakta-ingest="bakta.ingestBaktaAnnotations()"
            @bakta-reset="bakta.resetBakta()"
        />

        <!-- ── Analysis Tab ─────────────────────────────────────────────── -->
        <AnalysisTab
            v-if="activeTab === 'analysis' && job.status === 'completed'"
            :loading="false"
            :stats="stats"
        />
      </div>

      <!-- Failed state -->
      <div v-if="job.status === 'failed'" class="error-section">
        <h3>Error</h3>
        <p>{{ job.error_message || 'An unknown error occurred.' }}</p>
      </div>

      <div class="actions">
        <RouterLink to="/submit" class="btn btn-primary">Submit New Job</RouterLink>
      </div>
    </template>
  </div>
</template>

<style scoped>
/* Only layout / chrome styles live here — component-specific styles are in each child */
.job-detail { max-width: 1200px; margin: 0 auto; }
.loading-state, .error-state { text-align: center; padding: 4rem 2rem; }
.spinner-large { width: 48px; height: 48px; border: 3px solid var(--color-border); border-top-color: hsla(160,100%,37%,1); border-radius: 50%; animation: spin 0.8s linear infinite; margin: 0 auto 1rem; }
.spinner { width: 24px; height: 24px; border: 2px solid var(--color-border); border-top-color: hsla(160,100%,37%,1); border-radius: 50%; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.job-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1.5rem; }
.header-left { display: flex; flex-direction: column; gap: 0.5rem; }
.back-link { color: var(--color-text); text-decoration: none; font-size: 0.9rem; opacity: 0.8; }
.back-link:hover { opacity: 1; }
.job-header h2 { margin: 0; font-size: 1.5rem; color: var(--color-heading); }
.delete-btn { padding: 0.5rem 1rem; border: 1px solid #f44336; border-radius: 6px; background: transparent; color: #f44336; cursor: pointer; transition: all 0.2s; }
.delete-btn:hover:not(:disabled) { background: #f44336; color: white; }
.delete-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.status-card { display: flex; align-items: center; gap: 1rem; padding: 1rem 1.5rem; background: var(--color-background-soft); border: 1px solid var(--color-border); border-radius: 12px; margin-bottom: 1.5rem; }
.status-indicator { position: relative; width: 12px; height: 12px; border-radius: 50%; }
.pulse { position: absolute; inset: -4px; border-radius: 50%; background: inherit; animation: pulse 1.5s ease-out infinite; }
@keyframes pulse { 0%{opacity:0.8;transform:scale(1)} 100%{opacity:0;transform:scale(2)} }
.status-info { flex: 1; }
.status-label { display: block; font-weight: 600; color: var(--color-heading); }
.job-id { font-size: 0.85rem; color: var(--color-text); opacity: 0.7; font-family: monospace; }
.processing-spinner .spinner { border-top-color: #2196f3; }
.progress-section { margin-bottom: 1.5rem; }
.progress-bar { height: 8px; background: var(--color-background-mute); border-radius: 4px; overflow: hidden; margin-bottom: 0.5rem; }
.progress-fill { height: 100%; background: linear-gradient(90deg, hsla(160,100%,37%,1), hsla(160,100%,47%,1)); border-radius: 4px; transition: width 0.3s; }
.progress-fill.indeterminate { animation: indeterminate 1.5s infinite linear; background: linear-gradient(90deg, transparent, hsla(160,100%,37%,1), transparent); }
@keyframes indeterminate { 0%{transform:translateX(-100%)} 100%{transform:translateX(100%)} }
.progress-text { font-size: 0.85rem; color: var(--color-text); opacity: 0.8; }
.tab-navigation { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; border-bottom: 1px solid var(--color-border); }
.tab-btn { padding: 0.75rem 1.25rem; border: none; background: transparent; color: var(--color-text); font-size: 0.95rem; cursor: pointer; border-bottom: 2px solid transparent; margin-bottom: -1px; transition: all 0.2s; }
.tab-btn:hover { color: hsla(160,100%,37%,1); }
.tab-btn.active { color: hsla(160,100%,37%,1); border-bottom-color: hsla(160,100%,37%,1); }
.tab-content { min-height: 300px; }
.tab-panel { animation: fadeIn 0.2s ease; }
@keyframes fadeIn { from{opacity:0} to{opacity:1} }
.info-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1rem; margin-bottom: 2rem; }
.info-item { padding: 1rem; background: var(--color-background-soft); border-radius: 8px; }
.info-label { display: block; font-size: 0.8rem; color: var(--color-text); opacity: 0.7; margin-bottom: 0.25rem; }
.info-value { font-weight: 600; color: var(--color-heading); word-break: break-word; }
.results-section h3 { margin: 0 0 1rem; color: var(--color-heading); font-size: 1.2rem; }
.results-stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 1rem; margin-bottom: 2rem; }
.stat-card { padding: 1.25rem; border-radius: 12px; text-align: center; }
.stat-card.stat-hash { background: rgba(76,175,80,0.1); border: 1px solid rgba(76,175,80,0.2); }
.stat-card.stat-none { background: rgba(158,158,158,0.1); border: 1px solid rgba(158,158,158,0.2); }
.stat-value { display: block; font-size: 2rem; font-weight: 700; color: var(--color-heading); }
.stat-label { display: block; font-size: 0.85rem; color: var(--color-text); opacity: 0.8; margin-top: 0.25rem; }
.stat-percent { display: block; font-size: 0.9rem; font-weight: 600; margin-top: 0.5rem; }
.stat-hash .stat-percent { color: #4caf50; }
.stat-none .stat-percent { color: #9e9e9e; }
.download-section { padding: 1.5rem; background: var(--color-background-soft); border-radius: 12px; border: 1px solid var(--color-border); }
.download-section h4 { margin: 0 0 1rem; color: var(--color-heading); }
.download-error { background: rgba(244,67,54,0.1); border: 1px solid rgba(244,67,54,0.3); color: #f44336; padding: 0.75rem; border-radius: 8px; margin-bottom: 1rem; }
.download-buttons { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 1rem; }
.download-btn { display: flex; flex-direction: column; padding: 1rem; background: var(--color-background); border: 1px solid var(--color-border); border-radius: 8px; cursor: pointer; transition: all 0.2s; }
.download-btn:hover:not(:disabled) { border-color: hsla(160,100%,37%,0.5); transform: translateY(-2px); }
.download-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.download-label { font-weight: 600; color: var(--color-heading); }
.download-desc { font-size: 0.75rem; color: var(--color-text); opacity: 0.7; }
.download-progress { display: flex; align-items: center; gap: 0.75rem; margin-top: 1rem; color: var(--color-text); }
/* Action cards */
.action-cards { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.85rem; margin-bottom: 2rem; }
.action-card { display: flex; align-items: center; gap: 0.9rem; padding: 1rem 1.1rem; background: var(--color-background); border: 1px solid var(--color-border); border-radius: 10px; cursor: pointer; text-align: left; transition: transform 0.15s, box-shadow 0.15s, border-color 0.15s; position: relative; overflow: hidden; width: 100%; }
.action-card::before { content:''; position:absolute; left:0;top:0;bottom:0; width:3px; border-radius:10px 0 0 10px; transition:width 0.15s; }
.action-card--sequences::before { background: hsla(160,100%,37%,1); }
.action-card--analysis::before  { background: #028090; }
.action-card--annotate::before  { background: #e08000; }
.action-card:hover:not(:disabled) { transform:translateY(-2px); box-shadow:0 4px 16px rgba(0,0,0,0.09); }
.action-card--sequences:hover:not(:disabled) { border-color: hsla(160,100%,37%,0.4); }
.action-card--analysis:hover:not(:disabled)  { border-color: rgba(2,128,144,0.4); }
.action-card--annotate:hover:not(:disabled)  { border-color: rgba(224,128,0,0.4); }
.action-card--disabled { cursor:default; opacity:0.6; }
.action-card--disabled::before { background: var(--color-border); }
.action-card--analyzing { border-color: rgba(224,128,0,0.35); background: rgba(224,128,0,0.03); }
.action-card--done { border-color: rgba(0,189,126,0.35); background: rgba(0,189,126,0.03); }
.action-card--done .action-card__icon { color: hsla(160,100%,37%,1); }
.action-card--done::before { background: hsla(160,100%,37%,1); }
.action-card__icon { flex-shrink:0; display:flex; align-items:center; justify-content:center; width:38px; height:38px; border-radius:8px; background:var(--color-background-soft); }
.action-card--sequences .action-card__icon { color: hsla(160,100%,37%,1); }
.action-card--analysis  .action-card__icon { color: #028090; }
.action-card--annotate  .action-card__icon { color: #e08000; }
.action-card--disabled  .action-card__icon { color: var(--color-text); opacity:0.4; }
.action-card__body { flex:1; display:flex; flex-direction:column; gap:0.2rem; min-width:0; }
.action-card__title { font-size:0.875rem; font-weight:650; color:var(--color-heading); white-space:nowrap; }
.action-card__desc  { font-size:0.73rem;  color:var(--color-text); opacity:0.7; line-height:1.4; }
.action-card__meta  { flex-shrink:0; display:flex; flex-direction:column; align-items:flex-end; gap:0.4rem; }
.action-card__badge { font-size:0.68rem; font-weight:600; padding:0.15rem 0.5rem; border-radius:99px; white-space:nowrap; }
.action-card__badge--green { background:hsla(160,100%,37%,0.12); color:hsla(160,100%,30%,1); }
.action-card__badge--teal  { background:rgba(2,128,144,0.12);    color:#016070; }
.action-card__badge--amber { background:rgba(224,128,0,0.13);    color:#a05a00; }
.action-card__badge--muted { background:var(--color-background-soft); color:var(--color-text); opacity:0.55; }
.action-card__arrow { color:var(--color-text); opacity:0.3; transition:opacity 0.15s,transform 0.15s; }
.action-card:hover:not(:disabled) .action-card__arrow { opacity:0.7; transform:translateX(2px); }
.annotate-card-actions { display:flex; align-items:center; gap:0.6rem; margin-top:0.45rem; flex-wrap:wrap; }
.annotate-card-btn { display:inline-flex; align-items:center; gap:0.3rem; padding:0.28rem 0.7rem; background:#e08000; color:#fff; border:none; border-radius:5px; font-size:0.73rem; font-weight:600; cursor:pointer; transition:background 0.13s,transform 0.1s; }
.annotate-card-btn:hover:not(:disabled) { background:#c07000; transform:translateY(-1px); }
.annotate-card-btn:disabled { opacity:0.5; cursor:not-allowed; }
.annotate-card-configure { background:none; border:none; padding:0; font-size:0.72rem; color:var(--color-text); opacity:0.55; cursor:pointer; transition:opacity 0.13s; white-space:nowrap; }
.annotate-card-configure:hover { opacity:1; }
.annotate-progress-bar { height:4px; background:var(--color-background-mute); border-radius:99px; overflow:hidden; margin-top:0.35rem; width:100%; }
.annotate-progress-fill { height:100%; background:#e08000; border-radius:99px; transition:width 0.4s ease; }
.annotate-stage { font-style:italic; }
.annotate-card-error { margin-top:0.3rem; font-size:0.7rem; color:#f44336; line-height:1.3; }
.spin-icon { animation:spin 1s linear infinite; }
.error-section { background: rgba(244,67,54,0.08); border: 1px solid rgba(244,67,54,0.25); border-radius: 8px; padding: 1.25rem; }
.error-section h3 { margin: 0 0 0.5rem; color: #f44336; }
.actions { margin-top: 2rem; padding-top: 1.5rem; border-top: 1px solid var(--color-border); }
.btn-primary { display:inline-flex; align-items:center; gap:0.5rem; padding:0.75rem 1.5rem; background:hsla(160,100%,37%,1); color:white; border:none; border-radius:8px; font-size:0.95rem; font-weight:500; text-decoration:none; cursor:pointer; transition:background 0.2s; }
.btn-primary:hover { background:hsla(160,100%,30%,1); }
.btn-secondary { display:inline-flex; align-items:center; gap:0.5rem; padding:0.75rem 1.5rem; background:transparent; color:var(--color-text); border:1px solid var(--color-border); border-radius:8px; font-size:0.95rem; text-decoration:none; cursor:pointer; transition:all 0.2s; }
.btn-secondary:hover { border-color:hsla(160,100%,37%,0.5); }
@media (max-width:860px) { .action-cards { grid-template-columns:1fr; } .action-card__desc { display:none; } }
@media (max-width:1100px) and (min-width:861px) { .action-cards { grid-template-columns:1fr 1fr; } }
@media (max-width:600px) { .job-header { flex-direction:column; gap:1rem; } .info-grid { grid-template-columns:1fr; } .results-stats { grid-template-columns:1fr; } .tab-btn { flex:1; padding:0.6rem 0.5rem; font-size:0.85rem; } }
</style>
