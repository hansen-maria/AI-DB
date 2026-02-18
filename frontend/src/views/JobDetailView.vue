<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRoute, useRouter, RouterLink } from 'vue-router'
import {
  getJob, deleteJob, downloadJobResults, downloadOptions, getJobStats,
  type PaginatedJobResponse, type JobStatus, type PaginationInfo,
  type SequenceFilter, type DownloadFormat, type FunctionalStats
} from '../api/jobs.ts'

const route = useRoute()
const router = useRouter()

// Job data
const job = ref<PaginatedJobResponse | null>(null)
const stats = ref<FunctionalStats | null>(null)
const pagination = ref<PaginationInfo | null>(null)

// UI state
const loading = ref(true)
const loadingSequences = ref(false)
const loadingStats = ref(false)
const error = ref('')
const deleting = ref(false)
const downloading = ref(false)
const downloadError = ref('')
const currentPage = ref(1)
const currentFilter = ref<SequenceFilter>('all')
const perPage = 20

// Tab state
const activeTab = ref<'overview' | 'sequences' | 'analysis'>('overview')

let pollInterval: number | null = null

const jobId = computed(() => route.params.id as string)

// Progress percentage
const progressPercent = computed(() => {
  if (!job.value || job.value.sequence_count === 0) return 0
  return Math.min(100, (job.value.processed_count / job.value.sequence_count) * 100)
})

const statusColors: Record<JobStatus, string> = {
  pending: '#ff9800',
  processing: '#2196f3',
  completed: '#4caf50',
  failed: '#f44336'
}

const statusLabels: Record<JobStatus, string> = {
  pending: 'Pending',
  processing: 'Processing',
  completed: 'Completed',
  failed: 'Failed'
}

const filterOptions: { value: SequenceFilter; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'hash_match', label: 'Matches' },
  { value: 'none', label: 'No Match' },
]

const pageNumbers = computed(() => {
  if (!pagination.value) return []
  const total = pagination.value.total_pages
  const current = pagination.value.page
  const pages: number[] = []
  let start = Math.max(1, current - 2)
  let end = Math.min(total, current + 2)
  if (current <= 3) end = Math.min(5, total)
  if (current >= total - 2) start = Math.max(1, total - 4)
  for (let i = start; i <= end; i++) pages.push(i)
  return pages
})

const annotationRate = computed(() => {
  if (!stats.value || stats.value.total_sequences === 0) return 0
  return Math.round((stats.value.annotated_sequences / stats.value.total_sequences) * 100)
})

async function loadJob(page = 1, filter: SequenceFilter = currentFilter.value) {
  if (page !== 1 || filter !== currentFilter.value) loadingSequences.value = true
  else loading.value = true
  currentPage.value = page
  currentFilter.value = filter

  try {
    const response = await getJob(jobId.value, page, perPage, filter)
    job.value = response
    pagination.value = response.pagination

    if (response.status === 'pending' || response.status === 'processing') {
      startPolling()
    } else if (response.status === 'completed' && !stats.value) {
      loadStats()
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load job'
  } finally {
    loading.value = false
    loadingSequences.value = false
  }
}

async function loadStats() {
  if (!job.value || job.value.status !== 'completed') return
  loadingStats.value = true
  try {
    stats.value = await getJobStats(jobId.value)
  } catch (e) {
    console.error('Failed to load stats:', e)
  } finally {
    loadingStats.value = false
  }
}

function goToPage(page: number) {
  if (page >= 1 && (!pagination.value || page <= pagination.value.total_pages)) {
    loadJob(page, currentFilter.value)
  }
}

function setFilter(filter: SequenceFilter) {
  if (filter !== currentFilter.value) loadJob(1, filter)
}

function startPolling() {
  if (pollInterval) return
  pollInterval = window.setInterval(async () => {
    try {
      const response = await getJob(jobId.value, currentPage.value, perPage, currentFilter.value)
      job.value = response
      pagination.value = response.pagination
      if (response.status === 'completed' || response.status === 'failed') {
        stopPolling()
        if (response.status === 'completed') loadStats()
      }
    } catch (e) {
      stopPolling()
    }
  }, 1000)
}

function stopPolling() {
  if (pollInterval) {
    clearInterval(pollInterval)
    pollInterval = null
  }
}

async function handleDelete() {
  if (!confirm('Are you sure you want to delete this job?')) return
  deleting.value = true
  try {
    await deleteJob(jobId.value)
    await router.push({name: 'jobs'})
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to delete job'
    deleting.value = false
  }
}

async function handleDownload(format: DownloadFormat) {
  downloading.value = true
  downloadError.value = ''
  try {
    await downloadJobResults(jobId.value, format)
  } catch (e) {
    downloadError.value = e instanceof Error ? e.message : 'Download failed'
  } finally {
    downloading.value = false
  }
}

function formatDate(dateStr: string) {
  return new Date(dateStr).toLocaleString()
}

function getUniRef100Url(id: string): string {
  return `https://www.uniprot.org/uniref/UniRef100_${id}`
}

function getUniParcUrl(id: string): string {
  return `https://www.uniprot.org/uniparc/${id}`
}

function getNcbiUrl(id: string): string {
  return `https://www.ncbi.nlm.nih.gov/protein/${id}`
}

function hasAnnotationLinks(seq: { uniparc_id?: string | null; ncbi_nrp_id?: string | null; uniref100_id?: string | null }): boolean {
  return !!(seq.uniparc_id || seq.ncbi_nrp_id || seq.uniref100_id)
}

const chartColors = [
  '#00bd7e', '#2196f3', '#ff9800', '#9c27b0', '#f44336',
  '#00bcd4', '#8bc34a', '#ffeb3b', '#795548', '#607d8b',
  '#e91e63', '#3f51b5', '#009688', '#ff5722', '#cddc39'
]

onMounted(() => loadJob())
onUnmounted(stopPolling)
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
          <h2>{{ job.filename || 'Job Details' }}</h2>
        </div>
        <button @click="handleDelete" :disabled="deleting" class="delete-btn">
          {{ deleting ? 'Deleting...' : 'Delete' }}
        </button>
      </div>

      <!-- Status -->
      <div class="status-card">
        <div class="status-indicator" :style="{ backgroundColor: statusColors[job.status] }">
          <span v-if="job.status === 'processing'" class="pulse"></span>
        </div>
        <div class="status-info">
          <span class="status-label">{{ statusLabels[job.status] }}</span>
          <span class="job-id">{{ job.job_id }}</span>
        </div>
        <div v-if="job.status === 'processing'" class="processing-spinner">
          <div class="spinner"></div>
        </div>
      </div>

      <!-- Progress -->
      <div v-if="job.status === 'processing'" class="progress-section">
        <div class="progress-bar">
          <div class="progress-fill" :class="{ 'indeterminate': job.sequence_count === 0 }"
               :style="{ width: job.sequence_count > 0 ? `${progressPercent}%` : '100%' }"></div>
        </div>
        <span class="progress-text" v-if="job.sequence_count > 0">
          {{ job.processed_count.toLocaleString() }} / {{ job.sequence_count.toLocaleString() }} ({{ progressPercent.toFixed(1) }}%)
        </span>
        <span class="progress-text" v-else>Counting sequences...</span>
      </div>

      <!-- Tabs -->
      <div v-if="job.status === 'completed'" class="tab-navigation">
        <button class="tab-btn" :class="{ active: activeTab === 'overview' }" @click="activeTab = 'overview'">
          Overview
        </button>
        <button class="tab-btn" :class="{ active: activeTab === 'sequences' }" @click="activeTab = 'sequences'">
          Sequences
        </button>
        <button class="tab-btn" :class="{ active: activeTab === 'analysis' }" @click="activeTab = 'analysis'">
          Functional Analysis
        </button>
      </div>

      <!-- Tab Content -->
      <div class="tab-content">
        <!-- Overview Tab -->
        <div v-if="activeTab === 'overview' || job.status !== 'completed'" class="tab-panel">
          <div class="info-grid">
            <div class="info-item">
              <span class="info-label">Filename</span>
              <span class="info-value">{{ job.filename || 'Direct Input' }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Sequences</span>
              <span class="info-value">{{ job.sequence_count.toLocaleString() }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Created</span>
              <span class="info-value">{{ formatDate(job.created_at) }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Updated</span>
              <span class="info-value">{{ formatDate(job.updated_at) }}</span>
            </div>
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

        <!-- Sequences Tab -->
        <div v-if="activeTab === 'sequences' && job.status === 'completed'" class="tab-panel">
          <div class="sequences-section">
            <div class="sequences-header">
              <h4>Sequence Details</h4>
              <div class="filter-controls">
                <div class="filter-buttons">
                  <button v-for="opt in filterOptions" :key="opt.value" class="filter-btn"
                          :class="{ active: currentFilter === opt.value }" :disabled="loadingSequences"
                          @click="setFilter(opt.value)">
                    {{ opt.label }}
                  </button>
                </div>
              </div>
            </div>

            <div v-if="job.filtered_count !== job.sequence_count" class="filtered-info">
              Showing {{ job.filtered_count }} of {{ job.sequence_count }} sequences
            </div>

            <div v-if="job.sequences && job.sequences.length > 0" class="sequences-table">
              <div class="table-wrapper">
                <table>
                  <thead>
                  <tr>
                    <th>ID</th>
                    <th>Length</th>
                    <th>Gene</th>
                    <th>Function / Product</th>
                    <th>Links</th>
                  </tr>
                  </thead>
                  <tbody>
                  <tr v-for="seq in job.sequences" :key="seq.id" :class="{ 'has-match': hasAnnotationLinks(seq) }">
                    <td class="seq-id">{{ seq.id }}</td>
                    <td class="seq-length">{{ seq.length.toLocaleString() }}</td>
                    <td class="seq-gene">
                      <span v-if="seq.gene" class="gene-name">{{ seq.gene }}</span>
                      <span v-else class="no-data">-</span>
                    </td>
                    <td class="seq-product">
                      <span v-if="seq.product" class="product-desc">{{ seq.product }}</span>
                      <span v-else class="no-data">-</span>
                    </td>
                    <td class="annotation-cell">
                      <template v-if="hasAnnotationLinks(seq)">
                        <div class="annotation-links">
                          <a v-if="seq.uniref100_id" :href="getUniRef100Url(seq.uniref100_id)" target="_blank" class="db-link uniref">UniRef</a>
                          <a v-if="seq.uniparc_id" :href="getUniParcUrl(seq.uniparc_id)" target="_blank" class="db-link uniparc">UniParc</a>
                          <a v-if="seq.ncbi_nrp_id" :href="getNcbiUrl(seq.ncbi_nrp_id)" target="_blank" class="db-link ncbi">NCBI</a>
                        </div>
                      </template>
                      <span v-else class="no-data">-</span>
                    </td>
                  </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <div v-else-if="job.filtered_count === 0" class="empty-filter-results">
              <p>No sequences match the current filter.</p>
              <button class="btn btn-secondary" @click="setFilter('all')">Show All</button>
            </div>

            <div v-if="loadingSequences" class="sequences-loading">
              <div class="spinner"></div> Loading...
            </div>

            <div v-if="pagination && pagination.total_pages > 1" class="sequences-pagination">
              <button class="page-btn" :disabled="!pagination.has_prev || loadingSequences" @click="goToPage(pagination.page - 1)">←</button>
              <button v-if="pageNumbers[0] > 1" class="page-btn" :disabled="loadingSequences" @click="goToPage(1)">1</button>
              <span v-if="pageNumbers[0] > 2" class="page-ellipsis">...</span>
              <button v-for="page in pageNumbers" :key="page" class="page-btn" :class="{ active: page === pagination.page }"
                      :disabled="loadingSequences" @click="goToPage(page)">{{ page }}</button>
              <span v-if="pageNumbers[pageNumbers.length - 1] < pagination.total_pages - 1" class="page-ellipsis">...</span>
              <button v-if="pageNumbers[pageNumbers.length - 1] < pagination.total_pages" class="page-btn"
                      :disabled="loadingSequences" @click="goToPage(pagination.total_pages)">{{ pagination.total_pages }}</button>
              <button class="page-btn" :disabled="!pagination.has_next || loadingSequences" @click="goToPage(pagination.page + 1)">→</button>
              <span class="page-info">Page {{ pagination.page }} of {{ pagination.total_pages }}</span>
            </div>
          </div>
        </div>

        <!-- Functional Analysis Tab -->
        <div v-if="activeTab === 'analysis' && job.status === 'completed'" class="tab-panel">
          <div v-if="loadingStats" class="loading-stats">
            <div class="spinner"></div> Loading functional analysis...
          </div>

          <div v-else-if="stats" class="analysis-section">
            <!-- Annotation Rate -->
            <div class="annotation-overview">
              <div class="annotation-rate">
                <div class="rate-circle">
                  <svg viewBox="0 0 36 36" class="circular-chart">
                    <path class="circle-bg" d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"/>
                    <path class="circle" :stroke-dasharray="`${annotationRate}, 100`"
                          d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"/>
                  </svg>
                  <span class="rate-value">{{ annotationRate }}%</span>
                </div>
                <div class="rate-info">
                  <span class="rate-label">Annotation Rate</span>
                  <span class="rate-detail">{{ stats.annotated_sequences.toLocaleString() }} of {{ stats.total_sequences.toLocaleString() }} sequences</span>
                </div>
              </div>
            </div>

            <!-- Charts -->
            <div class="charts-grid">
              <!-- Top Genes -->
              <div class="chart-card">
                <h4>Top Genes</h4>
                <div v-if="stats.top_genes.length > 0" class="horizontal-bars">
                  <div v-for="(item, index) in stats.top_genes.slice(0, 12)" :key="item.name" class="bar-item">
                    <span class="bar-label">{{ item.name }}</span>
                    <div class="bar-wrapper">
                      <div class="bar-fill" :style="{ width: `${(item.count / stats.top_genes[0].count) * 100}%`, backgroundColor: chartColors[index % chartColors.length] }"></div>
                    </div>
                    <span class="bar-value">{{ item.count }}</span>
                  </div>
                </div>
                <div v-else class="no-chart-data">No gene annotations found</div>
              </div>

              <!-- Top Products -->
              <div class="chart-card">
                <h4>Top Functions / Products</h4>
                <div v-if="stats.top_products.length > 0" class="horizontal-bars">
                  <div v-for="(item, index) in stats.top_products.slice(0, 12)" :key="item.name" class="bar-item">
                    <span class="bar-label" :title="item.name">{{ item.name }}</span>
                    <div class="bar-wrapper">
                      <div class="bar-fill" :style="{ width: `${(item.count / stats.top_products[0].count) * 100}%`, backgroundColor: chartColors[index % chartColors.length] }"></div>
                    </div>
                    <span class="bar-value">{{ item.count }}</span>
                  </div>
                </div>
                <div v-else class="no-chart-data">No product annotations found</div>
              </div>

              <!-- COG Categories -->
              <div class="chart-card">
                <h4>COG Functional Categories</h4>
                <div v-if="stats.cog_categories.length > 0" class="horizontal-bars">
                  <div v-for="(item, index) in stats.cog_categories" :key="item.code" class="bar-item">
                    <span class="bar-label"><span class="cog-code">{{ item.code }}</span> {{ item.name }}</span>
                    <div class="bar-wrapper">
                      <div class="bar-fill" :style="{ width: `${(item.count / stats.cog_categories[0].count) * 100}%`, backgroundColor: chartColors[index % chartColors.length] }"></div>
                    </div>
                    <span class="bar-value">{{ item.count }}</span>
                  </div>
                </div>
                <div v-else class="no-chart-data">No COG categories found</div>
              </div>

              <!-- EC Classes -->
              <div class="chart-card">
                <h4>Enzyme Classes (EC)</h4>
                <div v-if="stats.ec_classes.length > 0" class="horizontal-bars">
                  <div v-for="(item, index) in stats.ec_classes" :key="item.name" class="bar-item">
                    <span class="bar-label">{{ item.name }}</span>
                    <div class="bar-wrapper">
                      <div class="bar-fill" :style="{ width: `${(item.count / stats.ec_classes[0].count) * 100}%`, backgroundColor: chartColors[index % chartColors.length] }"></div>
                    </div>
                    <span class="bar-value">{{ item.count }}</span>
                  </div>
                </div>
                <div v-else class="no-chart-data">No enzyme classifications found</div>
              </div>

              <!-- GO Terms -->
              <div v-if="stats.go_terms.molecular_function.length > 0" class="chart-card chart-card-wide">
                <h4>Gene Ontology (GO) Terms</h4>
                <div class="go-items">
                  <div v-for="item in stats.go_terms.molecular_function.slice(0, 15)" :key="item.name" class="go-item">
                    <span class="go-id">{{ item.name }}</span>
                    <span class="go-count">{{ item.count }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Error -->
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
.job-detail { max-width: 1200px; margin: 0 auto; }

.loading-state, .error-state { text-align: center; padding: 4rem 2rem; }

.spinner-large {
  width: 48px; height: 48px;
  border: 3px solid var(--color-border);
  border-top-color: hsla(160, 100%, 37%, 1);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin: 0 auto 1rem;
}

.spinner {
  width: 24px; height: 24px;
  border: 2px solid var(--color-border);
  border-top-color: hsla(160, 100%, 37%, 1);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.job-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1.5rem; }
.header-left { display: flex; flex-direction: column; gap: 0.5rem; }
.back-link { color: var(--color-text); text-decoration: none; font-size: 0.9rem; opacity: 0.8; }
.back-link:hover { opacity: 1; }
.job-header h2 { margin: 0; font-size: 1.5rem; color: var(--color-heading); }

.delete-btn {
  padding: 0.5rem 1rem; border: 1px solid #f44336; border-radius: 6px;
  background: transparent; color: #f44336; cursor: pointer; transition: all 0.2s;
}
.delete-btn:hover:not(:disabled) { background: #f44336; color: white; }
.delete-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.status-card {
  display: flex; align-items: center; gap: 1rem; padding: 1rem 1.5rem;
  background: var(--color-background-soft); border: 1px solid var(--color-border);
  border-radius: 12px; margin-bottom: 1.5rem;
}

.status-indicator { position: relative; width: 12px; height: 12px; border-radius: 50%; }
.pulse { position: absolute; inset: -4px; border-radius: 50%; background: inherit; animation: pulse 1.5s ease-out infinite; }
@keyframes pulse { 0% { opacity: 0.8; transform: scale(1); } 100% { opacity: 0; transform: scale(2); } }

.status-info { flex: 1; }
.status-label { display: block; font-weight: 600; color: var(--color-heading); }
.job-id { font-size: 0.85rem; color: var(--color-text); opacity: 0.7; font-family: monospace; }
.processing-spinner .spinner { border-top-color: #2196f3; }

.progress-section { margin-bottom: 1.5rem; }
.progress-bar { height: 8px; background: var(--color-background-mute); border-radius: 4px; overflow: hidden; margin-bottom: 0.5rem; }
.progress-fill { height: 100%; background: linear-gradient(90deg, hsla(160, 100%, 37%, 1), hsla(160, 100%, 47%, 1)); border-radius: 4px; transition: width 0.3s; }
.progress-fill.indeterminate { animation: indeterminate 1.5s infinite linear; background: linear-gradient(90deg, transparent, hsla(160, 100%, 37%, 1), transparent); }
@keyframes indeterminate { 0% { transform: translateX(-100%); } 100% { transform: translateX(100%); } }
.progress-text { font-size: 0.85rem; color: var(--color-text); opacity: 0.8; }

.tab-navigation { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; border-bottom: 1px solid var(--color-border); }
.tab-btn {
  padding: 0.75rem 1.25rem; border: none; background: transparent; color: var(--color-text);
  font-size: 0.95rem; cursor: pointer; border-bottom: 2px solid transparent; margin-bottom: -1px; transition: all 0.2s;
}
.tab-btn:hover { color: hsla(160, 100%, 37%, 1); }
.tab-btn.active { color: hsla(160, 100%, 37%, 1); border-bottom-color: hsla(160, 100%, 37%, 1); }

.tab-content { min-height: 300px; }
.tab-panel { animation: fadeIn 0.2s ease; }
@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

.info-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1rem; margin-bottom: 2rem; }
.info-item { padding: 1rem; background: var(--color-background-soft); border-radius: 8px; }
.info-label { display: block; font-size: 0.8rem; color: var(--color-text); opacity: 0.7; margin-bottom: 0.25rem; }
.info-value { font-weight: 600; color: var(--color-heading); word-break: break-word; }

.results-section h3 { margin: 0 0 1rem 0; color: var(--color-heading); font-size: 1.2rem; }
.results-stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 1rem; margin-bottom: 2rem; }
.stat-card { padding: 1.25rem; border-radius: 12px; text-align: center; }
.stat-card.stat-hash { background: rgba(76, 175, 80, 0.1); border: 1px solid rgba(76, 175, 80, 0.2); }
.stat-card.stat-none { background: rgba(158, 158, 158, 0.1); border: 1px solid rgba(158, 158, 158, 0.2); }
.stat-value { display: block; font-size: 2rem; font-weight: 700; color: var(--color-heading); }
.stat-label { display: block; font-size: 0.85rem; color: var(--color-text); opacity: 0.8; margin-top: 0.25rem; }
.stat-percent { display: block; font-size: 0.9rem; font-weight: 600; margin-top: 0.5rem; }
.stat-hash .stat-percent { color: #4caf50; }
.stat-none .stat-percent { color: #9e9e9e; }

.download-section { padding: 1.5rem; background: var(--color-background-soft); border-radius: 12px; border: 1px solid var(--color-border); }
.download-section h4 { margin: 0 0 1rem 0; color: var(--color-heading); }
.download-error { background: rgba(244, 67, 54, 0.1); border: 1px solid rgba(244, 67, 54, 0.3); color: #f44336; padding: 0.75rem; border-radius: 8px; margin-bottom: 1rem; }
.download-buttons { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 1rem; }
.download-btn { display: flex; flex-direction: column; padding: 1rem; background: var(--color-background); border: 1px solid var(--color-border); border-radius: 8px; cursor: pointer; transition: all 0.2s; }
.download-btn:hover:not(:disabled) { border-color: hsla(160, 100%, 37%, 0.5); transform: translateY(-2px); }
.download-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.download-label { font-weight: 600; color: var(--color-heading); }
.download-desc { font-size: 0.75rem; color: var(--color-text); opacity: 0.7; }
.download-progress { display: flex; align-items: center; gap: 0.75rem; margin-top: 1rem; color: var(--color-text); }

.sequences-section { margin-top: 1rem; }
.sequences-header { display: flex; justify-content: space-between; align-items: center; gap: 1rem; margin-bottom: 1rem; flex-wrap: wrap; }
.sequences-header h4 { margin: 0; color: var(--color-heading); }
.filter-buttons { display: flex; gap: 0.5rem; }
.filter-btn { padding: 0.4rem 0.75rem; font-size: 0.85rem; border: 1px solid var(--color-border); border-radius: 6px; background: var(--color-background); color: var(--color-text); cursor: pointer; transition: all 0.2s; }
.filter-btn:hover:not(:disabled) { background: var(--color-background-soft); }
.filter-btn.active { background: hsla(160, 100%, 37%, 1); border-color: hsla(160, 100%, 37%, 1); color: white; }
.filtered-info { font-size: 0.9rem; color: var(--color-text); opacity: 0.7; margin-bottom: 1rem; }

.table-wrapper { overflow-x: auto; border: 1px solid var(--color-border); border-radius: 8px; }
table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }
th, td { padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid var(--color-border); }
th { background: var(--color-background-soft); font-weight: 600; color: var(--color-heading); }
tr:last-child td { border-bottom: none; }
tr.has-match { background: rgba(76, 175, 80, 0.03); }
.seq-id { font-family: monospace; color: hsla(160, 100%, 37%, 1); max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.seq-length { font-family: monospace; text-align: right; }
.gene-name { font-family: monospace; font-weight: 600; color: #ff9800; background: rgba(255, 152, 0, 0.1); padding: 0.15rem 0.4rem; border-radius: 4px; font-size: 0.85rem; }
.seq-product { max-width: 300px; }
.product-desc { font-size: 0.9rem; }
.no-data { color: var(--color-text); opacity: 0.4; }
.annotation-links { display: flex; gap: 0.3rem; flex-wrap: wrap; }
.db-link { padding: 0.2rem 0.4rem; border-radius: 4px; text-decoration: none; font-size: 0.7rem; font-weight: 600; }
.db-link.uniref { background: rgba(156, 39, 176, 0.1); color: #9c27b0; }
.db-link.uniparc { background: rgba(33, 150, 243, 0.1); color: #2196f3; }
.db-link.ncbi { background: rgba(76, 175, 80, 0.1); color: #4caf50; }

.sequences-pagination { display: flex; align-items: center; justify-content: center; gap: 0.5rem; margin-top: 1.5rem; flex-wrap: wrap; }
.page-btn { min-width: 36px; height: 36px; padding: 0 0.75rem; border: 1px solid var(--color-border); border-radius: 6px; background: var(--color-background); color: var(--color-text); cursor: pointer; }
.page-btn:hover:not(:disabled) { background: var(--color-background-soft); }
.page-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.page-btn.active { background: hsla(160, 100%, 37%, 1); border-color: hsla(160, 100%, 37%, 1); color: white; }
.page-ellipsis { color: var(--color-text); opacity: 0.6; }
.page-info { margin-left: 1rem; font-size: 0.85rem; color: var(--color-text); opacity: 0.7; }
.sequences-loading { display: flex; align-items: center; gap: 0.75rem; padding: 2rem; justify-content: center; color: var(--color-text); }
.empty-filter-results { text-align: center; padding: 2rem; background: var(--color-background-soft); border-radius: 8px; }

.loading-stats { display: flex; align-items: center; gap: 0.75rem; padding: 4rem; justify-content: center; color: var(--color-text); }
.analysis-section { animation: fadeIn 0.3s ease; }

.annotation-overview { margin-bottom: 2rem; }
.annotation-rate { display: flex; align-items: center; gap: 1.5rem; padding: 1.5rem; background: var(--color-background-soft); border-radius: 12px; border: 1px solid var(--color-border); }
.rate-circle { position: relative; width: 100px; height: 100px; }
.circular-chart { width: 100%; height: 100%; }
.circle-bg { fill: none; stroke: var(--color-border); stroke-width: 3.8; }
.circle { fill: none; stroke: hsla(160, 100%, 37%, 1); stroke-width: 3.8; stroke-linecap: round; animation: progress 1s ease-out forwards; }
@keyframes progress { from { stroke-dasharray: 0, 100; } }
.rate-value { position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); font-size: 1.5rem; font-weight: 700; color: hsla(160, 100%, 37%, 1); }
.rate-info { display: flex; flex-direction: column; gap: 0.25rem; }
.rate-label { font-size: 1.1rem; font-weight: 600; color: var(--color-heading); }
.rate-detail { font-size: 0.9rem; color: var(--color-text); opacity: 0.8; }

.charts-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 1.5rem; }
.chart-card { background: var(--color-background-soft); border: 1px solid var(--color-border); border-radius: 12px; padding: 1.5rem; }
.chart-card-wide { grid-column: 1 / -1; }
.chart-card h4 { margin: 0 0 1rem 0; color: var(--color-heading); font-size: 1rem; }
.no-chart-data { text-align: center; padding: 2rem; color: var(--color-text); opacity: 0.6; }

.horizontal-bars { display: flex; flex-direction: column; gap: 0.5rem; }
.bar-item { display: grid; grid-template-columns: minmax(100px, 1fr) 2fr auto; gap: 0.75rem; align-items: center; }
.bar-label { font-size: 0.85rem; color: var(--color-text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.cog-code { display: inline-block; width: 20px; height: 20px; line-height: 20px; text-align: center; background: hsla(160, 100%, 37%, 0.15); color: hsla(160, 100%, 37%, 1); border-radius: 4px; font-weight: 600; font-size: 0.75rem; margin-right: 0.5rem; }
.bar-wrapper { height: 20px; background: var(--color-background); border-radius: 4px; overflow: hidden; }
.bar-fill { height: 100%; border-radius: 4px; transition: width 0.5s ease; }
.bar-value { font-size: 0.85rem; font-weight: 600; color: var(--color-heading); min-width: 40px; text-align: right; }

.go-items { display: flex; flex-wrap: wrap; gap: 0.5rem; }
.go-item { display: flex; justify-content: space-between; gap: 0.5rem; padding: 0.35rem 0.5rem; background: var(--color-background); border-radius: 4px; font-size: 0.8rem; }
.go-id { font-family: monospace; color: var(--color-text); }
.go-count { font-weight: 600; color: hsla(160, 100%, 37%, 1); }

.error-section { background: rgba(244, 67, 54, 0.1); border: 1px solid rgba(244, 67, 54, 0.3); border-radius: 8px; padding: 1.5rem; margin-bottom: 2rem; }
.error-section h3 { margin: 0 0 0.5rem 0; color: #f44336; }

.actions { margin-top: 2rem; text-align: center; }
.btn { padding: 0.75rem 1.5rem; font-size: 0.95rem; font-weight: 500; border-radius: 6px; cursor: pointer; text-decoration: none; display: inline-block; border: none; }
.btn-primary { background: hsla(160, 100%, 37%, 1); color: white; }
.btn-primary:hover { background: hsla(160, 100%, 32%, 1); }
.btn-secondary { background: transparent; color: hsla(160, 100%, 37%, 1); border: 1px solid hsla(160, 100%, 37%, 1); }

@media (max-width: 900px) { .charts-grid { grid-template-columns: 1fr; } }
@media (max-width: 600px) {
  .job-header { flex-direction: column; gap: 1rem; }
  .info-grid { grid-template-columns: 1fr; }
  .results-stats { grid-template-columns: 1fr; }
  .tab-btn { flex: 1; padding: 0.6rem 0.5rem; font-size: 0.85rem; }
  .sequences-header { flex-direction: column; }
  .annotation-rate { flex-direction: column; text-align: center; }
  .bar-item { grid-template-columns: 1fr; gap: 0.25rem; }
}
</style>
