<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useRoute, useRouter, RouterLink } from 'vue-router'
import {
  getJob, deleteJob, downloadJobResults, downloadOptions, getJobStats,
  type PaginatedJobResponse, type JobStatus,
  type SequenceFilter, type DownloadFormat, type FunctionalStats
} from '../api/jobs.ts'

const route = useRoute()
const router = useRouter()

// Job data
const job = ref<PaginatedJobResponse | null>(null)
const allSequences = ref<any[]>([]) // All sequences for client-side filtering
const stats = ref<FunctionalStats | null>(null)

// UI state
const loading = ref(true)
const loadingStats = ref(false)
const error = ref('')
const deleting = ref(false)
const downloading = ref(false)
const downloadError = ref('')
const currentPage = ref(1)
const perPage = 20

// Tab state
const activeTab = ref<'overview' | 'sequences' | 'analysis'>('overview')

// Advanced filter state
const showAdvancedFilters = ref(false)
const currentFilter = ref<SequenceFilter>('all')
const searchText = ref('')
const debouncedSearch = ref('') // Debounced version for filtering
const minLength = ref<number | undefined>(undefined)
const maxLength = ref<number | undefined>(undefined)
const selectedCog = ref('')
const selectedEcClass = ref('')
const hasGeneOnly = ref(false)
const hasProductOnly = ref(false)

// Debounce search input
let searchTimeout: number | null = null
watch(searchText, (val) => {
  if (searchTimeout) clearTimeout(searchTimeout)
  searchTimeout = window.setTimeout(() => {
    debouncedSearch.value = val
  }, 80)
})

let pollInterval: number | null = null

const jobId = computed(() => route.params.id as string)

// Client-side filtered sequences
const filteredSequences = computed(() => {
  if (!allSequences.value.length) return []

  return allSequences.value.filter(seq => {
    // Basic filter (match status)
    if (currentFilter.value === 'hash_match' && seq.annotation_source !== 'hash_match') return false
    if (currentFilter.value === 'none' && seq.annotation_source) return false

    // Text search (use debounced value)
    if (debouncedSearch.value) {
      const search = debouncedSearch.value.toLowerCase()
      const idMatch = seq.id?.toLowerCase().includes(search)
      const geneMatch = seq.gene?.toLowerCase().includes(search)
      const productMatch = seq.product?.toLowerCase().includes(search)
      if (!idMatch && !geneMatch && !productMatch) return false
    }

    // Length filters
    if (minLength.value !== undefined && seq.length < minLength.value) return false
    if (maxLength.value !== undefined && seq.length > maxLength.value) return false

    // COG filter
    if (selectedCog.value && (!seq.cog_category || !seq.cog_category.includes(selectedCog.value))) return false

    // EC class filter
    if (selectedEcClass.value) {
      if (!seq.ec_ids) return false
      const hasEc = seq.ec_ids.split(',').some((e: string) => e.trim().startsWith(selectedEcClass.value))
      if (!hasEc) return false
    }

    // Has gene filter
    if (hasGeneOnly.value && (!seq.gene || seq.gene === '')) return false

    // Has product filter
    if (hasProductOnly.value && (!seq.product || seq.product === '')) return false

    return true
  })
})

// Paginated sequences from filtered results
const paginatedSequences = computed(() => {
  const start = (currentPage.value - 1) * perPage
  return filteredSequences.value.slice(start, start + perPage)
})

// Pagination info
const pagination = computed(() => {
  const total = filteredSequences.value.length
  const totalPages = Math.ceil(total / perPage) || 1
  return {
    page: currentPage.value,
    per_page: perPage,
    total_items: total,
    total_pages: totalPages,
    has_next: currentPage.value < totalPages,
    has_prev: currentPage.value > 1
  }
})

// Progress percentage
const progressPercent = computed(() => {
  if (!job.value || job.value.sequence_count === 0) return 0
  return Math.min(100, (job.value.processed_count / job.value.sequence_count) * 100)
})

// Check if any advanced filters are active
const hasActiveFilters = computed(() => {
  return debouncedSearch.value !== '' ||
      minLength.value !== undefined ||
      maxLength.value !== undefined ||
      selectedCog.value !== '' ||
      selectedEcClass.value !== '' ||
      hasGeneOnly.value ||
      hasProductOnly.value ||
      currentFilter.value !== 'all'
})

// Reset to page 1 when filters change
watch([debouncedSearch, currentFilter, minLength, maxLength, selectedCog, selectedEcClass, hasGeneOnly, hasProductOnly], () => {
  currentPage.value = 1
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

const cogCategories = [
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

const ecClasses = [
  { value: '1', label: 'EC 1 - Oxidoreductases' },
  { value: '2', label: 'EC 2 - Transferases' },
  { value: '3', label: 'EC 3 - Hydrolases' },
  { value: '4', label: 'EC 4 - Lyases' },
  { value: '5', label: 'EC 5 - Isomerases' },
  { value: '6', label: 'EC 6 - Ligases' },
  { value: '7', label: 'EC 7 - Translocases' },
]

const pageNumbers = computed(() => {
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

async function loadJob() {
  loading.value = true

  try {
    // Load all sequences at once (up to 10000) for client-side filtering
    const response = await getJob(jobId.value, 1, 10000, 'all')
    job.value = response
    allSequences.value = response.sequences || []

    if (response.status === 'pending' || response.status === 'processing') {
      startPolling()
    } else if (response.status === 'completed' && !stats.value) {
      loadStats()
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load job'
  } finally {
    loading.value = false
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

function clearFilters() {
  currentFilter.value = 'all'
  searchText.value = ''
  debouncedSearch.value = ''
  minLength.value = undefined
  maxLength.value = undefined
  selectedCog.value = ''
  selectedEcClass.value = ''
  hasGeneOnly.value = false
  hasProductOnly.value = false
  currentPage.value = 1
}

function goToPage(page: number) {
  if (page >= 1 && page <= pagination.value.total_pages) {
    currentPage.value = page
  }
}

function startPolling() {
  if (pollInterval) return
  pollInterval = window.setInterval(async () => {
    try {
      const response = await getJob(jobId.value, 1, 10000, 'all')
      job.value = response
      allSequences.value = response.sequences || []
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

const sequentialColors = [
  '#00bd7e', '#00ad73', '#009d68', '#008d5d', '#007d52',
  '#006d47', '#005d3c', '#004d31', '#003d26', '#002d1b'
]


const categoricalColors = [
  '#00bd7e',
  '#00a896',
  '#028090',
  '#05668d',
  '#6b5b95',
  '#d64161',
  '#ff7b25',
  '#f6ab3c',
  '#3d5a80',
  '#7eb77f',
]

function getSequentialColor(index: number): string {
  return sequentialColors[Math.min(index, sequentialColors.length - 1)]
}

function getCategoricalColor(index: number): string {
  return categoricalColors[index % categoricalColors.length]
}

onMounted(() => loadJob())
onUnmounted(() => {
  stopPolling()
  if (searchTimeout) clearTimeout(searchTimeout)
})
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
            <!-- Search and Filter Bar -->
            <div class="search-filter-bar">
              <div class="search-box">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
                </svg>
                <input
                    type="text"
                    v-model="searchText"
                    placeholder="Search ID, gene, product..."
                    class="search-input"
                />
                <button v-if="searchText" class="clear-search" @click="searchText = ''">×</button>
              </div>

              <div class="filter-buttons">
                <button
                    v-for="opt in filterOptions"
                    :key="opt.value"
                    class="filter-btn"
                    :class="{ active: currentFilter === opt.value }"
                    @click="currentFilter = opt.value"
                >
                  {{ opt.label }}
                </button>
              </div>

              <button
                  class="advanced-toggle"
                  :class="{ active: showAdvancedFilters, 'has-filters': hasActiveFilters }"
                  @click="showAdvancedFilters = !showAdvancedFilters"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/>
                </svg>
                Filters
                <span v-if="hasActiveFilters" class="filter-badge">!</span>
              </button>
            </div>

            <!-- Advanced Filter Panel -->
            <div v-if="showAdvancedFilters" class="advanced-filters">
              <div class="filter-row">
                <div class="filter-group">
                  <label>Sequence Length</label>
                  <div class="length-inputs">
                    <input type="number" v-model.number="minLength" placeholder="Min" min="0" />
                    <span>–</span>
                    <input type="number" v-model.number="maxLength" placeholder="Max" min="0" />
                    <span class="unit">aa</span>
                  </div>
                </div>

                <div class="filter-group">
                  <label>COG Category</label>
                  <select v-model="selectedCog">
                    <option value="">All categories</option>
                    <option v-for="cog in cogCategories" :key="cog.value" :value="cog.value">
                      {{ cog.label }}
                    </option>
                  </select>
                </div>

                <div class="filter-group">
                  <label>Enzyme Class (EC)</label>
                  <select v-model="selectedEcClass">
                    <option value="">All classes</option>
                    <option v-for="ec in ecClasses" :key="ec.value" :value="ec.value">
                      {{ ec.label }}
                    </option>
                  </select>
                </div>
              </div>

              <div class="filter-row">
                <div class="filter-group checkbox-group">
                  <label class="checkbox-label">
                    <input type="checkbox" v-model="hasGeneOnly" />
                    <span>Has gene name</span>
                  </label>
                  <label class="checkbox-label">
                    <input type="checkbox" v-model="hasProductOnly" />
                    <span>Has function description</span>
                  </label>
                </div>

                <button v-if="hasActiveFilters" class="clear-filters-btn" @click="clearFilters">
                  Clear all filters
                </button>
              </div>
            </div>

            <!-- Filter Results Info -->
            <div class="filtered-info">
              <span v-if="hasActiveFilters">
                <strong>{{ filteredSequences.length.toLocaleString() }}</strong> of {{ allSequences.length.toLocaleString() }} sequences
                <span v-if="searchText && searchText !== debouncedSearch" class="typing-indicator">...</span>
              </span>
              <span v-else>
                {{ allSequences.length.toLocaleString() }} sequences
              </span>
            </div>

            <!-- Table -->
            <div v-if="paginatedSequences.length > 0" class="sequences-table">
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
                  <tr v-for="seq in paginatedSequences" :key="seq.id" :class="{ 'has-match': hasAnnotationLinks(seq) }">
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

            <div v-else-if="filteredSequences.length === 0 && allSequences.length > 0" class="empty-filter-results">
              <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
              </svg>
              <p>No sequences match the current filters.</p>
              <button class="btn btn-secondary" @click="clearFilters">Clear Filters</button>
            </div>

            <!-- Pagination -->
            <div v-if="pagination.total_pages > 1" class="sequences-pagination">
              <button class="page-btn" :disabled="!pagination.has_prev" @click="goToPage(pagination.page - 1)">←</button>
              <button v-if="pageNumbers[0] > 1" class="page-btn" @click="goToPage(1)">1</button>
              <span v-if="pageNumbers[0] > 2" class="page-ellipsis">...</span>
              <button v-for="page in pageNumbers" :key="page" class="page-btn" :class="{ active: page === pagination.page }"
                      @click="goToPage(page)">{{ page }}</button>
              <span v-if="pageNumbers[pageNumbers.length - 1] < pagination.total_pages - 1" class="page-ellipsis">...</span>
              <button v-if="pageNumbers[pageNumbers.length - 1] < pagination.total_pages" class="page-btn"
                      @click="goToPage(pagination.total_pages)">{{ pagination.total_pages }}</button>
              <button class="page-btn" :disabled="!pagination.has_next" @click="goToPage(pagination.page + 1)">→</button>
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
                      <div class="bar-fill" :style="{ width: `${(item.count / stats.top_genes[0].count) * 100}%`, backgroundColor: getSequentialColor(index) }"></div>
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
                      <div class="bar-fill" :style="{ width: `${(item.count / stats.top_products[0].count) * 100}%`, backgroundColor: getSequentialColor(index) }"></div>
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
                      <div class="bar-fill" :style="{ width: `${(item.count / stats.cog_categories[0].count) * 100}%`, backgroundColor: getCategoricalColor(index) }"></div>
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
                      <div class="bar-fill" :style="{ width: `${(item.count / stats.ec_classes[0].count) * 100}%`, backgroundColor: getCategoricalColor(index) }"></div>
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

/* Search and Filter Bar */
.sequences-section { margin-top: 1rem; }

.search-filter-bar {
  display: flex;
  gap: 1rem;
  align-items: center;
  flex-wrap: wrap;
  margin-bottom: 1rem;
}

.search-box {
  flex: 1;
  min-width: 250px;
  position: relative;
  display: flex;
  align-items: center;
}

.search-box svg {
  position: absolute;
  left: 12px;
  color: var(--color-text);
  opacity: 0.5;
}

.search-input {
  width: 100%;
  padding: 0.6rem 2.5rem 0.6rem 2.5rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-background);
  color: var(--color-text);
  font-size: 0.9rem;
}

.search-input:focus {
  outline: none;
  border-color: hsla(160, 100%, 37%, 0.5);
  box-shadow: 0 0 0 3px hsla(160, 100%, 37%, 0.1);
}

.typing-indicator {
  color: hsla(160, 100%, 37%, 1);
  animation: blink 0.8s infinite;
}

@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0.3; }
}

.clear-search {
  position: absolute;
  right: 8px;
  background: none;
  border: none;
  color: var(--color-text);
  opacity: 0.5;
  cursor: pointer;
  font-size: 1.2rem;
  padding: 0.25rem;
}

.clear-search:hover { opacity: 1; }

.filter-buttons { display: flex; gap: 0.5rem; }
.filter-btn {
  padding: 0.5rem 1rem;
  font-size: 0.85rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-background);
  color: var(--color-text);
  cursor: pointer;
  transition: all 0.15s ease;
}
.filter-btn:hover:not(:disabled) { background: var(--color-background-soft); }
.filter-btn.active {
  background: hsla(160, 100%, 37%, 1);
  border-color: hsla(160, 100%, 37%, 1);
  color: white;
  transform: scale(1.02);
}

.advanced-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-background);
  color: var(--color-text);
  cursor: pointer;
  position: relative;
  transition: all 0.2s;
}

.advanced-toggle:hover { background: var(--color-background-soft); }
.advanced-toggle.active { border-color: hsla(160, 100%, 37%, 0.5); background: hsla(160, 100%, 37%, 0.05); }
.advanced-toggle.has-filters { border-color: #ff9800; }

.filter-badge {
  position: absolute;
  top: -4px;
  right: -4px;
  width: 16px;
  height: 16px;
  background: #ff9800;
  color: white;
  border-radius: 50%;
  font-size: 0.7rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* Advanced Filter Panel */
.advanced-filters {
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 1rem;
  margin-bottom: 1rem;
  animation: slideDown 0.2s ease;
}

@keyframes slideDown {
  from { opacity: 0; transform: translateY(-10px); }
  to { opacity: 1; transform: translateY(0); }
}

.filter-row {
  display: flex;
  gap: 1.5rem;
  flex-wrap: wrap;
  margin-bottom: 1rem;
}

.filter-row:last-child { margin-bottom: 0; }

.filter-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 180px;
}

.filter-group label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text);
  opacity: 0.8;
}

.filter-group select,
.filter-group input[type="number"] {
  padding: 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-background);
  color: var(--color-text);
  font-size: 0.85rem;
}

.length-inputs {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.length-inputs input {
  width: 80px;
}

.length-inputs span { color: var(--color-text); opacity: 0.5; }
.unit { font-size: 0.8rem; }

.checkbox-group {
  flex-direction: row;
  align-items: center;
  gap: 1.5rem;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
  font-size: 0.9rem !important;
  font-weight: normal !important;
}

.checkbox-label input { cursor: pointer; }

.clear-filters-btn {
  padding: 0.5rem 1rem;
  border: 1px solid #f44336;
  border-radius: 6px;
  background: transparent;
  color: #f44336;
  cursor: pointer;
  font-size: 0.85rem;
  margin-left: auto;
}

.clear-filters-btn:hover { background: rgba(244, 67, 54, 0.1); }

.filtered-info {
  font-size: 0.9rem;
  color: var(--color-text);
  padding: 0.75rem 0;
  border-bottom: 1px solid var(--color-border);
  margin-bottom: 1rem;
}

.filtered-info strong {
  color: hsla(160, 100%, 37%, 1);
  font-size: 1rem;
}

/* Table */
.sequences-table {
  animation: fadeIn 0.15s ease;
}

.table-wrapper { overflow-x: auto; border: 1px solid var(--color-border); border-radius: 8px; }
table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }
th, td { padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid var(--color-border); }
th { background: var(--color-background-soft); font-weight: 600; color: var(--color-heading); position: sticky; top: 0; }
tr:last-child td { border-bottom: none; }
tr.has-match { background: rgba(76, 175, 80, 0.03); }
tbody tr { transition: background-color 0.1s ease; }
tbody tr:hover { background: var(--color-background-soft); }
.seq-id { font-family: monospace; color: hsla(160, 100%, 37%, 1); max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.seq-length { font-family: monospace; }
.gene-name { font-family: monospace; font-weight: 600; color: #ff9800; background: rgba(255, 152, 0, 0.1); padding: 0.15rem 0.4rem; border-radius: 4px; font-size: 0.85rem; }
.seq-product { max-width: 300px; }
.product-desc { font-size: 0.9rem; }
.no-data { color: var(--color-text); opacity: 0.4; }
.annotation-links { display: flex; gap: 0.3rem; flex-wrap: wrap; }
.db-link { padding: 0.2rem 0.4rem; border-radius: 4px; text-decoration: none; font-size: 0.7rem; font-weight: 600; transition: transform 0.1s ease; }
.db-link:hover { transform: translateY(-1px); }
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

.empty-filter-results {
  text-align: center;
  padding: 3rem 2rem;
  background: var(--color-background-soft);
  border-radius: 8px;
}
.empty-filter-results svg { color: var(--color-text); opacity: 0.3; margin-bottom: 1rem; }
.empty-filter-results p { margin-bottom: 1rem; color: var(--color-text); opacity: 0.7; }

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
  .search-filter-bar { flex-direction: column; align-items: stretch; }
  .search-box { min-width: 100%; }
  .filter-buttons { justify-content: center; }
  .advanced-toggle { justify-content: center; }
  .filter-row { flex-direction: column; }
  .filter-group { min-width: 100%; }
  .checkbox-group { flex-wrap: wrap; }
  .annotation-rate { flex-direction: column; text-align: center; }
  .bar-item { grid-template-columns: 1fr; gap: 0.25rem; }
}
</style>
