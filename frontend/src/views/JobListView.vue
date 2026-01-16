<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { RouterLink } from 'vue-router'
import { listJobs, type JobSummary, type JobStatus, type PaginationInfo } from '../api/jobs.ts'

const jobs = ref<JobSummary[]>([])
const pagination = ref<PaginationInfo | null>(null)
const loading = ref(true)
const error = ref('')
const currentPage = ref(1)
const perPage = 20

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

// Generate page numbers to display
const pageNumbers = computed(() => {
  if (!pagination.value) return []
  const total = pagination.value.total_pages
  const current = pagination.value.page
  const pages: number[] = []

  // Show max 5 pages around current
  let start = Math.max(1, current - 2)
  let end = Math.min(total, current + 2)

  // Adjust if near edges
  if (current <= 3) {
    end = Math.min(5, total)
  }
  if (current >= total - 2) {
    start = Math.max(1, total - 4)
  }

  for (let i = start; i <= end; i++) {
    pages.push(i)
  }
  return pages
})

async function loadJobs(page = 1) {
  loading.value = true
  error.value = ''
  currentPage.value = page

  try {
    const response = await listJobs(page, perPage)
    jobs.value = response.jobs
    pagination.value = response.pagination
  } catch (e) {
    if (e instanceof Error && e.message === 'API not available') {
      jobs.value = []
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

function formatDate(dateStr: string) {
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - date.getTime()

  if (diff < 60000) return 'Just now'
  if (diff < 3600000) {
    const mins = Math.floor(diff / 60000)
    return `${mins} min${mins > 1 ? 's' : ''} ago`
  }
  if (diff < 86400000) {
    const hours = Math.floor(diff / 3600000)
    return `${hours} hour${hours > 1 ? 's' : ''} ago`
  }
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
          <line x1="12" y1="5" x2="12" y2="19"/>
          <line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
        New Job
      </RouterLink>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <p>Loading jobs...</p>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="error-state">
      <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="12" y1="8" x2="12" y2="12"/>
        <line x1="12" y1="16" x2="12.01" y2="16"/>
      </svg>
      <p>{{ error }}</p>
      <button @click="loadJobs(currentPage)" class="btn btn-secondary">Try Again</button>
    </div>

    <!-- Empty State -->
    <div v-else-if="jobs.length === 0" class="empty-state">
      <svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
        <polyline points="14 2 14 8 20 8"/>
        <line x1="12" y1="18" x2="12" y2="12"/>
        <line x1="9" y1="15" x2="15" y2="15"/>
      </svg>
      <h3>No jobs yet</h3>
      <p>Submit your first FASTA file to get started with annotation.</p>
      <RouterLink to="/submit" class="btn btn-primary">Submit First Job</RouterLink>
    </div>

    <!-- Jobs List -->
    <template v-else>
      <div class="jobs-list">
        <RouterLink
            v-for="job in jobs"
            :key="job.job_id"
            :to="{ name: 'job', params: { id: job.job_id } }"
            class="job-card"
        >
          <div class="job-status">
            <span class="status-dot" :style="{ backgroundColor: statusColors[job.status] }">
              <span v-if="job.status === 'processing'" class="pulse"></span>
            </span>
            <span class="status-text">{{ statusLabels[job.status] }}</span>
          </div>

          <div class="job-info">
            <span class="job-name">{{ job.filename || 'Direct Input' }}</span>
            <span class="job-id">{{ job.job_id.substring(0, 8) }}...</span>
          </div>

          <div class="job-stats">
            <span class="stat">
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                <polyline points="14 2 14 8 20 8"/>
              </svg>
              {{ job.sequence_count }} seq
            </span>
            <span v-if="job.status === 'completed'" class="stat success">
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
              {{ job.hash_matches }} matches
            </span>
          </div>

          <div class="job-time">
            {{ formatDate(job.updated_at) }}
          </div>

          <svg class="arrow" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="9 18 15 12 9 6"/>
          </svg>
        </RouterLink>
      </div>

      <!-- Pagination -->
      <div v-if="pagination && pagination.total_pages > 1" class="pagination">
        <button
            class="page-btn"
            :disabled="!pagination.has_prev"
            @click="goToPage(pagination.page - 1)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="15 18 9 12 15 6"/>
          </svg>
        </button>

        <button
            v-if="pageNumbers[0] > 1"
            class="page-btn"
            @click="goToPage(1)"
        >1</button>
        <span v-if="pageNumbers[0] > 2" class="page-ellipsis">...</span>

        <button
            v-for="page in pageNumbers"
            :key="page"
            class="page-btn"
            :class="{ active: page === pagination.page }"
            @click="goToPage(page)"
        >{{ page }}</button>

        <span v-if="pageNumbers[pageNumbers.length - 1] < pagination.total_pages - 1" class="page-ellipsis">...</span>
        <button
            v-if="pageNumbers[pageNumbers.length - 1] < pagination.total_pages"
            class="page-btn"
            @click="goToPage(pagination.total_pages)"
        >{{ pagination.total_pages }}</button>

        <button
            class="page-btn"
            :disabled="!pagination.has_next"
            @click="goToPage(pagination.page + 1)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="9 18 15 12 9 6"/>
          </svg>
        </button>

        <span class="page-info">
          {{ pagination.total_items }} total jobs
        </span>
      </div>

      <!-- Refresh Button -->
      <div class="refresh-section">
        <button @click="loadJobs(currentPage)" class="refresh-btn">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="23 4 23 10 17 10"/>
            <polyline points="1 20 1 14 7 14"/>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
          </svg>
          Refresh
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
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