<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRoute, useRouter, RouterLink } from 'vue-router'
import { getJob, deleteJob, type JobResponse, type JobStatus } from '../api/jobs.ts'

const route = useRoute()
const router = useRouter()

const job = ref<JobResponse | null>(null)
const loading = ref(true)
const error = ref('')
const deleting = ref(false)
let pollInterval: number | null = null

const jobId = computed(() => route.params.id as string)

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

async function loadJob() {
  try {
    job.value = await getJob(jobId.value)

    // Start polling if job is not complete
    if (job.value.status === 'pending' || job.value.status === 'processing') {
      startPolling()
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load job'
  } finally {
    loading.value = false
  }
}

function startPolling() {
  if (pollInterval) return

  pollInterval = window.setInterval(async () => {
    try {
      job.value = await getJob(jobId.value)

      if (job.value.status === 'completed' || job.value.status === 'failed') {
        stopPolling()
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
    router.push({ name: 'jobs' })
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to delete job'
    deleting.value = false
  }
}

function formatDate(dateStr: string) {
  return new Date(dateStr).toLocaleString()
}

onMounted(loadJob)
onUnmounted(stopPolling)
</script>

<template>
  <div class="job-detail">
    <!-- Loading State -->
    <div v-if="loading" class="loading-state">
      <div class="spinner-large"></div>
      <p>Loading job details...</p>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="error-state">
      <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="12" y1="8" x2="12" y2="12"/>
        <line x1="12" y1="16" x2="12.01" y2="16"/>
      </svg>
      <h3>Error</h3>
      <p>{{ error }}</p>
      <RouterLink to="/jobs" class="btn btn-secondary">Back to Jobs</RouterLink>
    </div>

    <!-- Job Details -->
    <template v-else-if="job">
      <!-- Header -->
      <div class="job-header">
        <div class="header-left">
          <RouterLink to="/jobs" class="back-link">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="19" y1="12" x2="5" y2="12"/>
              <polyline points="12 19 5 12 12 5"/>
            </svg>
            Back to Jobs
          </RouterLink>
          <h2>Job Details</h2>
        </div>
        <button
            @click="handleDelete"
            :disabled="deleting"
            class="delete-btn"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="3 6 5 6 21 6"/>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
          </svg>
          {{ deleting ? 'Deleting...' : 'Delete' }}
        </button>
      </div>

      <!-- Status Card -->
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

      <!-- Info Grid -->
      <div class="info-grid">
        <div class="info-item">
          <span class="info-label">Filename</span>
          <span class="info-value">{{ job.filename || 'Direct Input' }}</span>
        </div>
        <div class="info-item">
          <span class="info-label">Sequences</span>
          <span class="info-value">{{ job.sequence_count }}</span>
        </div>
        <div class="info-item">
          <span class="info-label">Created</span>
          <span class="info-value">{{ formatDate(job.created_at) }}</span>
        </div>
        <div class="info-item">
          <span class="info-label">Last Updated</span>
          <span class="info-value">{{ formatDate(job.updated_at) }}</span>
        </div>
      </div>

      <!-- Progress (if processing) -->
      <div v-if="job.status === 'processing'" class="progress-section">
        <h3>Progress</h3>
        <div class="progress-bar">
          <div
              class="progress-fill"
              :style="{ width: `${(job.processed_count / job.sequence_count) * 100}%` }"
          ></div>
        </div>
        <span class="progress-text">
          {{ job.processed_count }} / {{ job.sequence_count }} sequences processed
        </span>
      </div>

      <!-- Results (if completed) -->
      <div v-if="job.status === 'completed'" class="results-section">
        <h3>Results</h3>

        <div class="results-stats">
          <div class="stat-card stat-hash">
            <span class="stat-value">{{ job.hash_matches }}</span>
            <span class="stat-label">Hash Matches</span>
            <span class="stat-percent">{{ Math.round((job.hash_matches / job.sequence_count) * 100) }}%</span>
          </div>
          <div class="stat-card stat-alignment">
            <span class="stat-value">{{ job.alignment_matches }}</span>
            <span class="stat-label">Alignment Matches</span>
            <span class="stat-percent">{{ Math.round((job.alignment_matches / job.sequence_count) * 100) }}%</span>
          </div>
          <div class="stat-card stat-none">
            <span class="stat-value">{{ job.sequence_count - job.hash_matches - job.alignment_matches }}</span>
            <span class="stat-label">No Match</span>
            <span class="stat-percent">{{ Math.round(((job.sequence_count - job.hash_matches - job.alignment_matches) / job.sequence_count) * 100) }}%</span>
          </div>
        </div>

        <!-- Sequence Table -->
        <div v-if="job.sequences && job.sequences.length > 0" class="sequences-table">
          <h4>Sequence Details</h4>
          <div class="table-wrapper">
            <table>
              <thead>
              <tr>
                <th>ID</th>
                <th>Length</th>
                <th>MD5 Hash</th>
                <th>Annotation</th>
                <th>Source</th>
              </tr>
              </thead>
              <tbody>
              <tr v-for="seq in job.sequences" :key="seq.id">
                <td class="seq-id">{{ seq.id }}</td>
                <td>{{ seq.length }}</td>
                <td class="hash">{{ seq.md5_hash.substring(0, 12) }}...</td>
                <td>{{ seq.annotation || '-' }}</td>
                <td>
                    <span v-if="seq.annotation_source" :class="['source-badge', seq.annotation_source]">
                      {{ seq.annotation_source === 'hash_match' ? 'Hash' : 'Alignment' }}
                    </span>
                  <span v-else class="source-badge none">None</span>
                </td>
              </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <!-- Error Message (if failed) -->
      <div v-if="job.status === 'failed'" class="error-section">
        <h3>Error</h3>
        <p class="error-message">{{ job.error_message || 'An unknown error occurred during processing.' }}</p>
      </div>

      <!-- Actions -->
      <div class="actions">
        <RouterLink to="/submit" class="btn btn-primary">Submit New Job</RouterLink>
      </div>
    </template>
  </div>
</template>

<style scoped>
.job-detail {
  max-width: 900px;
  margin: 0 auto;
}

/* Loading & Error States */
.loading-state,
.error-state {
  text-align: center;
  padding: 4rem 2rem;
}

.spinner-large {
  width: 48px;
  height: 48px;
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

.error-state h3 {
  margin: 0 0 0.5rem 0;
  color: var(--color-heading);
}

.error-state p {
  color: var(--color-text);
  opacity: 0.8;
  margin-bottom: 1.5rem;
}

/* Header */
.job-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 1.5rem;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.back-link {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  color: var(--color-text);
  text-decoration: none;
  font-size: 0.9rem;
  opacity: 0.8;
  transition: opacity 0.2s;
}

.back-link:hover {
  opacity: 1;
}

.job-header h2 {
  margin: 0;
  font-size: 1.5rem;
  color: var(--color-heading);
}

.delete-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  border: 1px solid #f44336;
  border-radius: 6px;
  background: transparent;
  color: #f44336;
  cursor: pointer;
  font-size: 0.9rem;
  transition: all 0.2s;
}

.delete-btn:hover:not(:disabled) {
  background: #f44336;
  color: white;
}

.delete-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Status Card */
.status-card {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem 1.5rem;
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  border-radius: 12px;
  margin-bottom: 1.5rem;
}

.status-indicator {
  position: relative;
  width: 12px;
  height: 12px;
  border-radius: 50%;
}

.pulse {
  position: absolute;
  inset: -4px;
  border-radius: 50%;
  background: inherit;
  animation: pulse 1.5s ease-out infinite;
}

@keyframes pulse {
  0% { opacity: 0.8; transform: scale(1); }
  100% { opacity: 0; transform: scale(2); }
}

.status-info {
  flex: 1;
}

.status-label {
  display: block;
  font-weight: 600;
  color: var(--color-heading);
}

.job-id {
  font-size: 0.85rem;
  color: var(--color-text);
  opacity: 0.7;
  font-family: monospace;
}

.processing-spinner .spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--color-border);
  border-top-color: #2196f3;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

/* Info Grid */
.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 1rem;
  margin-bottom: 2rem;
}

.info-item {
  padding: 1rem;
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  border-radius: 8px;
}

.info-label {
  display: block;
  font-size: 0.8rem;
  text-transform: uppercase;
  color: var(--color-text);
  opacity: 0.6;
  margin-bottom: 0.25rem;
}

.info-value {
  font-weight: 500;
  color: var(--color-heading);
  word-break: break-word;
}

/* Progress */
.progress-section {
  margin-bottom: 2rem;
}

.progress-section h3 {
  margin: 0 0 1rem 0;
  font-size: 1.1rem;
  color: var(--color-heading);
}

.progress-bar {
  height: 8px;
  background: var(--color-border);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 0.5rem;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #2196f3, #4caf50);
  border-radius: 4px;
  transition: width 0.3s;
}

.progress-text {
  font-size: 0.85rem;
  color: var(--color-text);
  opacity: 0.8;
}

/* Results */
.results-section h3 {
  margin: 0 0 1rem 0;
  font-size: 1.1rem;
  color: var(--color-heading);
}

.results-stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 1rem;
  margin-bottom: 2rem;
}

.stat-card {
  padding: 1.25rem;
  border-radius: 12px;
  text-align: center;
}

.stat-hash {
  background: rgba(76, 175, 80, 0.1);
  border: 1px solid rgba(76, 175, 80, 0.3);
}

.stat-alignment {
  background: rgba(33, 150, 243, 0.1);
  border: 1px solid rgba(33, 150, 243, 0.3);
}

.stat-none {
  background: rgba(158, 158, 158, 0.1);
  border: 1px solid rgba(158, 158, 158, 0.3);
}

.stat-value {
  display: block;
  font-size: 2rem;
  font-weight: 600;
  color: var(--color-heading);
}

.stat-label {
  display: block;
  font-size: 0.85rem;
  color: var(--color-text);
  margin: 0.25rem 0;
}

.stat-percent {
  font-size: 0.9rem;
  color: var(--color-text);
  opacity: 0.6;
}

/* Sequences Table */
.sequences-table {
  margin-top: 2rem;
}

.sequences-table h4 {
  margin: 0 0 1rem 0;
  font-size: 1rem;
  color: var(--color-heading);
}

.table-wrapper {
  overflow-x: auto;
  border: 1px solid var(--color-border);
  border-radius: 8px;
}

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.9rem;
}

th, td {
  padding: 0.75rem 1rem;
  text-align: left;
  border-bottom: 1px solid var(--color-border);
}

th {
  background: var(--color-background-soft);
  font-weight: 600;
  color: var(--color-heading);
}

tr:last-child td {
  border-bottom: none;
}

.seq-id {
  font-family: monospace;
  color: hsla(160, 100%, 37%, 1);
}

.hash {
  font-family: monospace;
  font-size: 0.85rem;
  color: var(--color-text);
  opacity: 0.8;
}

.source-badge {
  display: inline-block;
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 500;
}

.source-badge.hash_match {
  background: rgba(76, 175, 80, 0.15);
  color: #4caf50;
}

.source-badge.alignment {
  background: rgba(33, 150, 243, 0.15);
  color: #2196f3;
}

.source-badge.none {
  background: rgba(158, 158, 158, 0.15);
  color: #9e9e9e;
}

/* Error Section */
.error-section {
  background: rgba(244, 67, 54, 0.1);
  border: 1px solid rgba(244, 67, 54, 0.3);
  border-radius: 8px;
  padding: 1.5rem;
  margin-bottom: 2rem;
}

.error-section h3 {
  margin: 0 0 0.5rem 0;
  color: #f44336;
}

.error-section .error-message {
  margin: 0;
  color: var(--color-text);
}

/* Actions */
.actions {
  margin-top: 2rem;
  text-align: center;
}

.btn {
  padding: 0.75rem 1.5rem;
  font-size: 0.95rem;
  font-weight: 500;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.3s;
  text-decoration: none;
  display: inline-block;
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

/* Responsive */
@media (max-width: 600px) {
  .job-header {
    flex-direction: column;
    gap: 1rem;
  }

  .delete-btn {
    align-self: flex-start;
  }

  .info-grid {
    grid-template-columns: 1fr;
  }

  .results-stats {
    grid-template-columns: 1fr;
  }
}
</style>
