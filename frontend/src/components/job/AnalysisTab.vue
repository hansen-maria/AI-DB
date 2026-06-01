<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { FunctionalStats } from '../../api/jobs.ts'
import { exportJobStats } from '../../api/jobs.ts'
import { getSequentialColor, getCategoricalColor } from '../../constants/sequences.ts'

const props = defineProps<{
  jobId:   string
  loading: boolean
  stats:   FunctionalStats | null
}>()

// ── Stats export ─────────────────────────────────────────────────────────────

const exporting     = ref(false)
const exportError   = ref('')

async function handleExport() {
  exporting.value   = true
  exportError.value = ''
  try {
    await exportJobStats(props.jobId)
  } catch (e) {
    exportError.value = e instanceof Error ? e.message : 'Export failed'
  } finally {
    exporting.value = false
  }
}

const annotationRate = computed(() => {
  if (!props.stats || props.stats.total_sequences === 0) return 0
  return Math.round((props.stats.annotated_sequences / props.stats.total_sequences) * 100)
})

// ── GO Term label resolution ─────────────────────────────────────────────────

/** GO:xxxxxxx → human-readable name, populated via QuickGO REST API */
const goLabels = ref<Record<string, string>>({})
const goLabelsLoading = ref(false)

async function fetchGoLabels(ids: string[]) {
  if (ids.length === 0) return
  goLabelsLoading.value = true
  try {
    // QuickGO allows up to 200 IDs per request
    const chunkSize = 200
    for (let i = 0; i < ids.length; i += chunkSize) {
      const chunk  = ids.slice(i, i + chunkSize)
      const joined = chunk.join(',')
      const res = await fetch(
          `https://www.ebi.ac.uk/QuickGO/services/ontology/go/terms/${encodeURIComponent(joined)}`,
          { headers: { Accept: 'application/json' } }
      )
      if (!res.ok) continue
      const data = await res.json()
      for (const term of data.results ?? []) {
        goLabels.value[term.id] = term.name
      }
    }
  } catch (e) {
    console.warn('GO label fetch failed:', e)
  } finally {
    goLabelsLoading.value = false
  }
}

/** Collect all unique GO IDs from all three ontologies and resolve them */
function resolveAllGoIds(stats: FunctionalStats) {
  const ids = new Set<string>()
  for (const item of [
    ...stats.go_terms.molecular_function,
    ...stats.go_terms.biological_process,
    ...stats.go_terms.cellular_component,
  ]) {
    if (!(item.name in goLabels.value)) ids.add(item.name)
  }
  if (ids.size > 0) fetchGoLabels([...ids])
}

watch(
    () => props.stats,
    (s) => { if (s) resolveAllGoIds(s) },
    { immediate: true }
)

// ── GO section helpers ───────────────────────────────────────────────────────

const goSections = computed(() => {
  if (!props.stats) return []
  const { molecular_function, biological_process, cellular_component } = props.stats.go_terms
  return [
    { key: 'MF', label: 'Molecular Function', color: '#05668d', items: molecular_function },
    { key: 'BP', label: 'Biological Process', color: '#00a896', items: biological_process },
    { key: 'CC', label: 'Cellular Component',  color: '#6b5b95', items: cellular_component },
  ].filter(s => s.items.length > 0)
})

const hasGoTerms = computed(() => goSections.value.length > 0)
</script>

<template>
  <div class="tab-panel">
    <div v-if="loading" class="loading-stats">
      <div class="spinner"></div> Loading functional analysis...
    </div>

    <div v-else-if="stats" class="analysis-section">
      <!-- Toolbar -->
      <div class="analysis-toolbar">
        <div class="analysis-toolbar__info">
          <span class="analysis-toolbar__count">{{ stats.total_sequences.toLocaleString() }} sequences</span>
        </div>
        <div class="analysis-toolbar__actions">
          <span v-if="exportError" class="export-error">{{ exportError }}</span>
          <button class="export-btn" :disabled="exporting" @click="handleExport" title="Download statistics as CSV">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
              <polyline points="7 10 12 15 17 10"/>
              <line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
            {{ exporting ? 'Exporting…' : 'Export CSV' }}
          </button>
        </div>
      </div>

      <!-- Annotation Rate circle -->
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
            <span class="rate-detail">
              {{ stats.annotated_sequences.toLocaleString() }} of {{ stats.total_sequences.toLocaleString() }} sequences
            </span>
          </div>
        </div>
      </div>

      <!-- Charts grid -->
      <div class="charts-grid">
        <!-- Top Genes -->
        <div class="chart-card">
          <h4>Top Genes</h4>
          <div v-if="stats.top_genes.length > 0" class="horizontal-bars">
            <div v-for="(item, index) in stats.top_genes.slice(0, 12)" :key="item.name" class="bar-item">
              <span class="bar-label">{{ item.name }}</span>
              <div class="bar-wrapper">
                <div class="bar-fill"
                     :style="{ width: `${(item.count / stats.top_genes[0].count) * 100}%`, backgroundColor: getSequentialColor(index) }"></div>
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
                <div class="bar-fill"
                     :style="{ width: `${(item.count / stats.top_products[0].count) * 100}%`, backgroundColor: getSequentialColor(index) }"></div>
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
                <div class="bar-fill"
                     :style="{ width: `${(item.count / stats.cog_categories[0].count) * 100}%`, backgroundColor: getCategoricalColor(index) }"></div>
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
                <div class="bar-fill"
                     :style="{ width: `${(item.count / stats.ec_classes[0].count) * 100}%`, backgroundColor: getCategoricalColor(index) }"></div>
              </div>
              <span class="bar-value">{{ item.count }}</span>
            </div>
          </div>
          <div v-else class="no-chart-data">No enzyme classifications found</div>
        </div>

        <!-- GO Terms ─────────────────────────────────────────────────────── -->
        <div v-if="hasGoTerms" class="chart-card chart-card-wide">
          <div class="go-card-title">
            <h4>Gene Ontology (GO) Terms</h4>
            <span v-if="goLabelsLoading" class="go-loading-badge">
              <span class="go-spinner"></span> Resolving labels…
            </span>
          </div>

          <div class="go-ontologies-grid">
            <div v-for="section in goSections" :key="section.key" class="go-section">

              <div class="go-section-header" :style="{ borderLeftColor: section.color }">
                <span class="go-section-title">{{ section.label }}</span>
                <span class="go-section-count">{{ section.items.length }} terms</span>
              </div>

              <div class="horizontal-bars">
                <div v-for="item in section.items.slice(0, 10)" :key="item.name" class="bar-item go-bar-item">
                  <div class="go-label-block">
                    <a
                        :href="`https://www.ebi.ac.uk/QuickGO/term/${item.name}`"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="go-id-link"
                    >{{ item.name }}</a>
                    <span v-if="goLabels[item.name]" class="go-name">{{ goLabels[item.name] }}</span>
                    <span v-else-if="goLabelsLoading" class="go-name go-name-loading">loading…</span>
                  </div>
                  <div class="bar-wrapper">
                    <div class="bar-fill"
                         :style="{ width: `${(item.count / section.items[0].count) * 100}%`, backgroundColor: section.color }">
                    </div>
                  </div>
                  <span class="bar-value">{{ item.count }}</span>
                </div>
              </div>

            </div>
          </div>
        </div>
        <!-- /GO Terms -->

      </div>
    </div>
  </div>
</template>

<style scoped>
.loading-stats { display: flex; align-items: center; gap: 0.75rem; padding: 2rem; color: var(--color-text); }
.spinner { width: 24px; height: 24px; border: 2px solid var(--color-border); border-top-color: hsla(160,100%,37%,1); border-radius: 50%; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

/* Toolbar */
.analysis-toolbar { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1rem; }
.analysis-toolbar__info { font-size: 0.85rem; color: var(--color-text); opacity: 0.6; }
.analysis-toolbar__actions { display: flex; align-items: center; gap: 0.75rem; }
.export-btn {
  display: inline-flex; align-items: center; gap: 0.4rem;
  padding: 0.4rem 0.9rem;
  font-size: 0.82rem; font-weight: 500;
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  border-radius: 6px; color: var(--color-text);
  cursor: pointer; transition: border-color 0.15s, color 0.15s;
}
.export-btn:hover:not(:disabled) { border-color: hsla(160,100%,37%,0.6); color: hsla(160,100%,37%,1); }
.export-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.export-error { font-size: 0.78rem; color: #f44336; }

.analysis-section { display: flex; flex-direction: column; gap: 2rem; }
.annotation-overview { background: var(--color-background-soft); border: 1px solid var(--color-border); border-radius: 12px; padding: 1.5rem; }
.annotation-rate { display: flex; align-items: center; gap: 1.5rem; }
.rate-circle { position: relative; width: 80px; height: 80px; flex-shrink: 0; }
.circular-chart { display: block; width: 80px; height: 80px; }
.circle-bg { fill: none; stroke: var(--color-background-mute); stroke-width: 3.8; }
.circle { fill: none; stroke: hsla(160,100%,37%,1); stroke-width: 3.8; stroke-linecap: round; transform: rotate(-90deg); transform-origin: 50% 50%; transition: stroke-dasharray 0.6s ease; }
.rate-value { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; font-size: 1.1rem; font-weight: 700; color: var(--color-heading); }
.rate-info { display: flex; flex-direction: column; gap: 0.25rem; }
.rate-label { font-size: 1rem; font-weight: 600; color: var(--color-heading); }
.rate-detail { font-size: 0.85rem; color: var(--color-text); opacity: 0.7; }

.charts-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 1.5rem; }
.chart-card { background: var(--color-background-soft); border: 1px solid var(--color-border); border-radius: 12px; padding: 1.25rem; }
.chart-card h4 { margin: 0 0 1rem; color: var(--color-heading); font-size: 0.95rem; }
.chart-card-wide { grid-column: 1 / -1; }

.horizontal-bars { display: flex; flex-direction: column; gap: 0.5rem; }
.bar-item { display: grid; grid-template-columns: 140px 1fr 40px; align-items: center; gap: 0.75rem; font-size: 0.82rem; }
.bar-label { color: var(--color-text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.bar-wrapper { background: var(--color-background-mute); border-radius: 2px; overflow: hidden; height: 12px; }
.bar-fill { height: 100%; border-radius: 2px; transition: width 0.4s ease; }
.bar-value { text-align: right; font-variant-numeric: tabular-nums; color: var(--color-text); font-size: 0.8rem; }
.cog-code { display: inline-block; width: 1.2em; font-weight: 700; color: var(--color-heading); }
.no-chart-data { color: var(--color-text); opacity: 0.5; font-size: 0.875rem; padding: 1rem 0; text-align: center; }

/* ── GO Terms ──────────────────────────────────────────────────────────── */
.go-card-title { display: flex; align-items: center; gap: 0.75rem; margin-bottom: 1.25rem; }
.go-card-title h4 { margin: 0; }

.go-loading-badge {
  display: inline-flex; align-items: center; gap: 0.4rem;
  font-size: 0.75rem; color: var(--color-text); opacity: 0.6;
  background: var(--color-background-mute); border-radius: 20px; padding: 0.15rem 0.6rem;
}
.go-spinner {
  width: 10px; height: 10px;
  border: 1.5px solid var(--color-border);
  border-top-color: hsla(160,100%,37%,1);
  border-radius: 50%; animation: spin 0.8s linear infinite;
}

.go-ontologies-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 1.5rem;
}

.go-section-header {
  display: flex; align-items: center; gap: 0.5rem;
  margin-bottom: 0.65rem;
  padding-left: 0.6rem;
  border-left: 3px solid transparent;
}
.go-section-title { font-size: 0.85rem; font-weight: 600; color: var(--color-heading); }
.go-section-count { margin-left: auto; font-size: 0.75rem; color: var(--color-text); opacity: 0.5; }

/* Wider label column for GO IDs + name */
.go-bar-item { grid-template-columns: 150px 1fr 40px; }

.go-label-block { display: flex; flex-direction: column; gap: 0.05rem; min-width: 0; }

.go-id-link {
  font-family: monospace; font-size: 0.8rem; line-height: 1.3;
  color: hsla(160,100%,37%,1); text-decoration: none;
}
.go-id-link:hover { text-decoration: underline; }

.go-name {
  font-size: 0.75rem; color: var(--color-text); opacity: 0.7;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis; line-height: 1.3;
}
.go-name-loading { opacity: 0.35; font-style: italic; }

@media (max-width: 900px) {
  .charts-grid { grid-template-columns: 1fr; }
  .go-ontologies-grid { grid-template-columns: 1fr; }
}
@media (max-width: 600px) {
  .annotation-rate { flex-direction: column; text-align: center; }
  .bar-item { grid-template-columns: 1fr; gap: 0.25rem; }
  .go-bar-item { grid-template-columns: 1fr 36px; }
  .go-bar-item .bar-wrapper { display: none; }
}
</style>