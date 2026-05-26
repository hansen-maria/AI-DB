<script setup lang="ts">
import type { BaktaAnnotationSummary, IngestResponse, SequenceType } from '../../api/bakta.ts'

defineProps<{
  unmatchedCount:        number
  sequenceType:          SequenceType
  show:                  boolean
  analyzing:             boolean
  progressLabel:         string
  progressPercent:       number
  error:                 string
  result:                BaktaAnnotationSummary | null
  abortController:       AbortController | null
  genus:                 string
  species:               string
  completeGenome:        boolean
  ingesting:             boolean
  ingestResult:          IngestResponse | null
  ingestError:           string
  groupFeaturesByType:   (features: any[]) => Record<string, number>
}>()

const emit = defineEmits<{
  'update:show':           [value: boolean]
  'update:genus':          [value: string]
  'update:species':        [value: string]
  'update:completeGenome': [value: boolean]
  'analyze':               []
  'ingest':                []
  'reset':                 []
}>()
</script>

<template>
  <div v-if="unmatchedCount > 0" class="bakta-panel">
    <!-- Collapsible header -->
    <div class="bakta-header" @click="emit('update:show', !show)">
      <div class="bakta-title">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"
             fill="none" stroke="currentColor" stroke-width="2">
          <path d="M3 3c9 0 9 18 18 18"/><path d="M21 3C12 3 12 21 3 21"/>
          <path d="M7 8h4"/><path d="M13 16h4"/>
          <path d="M7.5 12H10"/><path d="M14 12h2.5"/>
        </svg>
        <span>Analyze {{ unmatchedCount }} unmatched sequences with Bakta</span>
        <span v-if="result"    class="bakta-badge bakta-badge--done">✓ Done</span>
        <span v-else-if="analyzing" class="bakta-badge bakta-badge--running">Running…</span>
      </div>
      <svg :class="{ rotated: show }" xmlns="http://www.w3.org/2000/svg" width="20" height="20"
           viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="6 9 12 15 18 9"/>
      </svg>
    </div>

    <div v-if="show" class="bakta-content">
      <!-- Description -->
      <p class="bakta-description">
        <a href="https://bakta.computational.bio" target="_blank">Bakta</a>
        is a rapid, standardized annotation tool for bacterial genomes and proteins.
        <span v-if="sequenceType === 'protein'">
          Sequences detected as <strong>protein</strong> — using the
          <strong>bakta_proteins</strong> workflow (V2 API). No additional configuration needed.
        </span>
        <span v-else>
          Sequences detected as <strong>nucleotide</strong> — using the full
          <strong>genome annotation</strong> workflow (V1 API).
        </span>
      </p>

      <!-- Config form (only before first run) -->
      <div v-if="!analyzing && !result" class="bakta-form">
        <template v-if="sequenceType === 'nucleotide'">
          <div class="bakta-form-row">
            <div class="bakta-field">
              <label>Genus <span class="bakta-optional">(optional)</span></label>
              <input :value="genus" type="text" placeholder="e.g. Escherichia" class="bakta-input"
                     @input="emit('update:genus', ($event.target as HTMLInputElement).value)" />
            </div>
            <div class="bakta-field">
              <label>Species <span class="bakta-optional">(optional)</span></label>
              <input :value="species" type="text" placeholder="e.g. coli" class="bakta-input"
                     @input="emit('update:species', ($event.target as HTMLInputElement).value)" />
            </div>
            <div class="bakta-field bakta-field--inline">
              <label class="bakta-checkbox-label">
                <input :checked="completeGenome" type="checkbox"
                       @change="emit('update:completeGenome', ($event.target as HTMLInputElement).checked)" />
                Complete genome
              </label>
            </div>
          </div>
        </template>
        <p v-else class="bakta-note" style="margin-bottom:0.5rem">
          The bakta_proteins workflow requires no additional configuration.
        </p>

        <button class="btn btn-bakta" :disabled="unmatchedCount === 0" @click="emit('analyze')">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
               fill="none" stroke="currentColor" stroke-width="2">
            <polygon points="5 3 19 12 5 21 5 3"/>
          </svg>
          Run Bakta Annotation ({{ unmatchedCount }} sequences)
        </button>
      </div>

      <!-- Progress -->
      <div v-if="analyzing" class="bakta-progress">
        <div class="bakta-progress-label">{{ progressLabel }}</div>
        <div class="bakta-progress-bar">
          <div class="bakta-progress-fill" :style="{ width: progressPercent + '%' }"></div>
        </div>
        <span class="bakta-progress-pct">{{ progressPercent }}%</span>
        <p class="bakta-note">
          <span v-if="sequenceType === 'protein'">Protein annotation typically takes 2–5 minutes.</span>
          <span v-else>Genome annotation typically takes 10–15 minutes.</span>
        </p>
        <button class="btn btn-secondary-psos" style="margin-top:0.5rem"
                @click="abortController?.abort()">Cancel</button>
      </div>

      <!-- Error -->
      <div v-if="error && !analyzing" class="psos-error">
        <pre class="bakta-error-text">{{ error }}</pre>
        <button class="btn btn-secondary-psos" style="margin-top:0.5rem" @click="emit('analyze')">Retry</button>
      </div>

      <!-- Results -->
      <div v-if="result && !analyzing" class="bakta-results">
        <!-- Web viewer link -->
        <a :href="result.webViewerUrl" target="_blank" class="bakta-viewer-link">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
               fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
            <polyline points="15 3 21 3 21 9"/>
            <line x1="10" y1="14" x2="21" y2="3"/>
          </svg>
          Open in Bakta Web Viewer →
        </a>

        <!-- Stats grid -->
        <div v-if="result.stats" class="bakta-stats-grid">
          <div v-if="result.stats.no_cdss        !== undefined" class="bakta-stat"><span class="bakta-stat__value">{{ result.stats.no_cdss }}</span><span class="bakta-stat__label">CDSs</span></div>
          <div v-if="result.stats.no_hypotheticals !== undefined" class="bakta-stat"><span class="bakta-stat__value">{{ result.stats.no_hypotheticals }}</span><span class="bakta-stat__label">Hypotheticals</span></div>
          <div v-if="result.stats.no_trnas        !== undefined" class="bakta-stat"><span class="bakta-stat__value">{{ result.stats.no_trnas }}</span><span class="bakta-stat__label">tRNAs</span></div>
          <div v-if="result.stats.no_rrnas        !== undefined" class="bakta-stat"><span class="bakta-stat__value">{{ result.stats.no_rrnas }}</span><span class="bakta-stat__label">rRNAs</span></div>
          <div v-if="result.stats.no_ncrnas       !== undefined" class="bakta-stat"><span class="bakta-stat__value">{{ result.stats.no_ncrnas }}</span><span class="bakta-stat__label">ncRNAs</span></div>
          <div v-if="result.stats.no_pseudogenes  !== undefined" class="bakta-stat"><span class="bakta-stat__value">{{ result.stats.no_pseudogenes }}</span><span class="bakta-stat__label">Pseudogenes</span></div>
          <div v-if="result.stats.gc              !== undefined" class="bakta-stat"><span class="bakta-stat__value">{{ (result.stats.gc * 100).toFixed(1) }}%</span><span class="bakta-stat__label">GC content</span></div>
          <div v-if="result.stats.size            !== undefined" class="bakta-stat"><span class="bakta-stat__value">{{ Math.round(result.stats.size / 1000) }} kb</span><span class="bakta-stat__label">Genome size</span></div>
        </div>

        <!-- Feature type breakdown -->
        <div v-if="result.features?.length" class="bakta-feature-types">
          <h4 class="bakta-section-title">Feature types</h4>
          <div class="bakta-feature-grid">
            <div v-for="[type, count] in Object.entries(groupFeaturesByType(result.features)).sort((a, b) => (b[1] as number) - (a[1] as number))"
                 :key="type" class="bakta-feature-type">
              <span class="bakta-feature-type__name">{{ type }}</span>
              <span class="bakta-feature-type__count">{{ count }}</span>
            </div>
          </div>
        </div>

        <!-- Download links – nucleotide (V1) -->
        <div v-if="result.resultFilesNucleotide" class="bakta-downloads">
          <h4 class="bakta-section-title">Download results</h4>
          <div class="bakta-download-links">
            <a v-if="result.resultFilesNucleotide.TSV"      :href="result.resultFilesNucleotide.TSV"      target="_blank" class="bakta-dl-link">TSV</a>
            <a v-if="result.resultFilesNucleotide.GFF3"     :href="result.resultFilesNucleotide.GFF3"     target="_blank" class="bakta-dl-link">GFF3</a>
            <a v-if="result.resultFilesNucleotide.GBFF"     :href="result.resultFilesNucleotide.GBFF"     target="_blank" class="bakta-dl-link">GenBank</a>
            <a v-if="result.resultFilesNucleotide.FAA"      :href="result.resultFilesNucleotide.FAA"      target="_blank" class="bakta-dl-link">FAA (proteins)</a>
            <a v-if="result.resultFilesNucleotide.FFN"      :href="result.resultFilesNucleotide.FFN"      target="_blank" class="bakta-dl-link">FFN (genes)</a>
            <a v-if="result.resultFilesNucleotide.FNA"      :href="result.resultFilesNucleotide.FNA"      target="_blank" class="bakta-dl-link">FNA</a>
            <a v-if="result.resultFilesNucleotide.EMBL"     :href="result.resultFilesNucleotide.EMBL"     target="_blank" class="bakta-dl-link">EMBL</a>
            <a v-if="result.resultFilesNucleotide.JSON"     :href="result.resultFilesNucleotide.JSON"     target="_blank" class="bakta-dl-link">JSON</a>
            <a v-if="result.resultFilesNucleotide.TXTLogs"  :href="result.resultFilesNucleotide.TXTLogs"  target="_blank" class="bakta-dl-link">Log</a>
          </div>
        </div>

        <!-- Download links – protein (V2) -->
        <div v-if="result.resultFilesProtein" class="bakta-downloads">
          <h4 class="bakta-section-title">Download results</h4>
          <div class="bakta-download-links">
            <a v-if="result.resultFilesProtein.tsv"               :href="result.resultFilesProtein.tsv"               target="_blank" class="bakta-dl-link">TSV</a>
            <a v-if="result.resultFilesProtein.faa"               :href="result.resultFilesProtein.faa"               target="_blank" class="bakta-dl-link">FAA (proteins)</a>
            <a v-if="result.resultFilesProtein.hypotheticals_tsv" :href="result.resultFilesProtein.hypotheticals_tsv" target="_blank" class="bakta-dl-link">Hypotheticals TSV</a>
            <a v-if="result.resultFilesProtein.json"              :href="result.resultFilesProtein.json"              target="_blank" class="bakta-dl-link">JSON</a>
          </div>
        </div>

        <!-- Ingest into AI-DB DB -->
        <div class="bakta-ingest">
          <h4 class="bakta-section-title">Add to AI-DB annotations DB</h4>
          <p class="bakta-description" style="margin:0 0 0.6rem">
            Write Bakta annotations back into the local AI-DB annotations DB so future jobs
            recognize these sequences via hash lookup — without re-running Bakta.
            Annotated proteins populate <code>ups</code>, <code>ips</code> and <code>psc</code>;
            hypothetical proteins are stored in <code>ups</code> only.
          </p>

          <div v-if="ingestResult" class="bakta-ingest-result">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
                 fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <span>
              <strong>{{ ingestResult.ingested }}</strong> new sequences added,
              <strong>{{ ingestResult.updated }}</strong> existing entries updated
              ({{ ingestResult.total }} total)
            </span>
          </div>

          <div v-else-if="ingestError" class="bakta-error" style="margin-bottom:0.5rem">
            <pre class="bakta-error-text">{{ ingestError }}</pre>
          </div>

          <button v-if="!ingestResult" class="btn btn-bakta" :disabled="ingesting"
                  style="margin-top:0.25rem" @click="emit('ingest')">
            <svg v-if="ingesting" xmlns="http://www.w3.org/2000/svg" width="16" height="16"
                 viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                 style="animation:spin 1s linear infinite">
              <path d="M21 12a9 9 0 1 1-6.219-8.56"/>
            </svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" width="16" height="16"
                 viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
              <polyline points="7 10 12 15 17 10"/>
              <line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
            {{ ingesting ? 'Ingesting…' : `Add ${result?.featureCount ?? result?.features?.length ?? 0} features to AI-DB annotations DB` }}
          </button>
        </div>

        <!-- Re-run -->
        <button class="btn btn-secondary-psos" style="margin-top:0.5rem;align-self:flex-start"
                @click="emit('reset')">
          Re-run with different settings
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
  svg.rotated { transform: rotate(180deg); transition: transform 0.2s; }
  .bakta-panel { margin-top: 2rem; border: 1px solid var(--color-border); border-radius: 12px; overflow: hidden; }
  .bakta-header { display: flex; justify-content: space-between; align-items: center; padding: 1rem 1.25rem; background: var(--color-background-soft); cursor: pointer; user-select: none; }
  .bakta-header:hover { background: var(--color-background-mute); }
  .bakta-title { display: flex; align-items: center; gap: 0.75rem; font-weight: 600; color: var(--color-heading); }
  .bakta-badge { padding: 0.15rem 0.5rem; border-radius: 99px; font-size: 0.75rem; font-weight: 600; }
  .bakta-badge--done    { background: rgba(76,175,80,0.15); color: #2e7d32; }
  .bakta-badge--running { background: rgba(33,150,243,0.15); color: #1565c0; }
  .bakta-content { padding: 1.25rem; display: flex; flex-direction: column; gap: 1rem; }
  .bakta-description { margin: 0; color: var(--color-text); font-size: 0.9rem; line-height: 1.6; }
  .bakta-form { display: flex; flex-direction: column; gap: 1rem; }
  .bakta-form-row { display: flex; gap: 1rem; flex-wrap: wrap; }
  .bakta-field { display: flex; flex-direction: column; gap: 0.35rem; flex: 1; min-width: 140px; }
  .bakta-field label { font-size: 0.85rem; font-weight: 500; color: var(--color-heading); }
  .bakta-optional { font-weight: 400; opacity: 0.6; font-size: 0.8rem; }
  .bakta-input { padding: 0.5rem 0.75rem; border: 1px solid var(--color-border); border-radius: 6px; background: var(--color-background); color: var(--color-text); font-size: 0.9rem; }
  .bakta-field--inline { justify-content: flex-end; }
  .bakta-checkbox-label { display: flex; align-items: center; gap: 0.5rem; font-size: 0.875rem; cursor: pointer; }
  .btn-bakta { display: inline-flex; align-items: center; gap: 0.5rem; padding: 0.6rem 1.2rem; background: #e08000; color: white; border: none; border-radius: 8px; font-size: 0.9rem; font-weight: 500; cursor: pointer; transition: background 0.2s; }
  .btn-bakta:hover:not(:disabled) { background: #c07000; }
  .btn-bakta:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-secondary-psos { display: inline-flex; align-items: center; gap: 0.5rem; padding: 0.5rem 1rem; background: transparent; color: var(--color-text); border: 1px solid var(--color-border); border-radius: 8px; font-size: 0.875rem; cursor: pointer; transition: all 0.2s; }
  .btn-secondary-psos:hover { border-color: hsla(160,100%,37%,0.5); }
  .bakta-progress { display: flex; flex-direction: column; gap: 0.5rem; }
  .bakta-progress-label { font-size: 0.9rem; color: var(--color-heading); }
  .bakta-progress-bar { height: 8px; background: var(--color-background-mute); border-radius: 4px; overflow: hidden; }
  .bakta-progress-fill { height: 100%; background: #e08000; border-radius: 4px; transition: width 0.3s; }
  .bakta-progress-pct { font-size: 0.85rem; color: var(--color-text); }
  .bakta-note { margin: 0; font-size: 0.85rem; color: var(--color-text); opacity: 0.7; }
  .psos-error { background: rgba(244,67,54,0.1); border: 1px solid rgba(244,67,54,0.3); color: #f44336; padding: 0.75rem; border-radius: 8px; }
  .bakta-error-text { margin: 0; white-space: pre-wrap; word-break: break-word; font-family: monospace; font-size: 0.8rem; line-height: 1.5; }
  .bakta-results { display: flex; flex-direction: column; gap: 1rem; }
  .bakta-viewer-link { display: inline-flex; align-items: center; gap: 0.5rem; color: #028090; text-decoration: none; font-weight: 500; }
  .bakta-viewer-link:hover { text-decoration: underline; }
  .bakta-stats-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 0.75rem; }
  .bakta-stat { display: flex; flex-direction: column; align-items: center; padding: 0.75rem; background: var(--color-background-soft); border-radius: 8px; border: 1px solid var(--color-border); }
  .bakta-stat__value { font-size: 1.5rem; font-weight: 700; color: var(--color-heading); }
  .bakta-stat__label { font-size: 0.75rem; color: var(--color-text); opacity: 0.7; margin-top: 0.15rem; }
  .bakta-section-title { margin: 0 0 0.5rem; font-size: 0.9rem; font-weight: 600; color: var(--color-heading); }
  .bakta-feature-types { border: 1px solid var(--color-border); border-radius: 8px; padding: 0.75rem; }
  .bakta-feature-grid { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .bakta-feature-type { display: flex; align-items: center; gap: 0.4rem; padding: 0.25rem 0.6rem; background: var(--color-background-soft); border-radius: 6px; font-size: 0.8rem; }
  .bakta-feature-type__name { color: var(--color-text); }
  .bakta-feature-type__count { font-weight: 600; color: var(--color-heading); }
  .bakta-downloads { border: 1px solid var(--color-border); border-radius: 8px; padding: 0.75rem; }
  .bakta-download-links { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .bakta-dl-link { display: inline-block; padding: 0.3rem 0.75rem; border: 1px solid var(--color-border); border-radius: 6px; color: var(--color-text); text-decoration: none; font-size: 0.82rem; transition: all 0.15s; }
  .bakta-dl-link:hover { border-color: #028090; color: #028090; }
  .bakta-ingest { border: 1px solid var(--color-border); border-radius: 8px; padding: 0.75rem; }
  .bakta-ingest-result { display: flex; align-items: center; gap: 0.5rem; padding: 0.5rem 0.75rem; background: rgba(56,161,105,0.1); border: 1px solid rgba(56,161,105,0.3); border-radius: 6px; font-size: 0.875rem; color: var(--color-text); }
  .bakta-ingest-result svg { color: #38a169; flex-shrink: 0; }
  .bakta-error { background: rgba(244,67,54,0.1); border: 1px solid rgba(244,67,54,0.3); padding: 0.5rem; border-radius: 6px; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
