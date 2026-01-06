<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { createJobWithFile, createJobWithContent } from '../api/jobs.ts'

const router = useRouter()

// State
const inputMode = ref<'file' | 'text'>('file')
const selectedFile = ref<File | null>(null)
const fastaContent = ref('')
const jobName = ref('')
const isSubmitting = ref(false)
const errorMessage = ref('')
const dragActive = ref(false)

// Example FASTA content
const exampleFasta = `>seq1_example_protein
MKFLILLFNILCLFPVLAADNHGVGPQGASGVDPITFDINSNQTGV
ASLLNFLGGTTVGSLQGKPLGQLACNPNQVKRKGDHIIYPGQQYTP
>seq2_hypothetical
MRYILAAVLLPMFAQSYKVDQTGSGPKNTFFINSNQTGVPEQYGDL
HGLNFLGGTTVGSLQGKPLGQLACNPNQVKRKGDHIIYPGQQYTPL
>seq3_membrane_protein
MKTAYIAKQRQISFVKSHFSRQLEERLGLIEVQAPILSRVGDGTQD
NLSGAEKAVQVKVKALPDAQFEVVHSLAKWKRQQIAA`

// Computed
const hasInput = computed(() => {
  if (inputMode.value === 'file') {
    return selectedFile.value !== null
  }
  return fastaContent.value.trim().length > 0
})

const sequenceCount = computed(() => {
  const content = inputMode.value === 'file'
      ? '' // Can't preview file content
      : fastaContent.value

  if (!content) return 0
  return (content.match(/^>/gm) || []).length
})

// Methods
function handleFileSelect(event: Event) {
  const input = event.target as HTMLInputElement
  if (input.files && input.files.length > 0) {
    selectFile(input.files[0])
  }
}

function selectFile(file: File) {
  const validExtensions = ['.fasta', '.fa', '.fna', '.faa', '.txt']
  const hasValidExtension = validExtensions.some(ext =>
      file.name.toLowerCase().endsWith(ext)
  )

  if (!hasValidExtension) {
    errorMessage.value = 'Invalid file format.\nSupported formats: .fasta, .fa, .fna, .faa, .txt'
    return
  }

  selectedFile.value = file
  errorMessage.value = ''
}

function handleDrop(event: DragEvent) {
  dragActive.value = false
  if (event.dataTransfer?.files && event.dataTransfer.files.length > 0) {
    selectFile(event.dataTransfer.files[0])
  }
}

function handleDragOver(event: DragEvent) {
  event.preventDefault()
  dragActive.value = true
}

function handleDragLeave() {
  dragActive.value = false
}

function clearFile() {
  selectedFile.value = null
}

function loadExample() {
  fastaContent.value = exampleFasta
  inputMode.value = 'text'
}

async function submitJob() {
  if (!hasInput.value || isSubmitting.value) return

  isSubmitting.value = true
  errorMessage.value = ''

  try {
    let response

    if (inputMode.value === 'file' && selectedFile.value) {
      response = await createJobWithFile(selectedFile.value, jobName.value || undefined)
    } else {
      response = await createJobWithContent(fastaContent.value, jobName.value || undefined)
    }

    // Navigate to job detail page
    router.push({ name: 'job', params: { id: response.job_id } })
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : 'An error occurred'
  } finally {
    isSubmitting.value = false
  }
}
</script>

<template>
  <div class="submit-page">
    <div class="page-header">
      <h2>Submit Annotation Job</h2>
      <p>Upload a FASTA file or paste your sequences directly to start annotation.</p>
    </div>

    <!-- Input Mode Tabs -->
    <div class="input-tabs">
      <button
          :class="['tab', { active: inputMode === 'file' }]"
          @click="inputMode = 'file'"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
          <polyline points="14 2 14 8 20 8"/>
          <line x1="12" y1="18" x2="12" y2="12"/>
          <line x1="9" y1="15" x2="15" y2="15"/>
        </svg>
        File Upload
      </button>
      <button
          :class="['tab', { active: inputMode === 'text' }]"
          @click="inputMode = 'text'"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="4 7 4 4 20 4 20 7"/>
          <line x1="9" y1="20" x2="15" y2="20"/>
          <line x1="12" y1="4" x2="12" y2="20"/>
        </svg>
        Paste Content
      </button>
    </div>

    <!-- File Upload Mode -->
    <div v-if="inputMode === 'file'" class="input-section">
      <div
          :class="['dropzone', { active: dragActive, 'has-file': selectedFile }]"
          @drop.prevent="handleDrop"
          @dragover="handleDragOver"
          @dragleave="handleDragLeave"
      >
        <template v-if="!selectedFile">
          <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="17 8 12 3 7 8"/>
            <line x1="12" y1="3" x2="12" y2="15"/>
          </svg>
          <p class="dropzone-text">
            Drag & Drop your FASTA file here<br>
            <span>or click to browse</span>
          </p>
          <p class="dropzone-hint">Supported formats: .fasta, .fa, .fna, .faa, .txt</p>
          <input
              type="file"
              accept=".fasta,.fa,.fna,.faa,.txt"
              @change="handleFileSelect"
              class="file-input"
          >
        </template>
        <template v-else>
          <div class="selected-file">
            <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
            </svg>
            <div class="file-info">
              <span class="file-name">{{ selectedFile.name }}</span>
              <span class="file-size">{{ (selectedFile.size / 1024).toFixed(1) }} KB</span>
            </div>
            <button @click.stop="clearFile" class="clear-btn" title="Remove file">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"/>
                <line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </div>
        </template>
      </div>
    </div>

    <!-- Text Input Mode -->
    <div v-if="inputMode === 'text'" class="input-section">
      <div class="textarea-header">
        <label for="fasta-input">FASTA Content</label>
        <button @click="loadExample" class="example-btn">Load Example</button>
      </div>
      <textarea
          id="fasta-input"
          v-model="fastaContent"
          placeholder=">sequence_id optional_description
MKFLILLFNILCLFPVLAADNHGVGPQGASGVDPITFDINSNQTGV
ASLLNFLGGTTVGSLQGKPLGQLACNPNQVKRKGDHIIYPGQQYTP

>another_sequence
MRYILAAVLLPMFAQSYKVDQTGSGPKNTFFINSNQTGVPEQYGDL"
          class="fasta-textarea"
          rows="12"
      ></textarea>
      <div v-if="sequenceCount > 0" class="sequence-count">
        {{ sequenceCount }} sequence{{ sequenceCount !== 1 ? 's' : '' }} detected
      </div>
    </div>

    <!-- Job Name (Optional) -->
    <div class="form-group">
      <label for="job-name">Job Name (optional)</label>
      <input
          type="text"
          id="job-name"
          v-model="jobName"
          placeholder="e.g., Sample_2025_01"
          class="text-input"
      >
    </div>

    <!-- Error Message -->
    <div v-if="errorMessage" class="error-message">
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="12" y1="8" x2="12" y2="12"/>
        <line x1="12" y1="16" x2="12.01" y2="16"/>
      </svg>
      {{ errorMessage }}
    </div>

    <!-- Submit Button -->
    <button
        @click="submitJob"
        :disabled="!hasInput || isSubmitting"
        :class="['submit-btn', { loading: isSubmitting }]"
    >
      <template v-if="isSubmitting">
        <span class="spinner"></span>
        Processing...
      </template>
      <template v-else>
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M22 2L11 13"/>
          <path d="M22 2L15 22L11 13L2 9L22 2Z"/>
        </svg>
        Start Job
      </template>
    </button>

    <!-- Info Box -->
    <div class="info-box">
      <h4>How it works</h4>
      <ol>
        <li>Upload your FASTA file or paste sequences directly</li>
        <li>Sequences are converted to MD5 hashes for privacy</li>
        <li>Hashes are matched against our annotation database</li>
        <li>Unmatched sequences fall back to Diamond alignment</li>
      </ol>
    </div>
  </div>
</template>

<style scoped>
.submit-page {
  max-width: 700px;
  margin: 0 auto;
}

.page-header {
  text-align: center;
  margin-bottom: 2rem;
}

.page-header h2 {
  font-size: 1.75rem;
  font-weight: 600;
  margin: 0 0 0.5rem 0;
  color: var(--color-heading);
}

.page-header p {
  color: var(--color-text);
  opacity: 0.8;
  margin: 0;
}

/* Input Tabs */
.input-tabs {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.tab {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-background);
  color: var(--color-text);
  cursor: pointer;
  transition: all 0.2s;
  font-size: 0.95rem;
}

.tab:hover {
  background: var(--color-background-soft);
}

.tab.active {
  background: hsla(160, 100%, 37%, 0.1);
  border-color: hsla(160, 100%, 37%, 1);
  color: hsla(160, 100%, 37%, 1);
}

/* Dropzone */
.input-section {
  margin-bottom: 1.5rem;
}

.dropzone {
  position: relative;
  border: 2px dashed var(--color-border);
  border-radius: 12px;
  padding: 3rem 2rem;
  text-align: center;
  transition: all 0.3s;
  cursor: pointer;
}

.dropzone:hover,
.dropzone.active {
  border-color: hsla(160, 100%, 37%, 1);
  background: hsla(160, 100%, 37%, 0.05);
}

.dropzone.has-file {
  border-style: solid;
  cursor: default;
}

.dropzone svg {
  color: var(--color-text);
  opacity: 0.5;
  margin-bottom: 1rem;
}

.dropzone-text {
  color: var(--color-heading);
  margin: 0 0 0.5rem 0;
  font-size: 1.05rem;
}

.dropzone-text span {
  color: hsla(160, 100%, 37%, 1);
}

.dropzone-hint {
  color: var(--color-text);
  opacity: 0.6;
  font-size: 0.85rem;
  margin: 0;
}

.file-input {
  position: absolute;
  inset: 0;
  opacity: 0;
  cursor: pointer;
}

/* Selected File */
.selected-file {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.selected-file svg {
  color: hsla(160, 100%, 37%, 1);
  opacity: 1;
  margin: 0;
}

.file-info {
  flex: 1;
  text-align: left;
}

.file-name {
  display: block;
  font-weight: 500;
  color: var(--color-heading);
}

.file-size {
  font-size: 0.85rem;
  color: var(--color-text);
  opacity: 0.7;
}

.clear-btn {
  padding: 0.5rem;
  border: none;
  background: transparent;
  color: var(--color-text);
  opacity: 0.6;
  cursor: pointer;
  border-radius: 6px;
  transition: all 0.2s;
}

.clear-btn:hover {
  opacity: 1;
  background: var(--color-background-mute);
}

/* Textarea */
.textarea-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.textarea-header label {
  font-weight: 500;
  color: var(--color-heading);
}

.example-btn {
  padding: 0.35rem 0.75rem;
  font-size: 0.85rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-background);
  color: var(--color-text);
  cursor: pointer;
  transition: all 0.2s;
}

.example-btn:hover {
  background: var(--color-background-soft);
}

.fasta-textarea {
  width: 100%;
  padding: 1rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-background);
  color: var(--color-text);
  font-family: 'Monaco', 'Consolas', monospace;
  font-size: 0.9rem;
  resize: vertical;
  transition: border-color 0.2s;
}

.fasta-textarea:focus {
  outline: none;
  border-color: hsla(160, 100%, 37%, 1);
}

.fasta-textarea::placeholder {
  color: var(--color-text);
  opacity: 0.4;
}

.sequence-count {
  margin-top: 0.5rem;
  font-size: 0.85rem;
  color: hsla(160, 100%, 37%, 1);
}

/* Form Group */
.form-group {
  margin-bottom: 1.5rem;
}

.form-group label {
  display: block;
  font-weight: 500;
  color: var(--color-heading);
  margin-bottom: 0.5rem;
}

.text-input {
  width: 100%;
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-background);
  color: var(--color-text);
  font-size: 0.95rem;
  transition: border-color 0.2s;
}

.text-input:focus {
  outline: none;
  border-color: hsla(160, 100%, 37%, 1);
}

/* Error Message */
.error-message {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: #ffebee;
  color: #c62828;
  border-radius: 8px;
  margin-bottom: 1.5rem;
  font-size: 0.95rem;
}

:root.dark .error-message {
  background: rgba(198, 40, 40, 0.15);
  color: #ef9a9a;
}

/* Submit Button */
.submit-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 1rem 1.5rem;
  font-size: 1rem;
  font-weight: 600;
  border: none;
  border-radius: 8px;
  background: hsla(160, 100%, 37%, 1);
  color: white;
  cursor: pointer;
  transition: all 0.3s;
}

.submit-btn:hover:not(:disabled) {
  background: hsla(160, 100%, 32%, 1);
}

.submit-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.submit-btn.loading {
  opacity: 0.8;
}

.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Info Box */
.info-box {
  margin-top: 2rem;
  padding: 1.5rem;
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  border-radius: 8px;
}

.info-box h4 {
  margin: 0 0 1rem 0;
  font-size: 1rem;
  color: var(--color-heading);
}

.info-box ol {
  margin: 0;
  padding-left: 1.25rem;
}

.info-box li {
  margin-bottom: 0.5rem;
  color: var(--color-text);
  opacity: 0.8;
  font-size: 0.95rem;
}

.info-box li:last-child {
  margin-bottom: 0;
}

/* Responsive */
@media (max-width: 600px) {
  .dropzone {
    padding: 2rem 1rem;
  }

  .input-tabs {
    flex-direction: column;
  }

  .textarea-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
  }
}
</style>
