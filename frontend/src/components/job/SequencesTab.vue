<script setup lang="ts">
import { filterOptions, cogCategories, ecClasses, getUniRef100Url, getUniParcUrl, getNcbiUrl, hasAnnotationLinks } from '../../constants/sequences.ts'
import type { SequenceFilter, FilteredDownloadFormat } from '../../api/jobs.ts'
import PsosPanel from './PsosPanel.vue'
import BaktaPanel from './BaktaPanel.vue'
import type { PsosAnnotation, PsosProfile } from '../../api/psos.ts'
import type { BaktaAnnotationSummary, IngestResponse, SequenceType } from '../../api/bakta.ts'

// ── Sequence data props ────────────────────────────────────────────────────
defineProps<{
  // Data
  allSequences:         any[]
  filteredSequences:    any[]
  paginatedSequences:   any[]
  unmatchedSequences:   any[]
  detectedSequenceType: SequenceType

  // Filter state
  currentFilter:        SequenceFilter
  searchText:           string
  debouncedSearch:      string
  minLength:            number | undefined
  maxLength:            number | undefined
  selectedCog:          string
  selectedEcClass:      string
  hasGeneOnly:          boolean
  hasProductOnly:       boolean
  showAdvancedFilters:  boolean
  hasActiveFilters:     boolean
  activeFilterBadges:   { label: string; type: string }[]

  // Pagination
  pagination:           { page: number; total_pages: number; has_prev: boolean; has_next: boolean }
  pageNumbers:          number[]

  // Psos props
  psosShow:             boolean
  psosProfile:          PsosProfile
  psosProfiles:         any[]
  psosAnalyzing:        boolean
  psosProgress:         number
  psosTotal:            number
  psosError:            string
  psosResults:          Map<string, PsosAnnotation>
  psosCopied:           boolean

  // Bakta props
  baktaShow:            boolean
  baktaAnalyzing:       boolean
  baktaProgressLabel:   string
  baktaProgressPercent: number
  baktaError:           string
  baktaResult:          BaktaAnnotationSummary | null
  baktaAbortController: AbortController | null
  baktaGenus:           string
  baktaSpecies:         string
  baktaCompleteGenome:  boolean
  baktaAutoIngestEnabled: boolean
  baktaIngesting:       boolean
  baktaIngestResult:    IngestResponse | null
  baktaIngestError:     string
  groupFeaturesByType:  (features: any[]) => Record<string, number>
}>()

const emit = defineEmits<{
  // Filters
  'update:currentFilter':   [v: SequenceFilter]
  'update:searchText':      [v: string]
  'update:minLength':       [v: number | undefined]
  'update:maxLength':       [v: number | undefined]
  'update:selectedCog':     [v: string]
  'update:selectedEcClass': [v: string]
  'update:hasGeneOnly':     [v: boolean]
  'update:hasProductOnly':  [v: boolean]
  'update:showAdvancedFilters': [v: boolean]
  'clear-filters':          []
  'download-filtered':      [format: FilteredDownloadFormat]
  'go-to-page':             [page: number]

  // Psos
  'update:psosShow':        [v: boolean]
  'update:psosProfile':     [v: PsosProfile]
  'psos-analyze':           []
  'psos-open':              []
  'psos-download-fasta':    []
  'psos-download-tsv':      []

  // Bakta
  'update:baktaShow':       [v: boolean]
  'update:baktaGenus':      [v: string]
  'update:baktaSpecies':    [v: string]
  'update:baktaCompleteGenome': [v: boolean]
  'update:baktaAutoIngestEnabled': [v: boolean]
  'bakta-analyze':          []
  'bakta-ingest':           []
  'bakta-reset':            []
}>()
</script>

<template>
  <div class="sequences-section">
    <!-- ── Search & Filter Bar ──────────────────────────────────────────── -->
    <div class="search-filter-bar">
      <div class="search-box">
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24"
             fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <input type="text" :value="searchText" placeholder="Search ID, gene, product…" class="search-input"
               @input="emit('update:searchText', ($event.target as HTMLInputElement).value)" />
        <button v-if="searchText" class="clear-search" @click="emit('update:searchText', '')">×</button>
      </div>

      <div class="filter-buttons">
        <button v-for="opt in filterOptions" :key="opt.value"
                class="filter-btn" :class="{ active: currentFilter === opt.value }"
                @click="emit('update:currentFilter', opt.value)">
          {{ opt.label }}
        </button>
      </div>

      <button class="advanced-toggle" :class="{ active: showAdvancedFilters, 'has-filters': hasActiveFilters }"
              @click="emit('update:showAdvancedFilters', !showAdvancedFilters)">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
             fill="none" stroke="currentColor" stroke-width="2">
          <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/>
        </svg>
        Filters
        <span v-if="hasActiveFilters" class="filter-badge">!</span>
      </button>
    </div>

    <!-- ── Advanced Filters ─────────────────────────────────────────────── -->
    <div v-if="showAdvancedFilters" class="advanced-filters">
      <div class="filter-row">
        <div class="filter-group">
          <label>Sequence Length</label>
          <div class="length-inputs">
            <input type="number" :value="minLength" placeholder="Min" min="0"
                   @input="emit('update:minLength', ($event.target as HTMLInputElement).valueAsNumber || undefined)" />
            <span>–</span>
            <input type="number" :value="maxLength" placeholder="Max" min="0"
                   @input="emit('update:maxLength', ($event.target as HTMLInputElement).valueAsNumber || undefined)" />
            <span class="unit">aa</span>
          </div>
        </div>
        <div class="filter-group">
          <label>COG Category</label>
          <select :value="selectedCog" @change="emit('update:selectedCog', ($event.target as HTMLSelectElement).value)">
            <option value="">All categories</option>
            <option v-for="c in cogCategories" :key="c.value" :value="c.value">{{ c.label }}</option>
          </select>
        </div>
        <div class="filter-group">
          <label>Enzyme Class (EC)</label>
          <select :value="selectedEcClass" @change="emit('update:selectedEcClass', ($event.target as HTMLSelectElement).value)">
            <option value="">All classes</option>
            <option v-for="e in ecClasses" :key="e.value" :value="e.value">{{ e.label }}</option>
          </select>
        </div>
      </div>
      <div class="filter-row">
        <div class="filter-group checkbox-group">
          <label class="checkbox-label">
            <input type="checkbox" :checked="hasGeneOnly"
                   @change="emit('update:hasGeneOnly', ($event.target as HTMLInputElement).checked)" />
            <span>Has gene name</span>
          </label>
          <label class="checkbox-label">
            <input type="checkbox" :checked="hasProductOnly"
                   @change="emit('update:hasProductOnly', ($event.target as HTMLInputElement).checked)" />
            <span>Has function description</span>
          </label>
        </div>
        <button v-if="hasActiveFilters" class="clear-filters-btn" @click="emit('clear-filters')">
          Clear all filters
        </button>
      </div>
    </div>

    <!-- ── Download Bar ──────────────────────────────────────────────────── -->
    <div class="seq-download-bar">
      <div class="seq-download-summary">
        <span class="seq-download-count">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" style="opacity:0.55">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          <strong>{{ filteredSequences.length.toLocaleString() }}</strong>
          <span v-if="hasActiveFilters"> of {{ allSequences.length.toLocaleString() }}</span>
          sequences
          <span v-if="searchText && searchText !== debouncedSearch" class="typing-indicator">...</span>
        </span>
        <span class="seq-dl-divider">·</span>
        <span v-if="activeFilterBadges.length === 0" class="seq-filter-badge seq-filter-badge--none">No filters active</span>
        <template v-else>
          <span v-for="badge in activeFilterBadges" :key="badge.label"
                class="seq-filter-badge" :class="`seq-filter-badge--${badge.type}`">{{ badge.label }}</span>
        </template>
      </div>
      <div class="seq-download-actions">
        <button class="seq-dl-btn" title="Tab-separated values" @click="emit('download-filtered', 'tsv')">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
          <span class="seq-dl-btn__ext">TSV</span>
        </button>
        <button class="seq-dl-btn" title="Comma-separated values" @click="emit('download-filtered', 'csv')">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
          <span class="seq-dl-btn__ext">CSV</span>
        </button>
        <button class="seq-dl-btn" title="FASTA with annotation header" @click="emit('download-filtered', 'fasta')">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
          <span class="seq-dl-btn__ext">FASTA</span>
        </button>
        <button class="seq-dl-btn" title="Full data as JSON array" @click="emit('download-filtered', 'json')">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
          <span class="seq-dl-btn__ext">JSON</span>
        </button>
      </div>
    </div>

    <!-- ── Sequence Table ──────────────────────────────────────────────── -->
    <div v-if="paginatedSequences.length > 0" class="sequences-table">
      <div class="table-wrapper">
        <table>
          <thead>
            <tr>
              <th>ID</th><th>Length</th><th>Gene</th><th>Function / Product</th><th>Links</th>
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
                <span v-if="seq.product === 'hypothetical protein'" class="hypothetical-badge">hypothetical</span>
                <span v-else-if="seq.product" class="product-desc">{{ seq.product }}</span>
                <span v-else class="no-data">-</span>
              </td>
              <td class="annotation-cell">
                <template v-if="hasAnnotationLinks(seq)">
                  <div class="annotation-links">
                    <a v-if="seq.uniref100_id" :href="getUniRef100Url(seq.uniref100_id)" target="_blank" class="db-link uniref">UniRef</a>
                    <a v-if="seq.uniparc_id"   :href="getUniParcUrl(seq.uniparc_id)"     target="_blank" class="db-link uniparc">UniParc</a>
                    <a v-if="seq.ncbi_nrp_id"  :href="getNcbiUrl(seq.ncbi_nrp_id)"       target="_blank" class="db-link ncbi">NCBI</a>
                    <span v-if="seq.annotation_source === 'aidb_db' && seq.product === 'hypothetical protein'"
                          class="db-link aidb-source" title="Identified via AI-DB annotations DB">AI-DB</span>
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
      <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24"
           fill="none" stroke="currentColor" stroke-width="1.5">
        <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
      <p>No sequences match the current filters.</p>
      <button class="btn btn-secondary" @click="emit('clear-filters')">Clear Filters</button>
    </div>

    <!-- ── Analysis Panels ──────────────────────────────────────────────── -->
    <PsosPanel
      :unmatchedCount="unmatchedSequences.length"
      :selectedProfile="psosProfile"
      :profiles="psosProfiles"
      :analyzing="psosAnalyzing"
      :progress="psosProgress"
      :total="psosTotal"
      :error="psosError"
      :results="psosResults"
      :copied="psosCopied"
      :show="psosShow"
      @update:show="emit('update:psosShow', $event)"
      @update:selectedProfile="emit('update:psosProfile', $event)"
      @analyze="emit('psos-analyze')"
      @open-in-psos="emit('psos-open')"
      @download-fasta="emit('psos-download-fasta')"
      @download-tsv="emit('psos-download-tsv')"
    />

    <BaktaPanel
      :unmatchedCount="unmatchedSequences.length"
      :sequenceType="detectedSequenceType"
      :show="baktaShow"
      :analyzing="baktaAnalyzing"
      :progressLabel="baktaProgressLabel"
      :progressPercent="baktaProgressPercent"
      :error="baktaError"
      :result="baktaResult"
      :abortController="baktaAbortController"
      :genus="baktaGenus"
      :species="baktaSpecies"
      :completeGenome="baktaCompleteGenome"
      :autoIngestEnabled="baktaAutoIngestEnabled"
      :ingesting="baktaIngesting"
      :ingestResult="baktaIngestResult"
      :ingestError="baktaIngestError"
      :groupFeaturesByType="groupFeaturesByType"
      @update:show="emit('update:baktaShow', $event)"
      @update:genus="emit('update:baktaGenus', $event)"
      @update:species="emit('update:baktaSpecies', $event)"
      @update:completeGenome="emit('update:baktaCompleteGenome', $event)"
      @update:autoIngestEnabled="emit('update:baktaAutoIngestEnabled', $event)"
      @analyze="emit('bakta-analyze')"
      @ingest="emit('bakta-ingest')"
      @reset="emit('bakta-reset')"
    />

    <!-- ── Pagination ────────────────────────────────────────────────────── -->
    <div v-if="pagination.total_pages > 1" class="sequences-pagination">
      <button class="page-btn" :disabled="!pagination.has_prev" @click="emit('go-to-page', pagination.page - 1)">←</button>
      <button v-if="pageNumbers[0] > 1" class="page-btn" @click="emit('go-to-page', 1)">1</button>
      <span v-if="pageNumbers[0] > 2" class="page-ellipsis">...</span>
      <button v-for="page in pageNumbers" :key="page" class="page-btn"
              :class="{ active: page === pagination.page }" @click="emit('go-to-page', page)">{{ page }}</button>
      <span v-if="pageNumbers[pageNumbers.length - 1] < pagination.total_pages - 1" class="page-ellipsis">...</span>
      <button v-if="pageNumbers[pageNumbers.length - 1] < pagination.total_pages" class="page-btn"
              @click="emit('go-to-page', pagination.total_pages)">{{ pagination.total_pages }}</button>
      <button class="page-btn" :disabled="!pagination.has_next" @click="emit('go-to-page', pagination.page + 1)">→</button>
      <span class="page-info">Page {{ pagination.page }} of {{ pagination.total_pages }}</span>
    </div>
  </div>
</template>

<style scoped>
  /* All sequences-* / search / filter / table / pagination styles from original */
  .sequences-section { margin-top: 1rem; }
  .search-filter-bar { display: flex; gap: 1rem; align-items: center; flex-wrap: wrap; margin-bottom: 1rem; }
  .search-box { flex: 1; min-width: 250px; position: relative; display: flex; align-items: center; }
  .search-box svg { position: absolute; left: 12px; color: var(--color-text); opacity: 0.5; }
  .search-input { width: 100%; padding: 0.6rem 2.5rem; border: 1px solid var(--color-border); border-radius: 8px; background: var(--color-background); color: var(--color-text); font-size: 0.9rem; }
  .search-input:focus { outline: none; border-color: hsla(160,100%,37%,0.5); box-shadow: 0 0 0 3px hsla(160,100%,37%,0.1); }
  .clear-search { position: absolute; right: 8px; background: none; border: none; color: var(--color-text); opacity: 0.5; cursor: pointer; font-size: 1.2rem; padding: 0.25rem; }
  .typing-indicator { color: hsla(160,100%,37%,1); animation: blink 0.8s infinite; }
  @keyframes blink { 0%,50%{opacity:1} 51%,100%{opacity:0.3} }
  .filter-buttons { display: flex; gap: 0.5rem; flex-wrap: wrap; }
  .filter-btn { padding: 0.4rem 0.85rem; border: 1px solid var(--color-border); border-radius: 99px; background: transparent; color: var(--color-text); font-size: 0.85rem; cursor: pointer; transition: all 0.2s; white-space: nowrap; }
  .filter-btn:hover { border-color: hsla(160,100%,37%,0.5); }
  .filter-btn.active { background: hsla(160,100%,37%,0.1); border-color: hsla(160,100%,37%,0.5); color: hsla(160,100%,30%,1); font-weight: 500; }
  .advanced-toggle { display: flex; align-items: center; gap: 0.4rem; padding: 0.4rem 0.85rem; border: 1px solid var(--color-border); border-radius: 99px; background: transparent; color: var(--color-text); font-size: 0.85rem; cursor: pointer; transition: all 0.2s; white-space: nowrap; position: relative; }
  .advanced-toggle.active,.advanced-toggle.has-filters { border-color: hsla(160,100%,37%,0.5); }
  .filter-badge { background: hsla(160,100%,37%,1); color: white; border-radius: 99px; padding: 0 4px; font-size: 0.7rem; font-weight: 700; }
  .advanced-filters { margin-bottom: 1rem; padding: 1rem; background: var(--color-background-soft); border-radius: 8px; border: 1px solid var(--color-border); display: flex; flex-direction: column; gap: 0.75rem; }
  .filter-row { display: flex; gap: 1rem; flex-wrap: wrap; align-items: flex-end; }
  .filter-group { display: flex; flex-direction: column; gap: 0.35rem; flex: 1; min-width: 160px; }
  .filter-group label { font-size: 0.82rem; font-weight: 500; color: var(--color-heading); }
  .filter-group select,.filter-group input[type="number"] { padding: 0.45rem 0.6rem; border: 1px solid var(--color-border); border-radius: 6px; background: var(--color-background); color: var(--color-text); font-size: 0.875rem; }
  .length-inputs { display: flex; align-items: center; gap: 0.4rem; }
  .length-inputs input { width: 80px; }
  .unit { font-size: 0.8rem; color: var(--color-text); opacity: 0.6; }
  .checkbox-group { flex-direction: row; align-items: center; flex-wrap: wrap; gap: 1rem; }
  .checkbox-label { display: flex; align-items: center; gap: 0.4rem; font-size: 0.875rem; cursor: pointer; color: var(--color-text); }
  .clear-filters-btn { padding: 0.4rem 1rem; background: transparent; border: 1px solid var(--color-border); border-radius: 6px; color: var(--color-text); font-size: 0.85rem; cursor: pointer; white-space: nowrap; }
  .clear-filters-btn:hover { border-color: #f44336; color: #f44336; }
  /* Download bar */
  .seq-download-bar { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 0.5rem; padding: 0.5rem 0; margin-bottom: 0.75rem; }
  .seq-download-summary { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; font-size: 0.85rem; color: var(--color-text); }
  .seq-download-count { display: flex; align-items: center; gap: 0.3rem; }
  .seq-dl-divider { opacity: 0.3; }
  .seq-filter-badge { padding: 0.15rem 0.5rem; border-radius: 99px; font-size: 0.75rem; font-weight: 500; }
  .seq-filter-badge--none { background: var(--color-background-soft); color: var(--color-text); opacity: 0.55; }
  .seq-filter-badge--status { background: hsla(160,100%,37%,0.1); color: hsla(160,100%,30%,1); }
  .seq-filter-badge--search { background: rgba(33,150,243,0.1); color: #1565c0; }
  .seq-filter-badge--length,.seq-filter-badge--cog,.seq-filter-badge--ec { background: rgba(156,39,176,0.1); color: #6a1b9a; }
  .seq-filter-badge--flag { background: rgba(255,152,0,0.1); color: #e65100; }
  .seq-download-actions { display: flex; gap: 0.35rem; }
  .seq-dl-btn { display: inline-flex; align-items: center; gap: 0.3rem; padding: 0.35rem 0.65rem; background: var(--color-background-soft); border: 1px solid var(--color-border); border-radius: 6px; font-size: 0.8rem; color: var(--color-text); cursor: pointer; transition: all 0.15s; }
  .seq-dl-btn:hover { border-color: hsla(160,100%,37%,0.5); color: hsla(160,100%,30%,1); }
  .seq-dl-btn__ext { font-weight: 600; font-size: 0.75rem; }
  /* Table */
  .sequences-table { overflow: hidden; border-radius: 8px; border: 1px solid var(--color-border); }
  .table-wrapper { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
  thead th { background: var(--color-background-soft); padding: 0.75rem 1rem; text-align: left; font-weight: 600; font-size: 0.82rem; color: var(--color-text); opacity: 0.8; border-bottom: 1px solid var(--color-border); white-space: nowrap; }
  tbody tr { border-bottom: 1px solid var(--color-border); transition: background 0.15s; }
  tbody tr:last-child { border-bottom: none; }
  tbody tr:hover { background: var(--color-background-soft); }
  tbody tr.has-match { background: rgba(0,189,126,0.02); }
  td { padding: 0.65rem 1rem; vertical-align: middle; }
  .seq-id { font-family: monospace; font-size: 0.8rem; color: var(--color-text); max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .seq-length { font-variant-numeric: tabular-nums; white-space: nowrap; }
  .gene-name { font-style: italic; color: var(--color-heading); }
  .product-desc { color: var(--color-text); }
  .hypothetical-badge { display: inline-block; padding: 0.1rem 0.45rem; background: var(--color-background-mute); border-radius: 4px; font-size: 0.75rem; color: var(--color-text); opacity: 0.6; }
  .no-data { color: var(--color-text); opacity: 0.3; }
  .annotation-links { display: flex; gap: 0.35rem; flex-wrap: wrap; }
  .db-link { display: inline-block; padding: 0.15rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: 500; text-decoration: none; transition: opacity 0.15s; }
  .db-link:hover { opacity: 0.8; }
  .db-link.uniref  { background: rgba(0,189,126,0.12); color: hsla(160,100%,30%,1); }
  .db-link.uniparc { background: rgba(33,150,243,0.12); color: #1565c0; }
  .db-link.ncbi    { background: rgba(156,39,176,0.12); color: #6a1b9a; }
  .db-link.aidb-source { background: rgba(224,128,0,0.12); color: #a05a00; cursor: help; }
  .empty-filter-results { text-align: center; padding: 3rem 1rem; color: var(--color-text); opacity: 0.6; }
  .empty-filter-results svg { margin: 0 auto 1rem; display: block; opacity: 0.4; }
  /* Pagination */
  .sequences-pagination { display: flex; align-items: center; gap: 0.35rem; flex-wrap: wrap; justify-content: center; padding: 1rem 0; }
  .page-btn { min-width: 36px; padding: 0.4rem 0.6rem; border: 1px solid var(--color-border); border-radius: 6px; background: var(--color-background); color: var(--color-text); font-size: 0.875rem; cursor: pointer; transition: all 0.15s; }
  .page-btn:hover:not(:disabled) { border-color: hsla(160,100%,37%,0.5); }
  .page-btn.active { background: hsla(160,100%,37%,0.1); border-color: hsla(160,100%,37%,0.5); color: hsla(160,100%,30%,1); font-weight: 600; }
  .page-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .page-ellipsis { padding: 0 0.2rem; color: var(--color-text); opacity: 0.4; }
  .page-info { font-size: 0.8rem; color: var(--color-text); opacity: 0.6; margin-left: 0.5rem; }
  @media (max-width: 600px) {
    .search-filter-bar { flex-direction: column; align-items: stretch; }
    .search-box { min-width: 100%; }
    .filter-buttons { justify-content: center; }
    .advanced-toggle { justify-content: center; }
    .filter-row { flex-direction: column; }
    .filter-group { min-width: 100%; }
    .checkbox-group { flex-wrap: wrap; }
  }
</style>
