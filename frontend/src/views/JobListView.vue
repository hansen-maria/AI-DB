<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { RouterLink } from 'vue-router'
import { listJobs, bulkDeleteJobs, type JobSummary, type JobStatus, type PaginationInfo } from '../api/jobs.ts'

// ── State ─────────────────────────────────────────────────────────────────────

const jobs        = ref<JobSummary[]>([])
const pagination  = ref<PaginationInfo | null>(null)
const loading     = ref(true)
const error       = ref('')
const currentPage = ref(1)
const perPage     = 20

// Filters
const searchInput  = ref('')
const statusFilter = ref<JobStatus | ''>('')

// Selection
const selectedIds = ref<Set<string>>(new Set())
const deleting    = ref(false)
const deleteError = ref('')

// ── Derived ───────────────────────────────────────────────────────────────────

const hasActiveFilters = computed(() => !!searchInput.value.trim() || !!statusFilter.value)

const allSelected = computed(() =>
    jobs.value.length > 0 && jobs.value.every(j => selectedIds.value.has(j.job_id))
)

const someSelected = computed(() => selectedIds.value.size > 0)

// ── Lookups ───────────────────────────────────────────────────────────────────

const statusColors: Record<JobStatus, string> = {
  pending:    '#ff9800',
  processing: '#2196f3',
  completed:  '#4caf50',
  failed:     '#f44336',
}

const statusLabels: Record<JobStatus, string> = {
  pending:    'Pending',
  processing: 'Processing',
  completed:  'Completed',
  failed:     'Failed',
}

const STATUS_PILLS: { value: JobStatus | ''; label: string }[] = [
  { value: '',           label: 'All'        },
  { value: 'completed',  label: 'Completed'  },
  { value: 'processing', label: 'Processing' },
  { value: 'pending',    label: 'Pending'    },
  { value: 'failed',     label: 'Failed'     },
]

// ── Pagination helpers ────────────────────────────────────────────────────────

const pageNumbers = computed(() => {
  if (!pagination.value) return []
  const total   = pagination.value.total_pages
  const current = pagination.value.page
  let start = Math.max(1, current - 2)
  let end   = Math.min(total, current + 2)
  if (current <= 3)           end   = Math.min(5, total)
  if (current >= total - 2)   start = Math.max(1, total - 4)
  const pages: number[] = []
  for (let i = start; i <= end; i++) pages.push(i)
  return pages
})

// ── Data loading ──────────────────────────────────────────────────────────────

async function loadJobs(page = 1) {
  loading.value = true
  error.value   = ''
  currentPage.value = page
  selectedIds.value = new Set()  // clear selection on reload

  try {
    const response = await listJobs({
      page,
      perPage,
      status: statusFilter.value  || undefined,
      search: searchInput.value.trim() || undefined,
    })
    jobs.value       = response.jobs
    pagination.value = response.pagination
  } catch (e) {
    if (e instanceof Error && e.message === 'API not available') {
      jobs.value       = []
      pagination.value = null
    } else {
      error.value = e instanceof Error ? e.message : 'Failed to load jobs'
    }
  } finally {
    loading.value = false
  }
}

function goToPage(page: number) {
  if (page >= 1 && (!pagination.value || page <= pagination.value.total_pages)) {
    loadJobs(page)
  }
}

// ── Filter actions ────────────────────────────────────────────────────────────

function applyStatusFilter(status: JobStatus | '') {
  statusFilter.value = status
  loadJobs(1)
}

function handleSearch() {
  loadJobs(1)
}

function clearFilters() {
  searchInput.value  = ''
  statusFilter.value = ''
  loadJobs(1)
}

// ── Selection ─────────────────────────────────────────────────────────────────

function toggleSelect(jobId: string) {
  const next = new Set(selectedIds.value)
  if (next.has(jobId)) next.delete(jobId)
  else                 next.add(jobId)
  selectedIds.value = next
}

function toggleSelectAll() {
  if (allSelected.value) {
    selectedIds.value = new Set()
  } else {
    selectedIds.value = new Set(jobs.value.map(j => j.job_id))
  }
}

// ── Bulk delete ───────────────────────────────────────────────────────────────

async function handleBulkDelete() {
  const count = selectedIds.value.size
  if (count === 0) return
  if (!confirm(`Delete ${count} job${count > 1 ? 's' : ''}? This cannot be undone.`)) return

  deleting.value    = true
  deleteError.value = ''
  try {
    const result = await bulkDeleteJobs([...selectedIds.value])
    if (result.forbidden.length > 0) {
      deleteError.value = `${result.forbidden.length} job(s) could not be deleted (not authorized).`
    }
    await loadJobs(currentPage.value)
  } catch (e) {
    deleteError.value = e instanceof Error ? e.message : 'Bulk delete failed'
  } finally {
    deleting.value = false
  }
}

// ── Formatting ────────────────────────────────────────────────────────────────

function formatDate(dateStr: string) {
  const date = new Date(dateStr)
  const diff = Date.now() - date.getTime()
  if (diff < 60_000)    return 'Just now'
  if (diff < 3_600_000) { const m = Math.floor(diff / 60_000);    return `${m} min${m > 1 ? 's' : ''} ago` }
  if (diff < 86_400_000){ const h = Math.floor(diff / 3_600_000); return `${h} hour${h > 1 ? 's' : ''} ago` }
  return date.toLocaleDateString()
}

onMounted(() => loadJobs())
</script>

<template>
  <div class="jobs-page">
    <div class="page-header">
      <div>
        <h2>All Jobs</h2>
        <p>View and manage your annotation jobs</p>
      </div>
      <RouterLink to="/submit" class="btn btn-primary">
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
        New Job
      </RouterLink>
    </div>

    <!-- Filter Bar -->
    <div class="filter-bar">
      <!-- Search -->
      <div class="search-wrapper">
        <svg class="search-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <input
            v-model="searchInput"
            class="search-input"
            placeholder="Search by filename…"
            @keyup.enter="handleSearch"
        />
        <button v-if="searchInput" class="search-clear" @click="searchInput = ''; loadJobs(1)" title="Clear">✕</button>
      </div>

      <!-- Status pills -->
      <div class="status-pills">
        <button
            v-for="pill in STATUS_PILLS"
            :key="pill.value"
            class="status-pill"
            :class="{ active: statusFilter === pill.value }"
            @click="applyStatusFilter(pill.value)"
        >{{ pill.label }}</button>
      </div>

      <!-- Clear filters -->
      <button v-if="hasActiveFilters" class="clear-filters-btn" @click="clearFilters">
        Clear filters
      </button>
    </div>

    <!-- Bulk actions bar (visible when items selected) -->
    <Transition name="bulk-bar">
      <div v-if="someSelected" class="bulk-actions-bar">
        <label class="select-all-label">
          <input type="checkbox" :checked="allSelected" @change="toggleSelectAll" />
          <span>{{ selectedIds.size }} selected</span>
        </label>
        <div class="bulk-actions-bar__right">
          <span v-if="deleteError" class="bulk-error">{{ deleteError }}</span>
          <button class="bulk-delete-btn" :disabled="deleting" @click="handleBulkDelete">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4h6v2"/>
            </svg>
            {{ deleting ? 'Deleting…' : `Delete ${selectedIds.size}` }}
          </button>
          <button class="bulk-cancel-btn" @click="selectedIds = new Set()">Cancel</button>
        </div>
      </div>
    </Transition>

    <!-- Loading State -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <p>Loading jobs...</p>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="error-state">
      <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
      </svg>
      <p>{{ error }}</p>
      <button @click="loadJobs(currentPage)" class="btn btn-secondary">Try Again</button>
    </div>

    <!-- Empty State -->
    <div v-else-if="jobs.length === 0" class="empty-state">
      <svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
        <polyline points="14 2 14 8 20 8"/><line x1="12" y1="18" x2="12" y2="12"/><line x1="9" y1="15" x2="15" y2="15"/>
      </svg>
      <h3>{{ hasActiveFilters ? 'No matching jobs' : 'No jobs yet' }}</h3>
      <p v-if="hasActiveFilters">
        No jobs match your current filters.
        <button class="inline-link" @click="clearFilters">Clear filters</button>
      </p>
      <p v-else>Submit your first FASTA file to get started with annotation.</p>
      <RouterLink v-if="!hasActiveFilters" to="/submit" class="btn btn-primary">Submit First Job</RouterLink>
    </div>

    <!-- Jobs List -->
    <template v-else>
      <!-- Select-all row when no bulk bar is visible -->
      <div v-if="!someSelected" class="list-header">
        <label class="select-all-label muted">
          <input type="checkbox" :checked="allSelected" @change="toggleSelectAll" />
          <span>Select all</span>
        </label>
        <span class="list-count">{{ pagination?.total_items ?? jobs.length }} jobs</span>
      </div>

      <div class="jobs-list">
        <div v-for="job in jobs" :key="job.job_id" class="job-card-wrapper">
          <!-- Checkbox -->
          <input
              type="checkbox"
              class="job-checkbox"
              :checked="selectedIds.has(job.job_id)"
              @change="toggleSelect(job.job_id)"
              @click.stop
          />

          <!-- Card link -->
          <RouterLink :to="{ name: 'job', params: { id: job.job_id } }" class="job-card" :class="{ selected: selectedIds.has(job.job_id) }">
            <div class="job-status">
              <span class="status-dot" :style="{ backgroundColor: statusColors[job.status] }">
                <span v-if="job.status === 'processing'" class="pulse"></span>
              </span>
              <span class="status-text">{{ statusLabels[job.status] }}</span>
            </div>

            <div class="job-info">
              <span class="job-name">{{ job.filename || 'Direct Input' }}</span>
              <span class="job-id">{{ job.job_id.substring(0, 8) }}…</span>
            </div>

            <div class="job-stats">
              <span class="stat">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                  <polyline points="14 2 14 8 20 8"/>
                </svg>
                {{ job.sequence_count.toLocaleString() }} seq
              </span>
              <span v-if="job.status === 'completed'" class="stat success">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
                {{ job.hash_matches.toLocaleString() }} matches
              </span>
            </div>

            <div class="job-time">{{ formatDate(job.updated_at) }}</div>

            <svg class="arrow" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="9 18 15 12 9 6"/>
            </svg>
          </RouterLink>
        </div>
      </div>

      <!-- Pagination -->
      <div v-if="pagination && pagination.total_pages > 1" class="pagination">
        <button class="page-btn" :disabled="!pagination.has_prev" @click="goToPage(pagination.page - 1)">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="15 18 9 12 15 6"/></svg>
        </button>
        <button v-if="pageNumbers[0] > 1" class="page-btn" @click="goToPage(1)">1</button>
        <span v-if="pageNumbers[0] > 2" class="page-ellipsis">...</span>
        <button v-for="page in pageNumbers" :key="page" class="page-btn" :class="{ active: page === pagination.page }" @click="goToPage(page)">{{ page }}</button>
        <span v-if="pageNumbers[pageNumbers.length - 1] < pagination.total_pages - 1" class="page-ellipsis">...</span>
        <button v-if="pageNumbers[pageNumbers.length - 1] < pagination.total_pages" class="page-btn" @click="goToPage(pagination.total_pages)">{{ pagination.total_pages }}</button>
        <button class="page-btn" :disabled="!pagination.has_next" @click="goToPage(pagination.page + 1)">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>
        </button>
        <span class="page-info">{{ pagination.total_items }} total jobs</span>
      </div>

      <!-- Refresh -->
      <div class="refresh-section">
        <button @click="loadJobs(currentPage)" class="refresh-btn">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
          </svg>
          Refresh
        </button>
      </div>
    </template>
  </div>
</template>
<style scoped>
/* ── Filter bar ─────────────────────────────────────────────────────────────── */
.filter-bar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.search-wrapper {
  position: relative;
  display: flex;
  align-items: center;
  flex: 1;
  min-width: 200px;
  max-width: 340px;
}
.search-icon { position: absolute; left: 0.65rem; color: var(--color-text); opacity: 0.45; pointer-events: none; }
.search-input {
  width: 100%;
  padding: 0.45rem 2rem 0.45rem 2.1rem;
  font-size: 0.875rem;
  border: 1px solid var(--color-border);
  border-radius: 7px;
  background: var(--color-background);
  color: var(--color-text);
}
.search-input:focus { outline: none; border-color: hsla(160,100%,37%,0.6); }
.search-clear { position: absolute; right: 0.6rem; background: none; border: none; cursor: pointer; color: var(--color-text); opacity: 0.45; font-size: 0.75rem; padding: 0; line-height: 1; }
.search-clear:hover { opacity: 1; }

.status-pills { display: flex; gap: 0.35rem; flex-wrap: wrap; }
.status-pill {
  padding: 0.3rem 0.7rem;
  font-size: 0.8rem;
  border: 1px solid var(--color-border);
  border-radius: 20px;
  background: var(--color-background);
  color: var(--color-text);
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}
.status-pill:hover { border-color: hsla(160,100%,37%,0.5); }
.status-pill.active { background: hsla(160,100%,37%,1); border-color: hsla(160,100%,37%,1); color: #fff; }

.clear-filters-btn { font-size: 0.8rem; background: none; border: none; color: var(--color-text); opacity: 0.6; cursor: pointer; text-decoration: underline; white-space: nowrap; }
.clear-filters-btn:hover { opacity: 1; }

/* ── Bulk actions bar ────────────────────────────────────────────────────────── */
.bulk-actions-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.6rem 1rem;
  margin-bottom: 0.75rem;
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  gap: 1rem;
  flex-wrap: wrap;
}
.bulk-actions-bar__right { display: flex; align-items: center; gap: 0.75rem; }
.bulk-error { font-size: 0.8rem; color: #f44336; }
.bulk-delete-btn {
  display: inline-flex; align-items: center; gap: 0.4rem;
  padding: 0.35rem 0.85rem;
  font-size: 0.82rem; font-weight: 500;
  background: #f44336; color: #fff;
  border: none; border-radius: 6px; cursor: pointer; transition: background 0.15s;
}
.bulk-delete-btn:hover:not(:disabled) { background: #d32f2f; }
.bulk-delete-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.bulk-cancel-btn { font-size: 0.82rem; background: none; border: 1px solid var(--color-border); border-radius: 6px; padding: 0.35rem 0.75rem; cursor: pointer; color: var(--color-text); transition: border-color 0.15s; }
.bulk-cancel-btn:hover { border-color: var(--color-text); }

.bulk-bar-enter-active, .bulk-bar-leave-active { transition: all 0.2s ease; }
.bulk-bar-enter-from, .bulk-bar-leave-to { opacity: 0; transform: translateY(-6px); }

/* ── List header ─────────────────────────────────────────────────────────────── */
.list-header { display: flex; align-items: center; justify-content: space-between; padding: 0 0.25rem; margin-bottom: 0.5rem; }
.list-count { font-size: 0.8rem; color: var(--color-text); opacity: 0.5; }

/* ── Select all label ────────────────────────────────────────────────────────── */
.select-all-label { display: flex; align-items: center; gap: 0.5rem; font-size: 0.85rem; color: var(--color-text); cursor: pointer; }
.select-all-label.muted { opacity: 0.5; }
.select-all-label input[type="checkbox"] { width: 15px; height: 15px; accent-color: hsla(160,100%,37%,1); cursor: pointer; }

/* ── Job card with checkbox ──────────────────────────────────────────────────── */
.job-card-wrapper {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0;
}
.job-checkbox { width: 16px; height: 16px; flex-shrink: 0; accent-color: hsla(160,100%,37%,1); cursor: pointer; }
.job-card { flex: 1; }
.job-card.selected { border-color: hsla(160,100%,37%,0.4); background: hsla(160,100%,37%,0.03); }

/* ── Inline link in empty state ──────────────────────────────────────────────── */
.inline-link { background: none; border: none; color: hsla(160,100%,37%,1); cursor: pointer; font-size: inherit; padding: 0; text-decoration: underline; }
.jobs-page {
  max-width: 800px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 2rem;
  gap: 1rem;
  flex-wrap: wrap;
}

.page-header h2 {
  margin: 0;
  font-size: 1.5rem;
  color: var(--color-heading);
}

.page-header p {
  margin: 0.25rem 0 0 0;
  color: var(--color-text);
  opacity: 0.8;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1.25rem;
  font-size: 0.95rem;
  font-weight: 500;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.3s;
  text-decoration: none;
  border: none;
}

.btn-primary {
  background: hsla(160, 100%, 37%, 1);
  color: white;
}

.btn-primary:hover {
  background: hsla(160, 100%, 32%, 1);
}

.btn-secondary {
  background: transparent;
  color: hsla(160, 100%, 37%, 1);
  border: 1px solid hsla(160, 100%, 37%, 1);
}

.btn-secondary:hover {
  background: hsla(160, 100%, 37%, 0.1);
}

/* Loading & Error States */
.loading-state,
.error-state,
.empty-state {
  text-align: center;
  padding: 4rem 2rem;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid var(--color-border);
  border-top-color: hsla(160, 100%, 37%, 1);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin: 0 auto 1rem;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.error-state svg {
  color: #f44336;
  margin-bottom: 1rem;
}

.error-state p,
.loading-state p {
  color: var(--color-text);
  margin-bottom: 1rem;
}

.empty-state svg {
  color: var(--color-text);
  opacity: 0.3;
  margin-bottom: 1rem;
}

.empty-state h3 {
  margin: 0 0 0.5rem 0;
  color: var(--color-heading);
}

.empty-state p {
  color: var(--color-text);
  opacity: 0.8;
  margin-bottom: 1.5rem;
}

/* Jobs List */
.jobs-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.job-card {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem 1.25rem;
  background: var(--color-background);
  border: 1px solid var(--color-border);
  border-radius: 10px;
  text-decoration: none;
  color: inherit;
  transition: all 0.2s;
}

.job-card:hover {
  border-color: var(--color-border-hover);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.job-status {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 100px;
}

.status-dot {
  position: relative;
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.pulse {
  position: absolute;
  inset: -3px;
  border-radius: 50%;
  background: inherit;
  animation: pulse 1.5s ease-out infinite;
}

@keyframes pulse {
  0% { opacity: 0.8; transform: scale(1); }
  100% { opacity: 0; transform: scale(2); }
}

.status-text {
  font-size: 0.85rem;
  font-weight: 500;
  color: var(--color-text);
}

.job-info {
  flex: 1;
  min-width: 0;
}

.job-name {
  display: block;
  font-weight: 500;
  color: var(--color-heading);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.job-id {
  font-size: 0.8rem;
  color: var(--color-text);
  opacity: 0.6;
  font-family: monospace;
}

.job-stats {
  display: flex;
  gap: 1rem;
}

.stat {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.85rem;
  color: var(--color-text);
  opacity: 0.8;
}

.stat.success {
  color: #4caf50;
  opacity: 1;
}

.job-time {
  font-size: 0.85rem;
  color: var(--color-text);
  opacity: 0.6;
  min-width: 80px;
  text-align: right;
}

.arrow {
  color: var(--color-text);
  opacity: 0.4;
  flex-shrink: 0;
}

/* Refresh Section */
.refresh-section {
  margin-top: 2rem;
  text-align: center;
}

.refresh-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  font-size: 0.9rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-background);
  color: var(--color-text);
  cursor: pointer;
  transition: all 0.2s;
}

.refresh-btn:hover {
  background: var(--color-background-soft);
}

/* Responsive */
@media (max-width: 600px) {
  .job-card {
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .job-status {
    min-width: auto;
  }

  .job-info {
    flex-basis: calc(100% - 120px);
  }

  .job-stats {
    order: 4;
    width: 100%;
  }

  .job-time {
    order: 3;
    min-width: auto;
  }

  .arrow {
    display: none;
  }
}

/* Pagination */
.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  margin-top: 2rem;
  flex-wrap: wrap;
}

.page-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 36px;
  height: 36px;
  padding: 0 0.75rem;
  font-size: 0.9rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-background);
  color: var(--color-text);
  cursor: pointer;
  transition: all 0.2s;
}

.page-btn:hover:not(:disabled) {
  background: var(--color-background-soft);
  border-color: var(--color-border-hover);
}

.page-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.page-btn.active {
  background: hsla(160, 100%, 37%, 1);
  border-color: hsla(160, 100%, 37%, 1);
  color: white;
}

.page-ellipsis {
  color: var(--color-text);
  opacity: 0.6;
  padding: 0 0.25rem;
}

.page-info {
  margin-left: 1rem;
  font-size: 0.85rem;
  color: var(--color-text);
  opacity: 0.7;
}

@media (max-width: 600px) {
  .pagination {
    gap: 0.25rem;
  }

  .page-btn {
    min-width: 32px;
    height: 32px;
    padding: 0 0.5rem;
    font-size: 0.85rem;
  }

  .page-info {
    width: 100%;
    text-align: center;
    margin: 0.5rem 0 0 0;
  }
}
</style>