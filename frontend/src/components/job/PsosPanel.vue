<script setup lang="ts">
import type { PsosProfile, PsosAnnotation } from '../../api/psos.ts'
import type { psosProfiles } from '../../api/psos.ts'

// All props mirror the state returned by usePsosAnalysis
defineProps<{
  unmatchedCount:      number
  selectedProfile:     PsosProfile
  profiles:            typeof psosProfiles
  analyzing:           boolean
  progress:            number
  total:               number
  error:               string
  results:             Map<string, PsosAnnotation>
  copied:              boolean
  show:                boolean
}>()

const emit = defineEmits<{
  'update:show':            [value: boolean]
  'update:selectedProfile': [value: PsosProfile]
  'analyze':                []
  'open-in-psos':           []
  'download-fasta':         []
  'download-tsv':           []
}>()

function getPsosJobUrl(id: string | undefined) {
  if (!id) return '#'
  return `https://psos.computational.bio/jobs/${id}`
}
</script>

<template>
  <div v-if="unmatchedCount > 0" class="psos-panel">
    <!-- Collapsible header -->
    <div class="psos-header" @click="emit('update:show', !show)">
      <div class="psos-title">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"
             fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
          <line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
        <span>Analyze {{ unmatchedCount }} unmatched sequences with Psos</span>
      </div>
      <svg :class="{ rotated: show }" xmlns="http://www.w3.org/2000/svg" width="20" height="20"
           viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="6 9 12 15 18 9"/>
      </svg>
    </div>

    <div v-if="show" class="psos-content">
      <p class="psos-description">
        <a href="https://psos.computational.bio" target="_blank">Psos</a>
        (Protein Sequence Observation Service) can analyze sequences that didn't match in the Bakta
        database. It provides signal peptide prediction, transmembrane domain detection, and
        subcellular localization.
      </p>

      <!-- Controls -->
      <div class="psos-controls">
        <div class="psos-profile-select">
          <label>Organism Profile:</label>
          <select
            :value="selectedProfile"
            :disabled="analyzing"
            @change="emit('update:selectedProfile', ($event.target as HTMLSelectElement).value as PsosProfile)"
          >
            <option v-for="p in profiles" :key="p.value" :value="p.value">{{ p.label }}</option>
          </select>
        </div>

        <div class="psos-buttons">
          <button class="btn btn-psos" :disabled="analyzing || unmatchedCount === 0" @click="emit('analyze')">
            <svg v-if="!analyzing" xmlns="http://www.w3.org/2000/svg" width="16" height="16"
                 viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polygon points="5 3 19 12 5 21 5 3"/>
            </svg>
            <div v-else class="spinner-small"></div>
            {{ analyzing ? `Analyzing ${progress}/${total}…` : 'Analyze with Psos' }}
          </button>

          <button class="btn btn-secondary-psos" :disabled="analyzing || unmatchedCount === 0"
                  title="Copy sequences to clipboard and open Psos" @click="emit('open-in-psos')">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
                 fill="none" stroke="currentColor" stroke-width="2">
              <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
              <polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/>
            </svg>
            {{ copied ? 'Copied!' : 'Open Psos' }}
          </button>

          <button class="btn btn-secondary-psos" :disabled="analyzing || unmatchedCount === 0"
                  title="Download FASTA for manual upload" @click="emit('download-fasta')">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
                 fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
              <polyline points="7 10 12 15 17 10"/>
              <line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
            FASTA
          </button>
        </div>
      </div>

      <!-- Error -->
      <div v-if="error" class="psos-error">
        {{ error }}
        <p class="psos-error-hint">Try using "Open Psos" to manually analyze sequences.</p>
      </div>

      <!-- Progress bar -->
      <div v-if="analyzing" class="psos-progress">
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: `${(progress / total) * 100}%` }"></div>
        </div>
        <span class="progress-text">{{ progress }} of {{ total }} sequences analyzed</span>
      </div>

      <!-- Results table -->
      <div v-if="results.size > 0" class="psos-results">
        <div class="psos-results-header">
          <h4>Psos Results ({{ results.size }} sequences analyzed)</h4>
          <button class="btn btn-download-psos" @click="emit('download-tsv')">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24"
                 fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
              <polyline points="7 10 12 15 17 10"/>
              <line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
            Download TSV
          </button>
        </div>
        <p class="psos-results-hint">
          Click "View in Psos" for detailed visualizations including signal peptide plots,
          transmembrane topology, and homology search results.
        </p>

        <div class="psos-results-table">
          <table>
            <thead>
              <tr>
                <th>Sequence ID</th>
                <th>Protein Name / Best Hit</th>
                <th>Features</th>
                <th>Details</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="[seqId, result] in results" :key="seqId">
                <td class="seq-id">{{ seqId }}</td>
                <td>
                  <div v-if="result.proteinName || result.bestHit" class="homology-hit">
                    <span v-if="result.proteinName" class="protein-name">{{ result.proteinName }}</span>
                    <span v-if="result.bestHit" class="hit-stats">
                      {{ result.bestHit.dbxref }} ·
                      {{ result.bestHit.percentIdentity.toFixed(1) }}% ·
                      E={{ result.bestHit.evalue.toExponential(1) }}
                    </span>
                  </div>
                  <span v-else class="no-data">No significant hits</span>
                </td>
                <td class="psos-features">
                  <span v-if="result.hasSignalPeptide" class="feature-badge signal">Signal Peptide</span>
                  <span v-if="result.transmembraneCount" class="feature-badge tm">{{ result.transmembraneCount }} TM</span>
                  <span v-if="!result.hasSignalPeptide && !result.transmembraneCount" class="no-data">-</span>
                </td>
                <td>
                  <a :href="getPsosJobUrl(result.psosJobId)" target="_blank" class="psos-link">
                    View in Psos →
                  </a>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  /* Chevron rotation */
  svg.rotated { transform: rotate(180deg); transition: transform 0.2s; }

  /* All psos-* styles from the original main file */
  .psos-panel {
    margin-top: 2rem;
    border: 1px solid var(--color-border);
    border-radius: 12px;
    overflow: hidden;
  }
  .psos-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 1rem 1.25rem;
    background: var(--color-background-soft);
    cursor: pointer;
    user-select: none;
  }
  .psos-header:hover { background: var(--color-background-mute); }
  .psos-title {
    display: flex; align-items: center; gap: 0.75rem;
    font-weight: 600; color: var(--color-heading);
  }
  .psos-content { padding: 1.25rem; display: flex; flex-direction: column; gap: 1rem; }
  .psos-description { margin: 0; color: var(--color-text); font-size: 0.9rem; line-height: 1.6; }
  .psos-controls { display: flex; gap: 1rem; align-items: flex-end; flex-wrap: wrap; }
  .psos-profile-select { display: flex; flex-direction: column; gap: 0.25rem; }
  .psos-profile-select label { font-size: 0.85rem; font-weight: 500; color: var(--color-heading); }
  .psos-profile-select select {
    padding: 0.5rem 0.75rem; border: 1px solid var(--color-border);
    border-radius: 6px; background: var(--color-background); color: var(--color-text); font-size: 0.9rem;
  }
  .psos-buttons { display: flex; gap: 0.5rem; flex-wrap: wrap; }
  .btn-psos {
    display: inline-flex; align-items: center; gap: 0.5rem;
    padding: 0.6rem 1.2rem; background: #028090; color: white;
    border: none; border-radius: 8px; font-size: 0.9rem; font-weight: 500;
    cursor: pointer; transition: background 0.2s;
  }
  .btn-psos:hover:not(:disabled) { background: #016070; }
  .btn-psos:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-secondary-psos {
    display: inline-flex; align-items: center; gap: 0.5rem;
    padding: 0.6rem 1rem; background: transparent; color: var(--color-text);
    border: 1px solid var(--color-border); border-radius: 8px;
    font-size: 0.9rem; cursor: pointer; transition: all 0.2s;
  }
  .btn-secondary-psos:hover:not(:disabled) { border-color: hsla(160,100%,37%,0.5); }
  .btn-secondary-psos:disabled { opacity: 0.5; cursor: not-allowed; }
  .psos-error {
    background: rgba(244,67,54,0.1); border: 1px solid rgba(244,67,54,0.3);
    color: #f44336; padding: 0.75rem; border-radius: 8px; font-size: 0.9rem;
  }
  .psos-error-hint { margin: 0.5rem 0 0; font-size: 0.85rem; }
  .psos-progress { display: flex; flex-direction: column; gap: 0.5rem; }
  .psos-results-header { display: flex; justify-content: space-between; align-items: center; }
  .psos-results-header h4 { margin: 0; color: var(--color-heading); }
  .btn-download-psos {
    display: inline-flex; align-items: center; gap: 0.35rem;
    padding: 0.4rem 0.8rem; border: 1px solid var(--color-border);
    border-radius: 6px; background: transparent; color: var(--color-text);
    font-size: 0.8rem; cursor: pointer; transition: all 0.2s;
  }
  .btn-download-psos:hover { border-color: hsla(160,100%,37%,0.5); }
  .psos-results-hint { margin: 0; font-size: 0.85rem; color: var(--color-text); opacity: 0.7; }
  .psos-results-table { overflow-x: auto; }
  .psos-results-table table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
  .psos-results-table th { text-align: left; padding: 0.5rem 0.75rem; border-bottom: 1px solid var(--color-border); font-size: 0.8rem; color: var(--color-text); opacity: 0.7; white-space: nowrap; }
  .psos-results-table td { padding: 0.5rem 0.75rem; border-bottom: 1px solid var(--color-border); }
  .homology-hit { display: flex; flex-direction: column; gap: 0.2rem; }
  .protein-name { font-weight: 500; color: var(--color-heading); font-size: 0.85rem; }
  .hit-stats { font-size: 0.78rem; color: var(--color-text); opacity: 0.75; font-family: monospace; }
  .psos-features { display: flex; gap: 0.35rem; flex-wrap: wrap; }
  .feature-badge { display: inline-block; padding: 0.15rem 0.5rem; border-radius: 99px; font-size: 0.75rem; font-weight: 500; }
  .feature-badge.signal { background: rgba(33,150,243,0.12); color: #1565c0; }
  .feature-badge.tm     { background: rgba(156,39,176,0.12); color: #6a1b9a; }
  .psos-link { color: #028090; text-decoration: none; font-size: 0.85rem; white-space: nowrap; }
  .psos-link:hover { text-decoration: underline; }
  .no-data { color: var(--color-text); opacity: 0.35; }
  .seq-id { font-family: monospace; font-size: 0.8rem; color: var(--color-text); }
  .spinner-small {
    width: 16px; height: 16px;
    border: 2px solid rgba(255,255,255,0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
