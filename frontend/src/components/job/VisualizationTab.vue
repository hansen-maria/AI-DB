<script setup lang="ts">
/**
 * VisualizationTab.vue
 *
 * Krona-style interactive sunburst charts via ECharts (CDN).
 * Data sources – all from existing AI-DB endpoints:
 *
 *   COG  – stats.cog_categories  (code + name + count)
 *   GO   – stats.go_terms        (BP / MF / CC + counts)
 *          GO labels resolved via QuickGO REST API (reused from AnalysisTab)
 *   EC   – stats.ec_classes      (top-level enzyme classes + counts)
 *          Detailed EC distribution computed from allSequences[].ec_ids
 *
 * Props mirror AnalysisTab so JobDetailView needs no new data fetching.
 */
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import type { FunctionalStats } from '../../api/jobs.ts'
import type { SequenceInfo } from '../../api/jobs.ts'

const props = defineProps<{
  jobId:        string
  jobStatus:    string
  stats:        FunctionalStats | null
  allSequences: SequenceInfo[]
}>()

// ── ECharts CDN ───────────────────────────────────────────────────────────────
const ECHARTS_CDN = 'https://cdn.jsdelivr.net/npm/echarts@5/dist/echarts.min.js'

async function loadECharts(): Promise<any> {
  if ((window as any).echarts) return (window as any).echarts
  return new Promise((resolve, reject) => {
    if (document.querySelector(`script[src="${ECHARTS_CDN}"]`)) {
      const poll = () => (window as any).echarts ? resolve((window as any).echarts) : setTimeout(poll, 40)
      poll(); return
    }
    const s = document.createElement('script')
    s.src = ECHARTS_CDN
    s.onload  = () => resolve((window as any).echarts)
    s.onerror = () => reject(new Error('Failed to load ECharts'))
    document.head.appendChild(s)
  })
}

// ── Active chart tab ──────────────────────────────────────────────────────────
type ChartTab = 'cog' | 'go' | 'ec'
const activeTab = ref<ChartTab>('cog')

// ── ECharts instances (one per chart) ─────────────────────────────────────────
const cogContainer = ref<HTMLDivElement | null>(null)
const goContainer  = ref<HTMLDivElement | null>(null)
const ecContainer  = ref<HTMLDivElement | null>(null)

let cogChart: any = null
let goChart:  any = null
let ecChart:  any = null

// ─────────────────────────────────────────────────────────────────────────────
// COG Sunburst
// ─────────────────────────────────────────────────────────────────────────────

// COG functional super-categories (classic NCBI grouping)
const COG_SUPER: Record<string, { label: string; color: string; codes: string[] }> = {
  'Information storage & processing': {
    color: '#1565c0',
    codes: ['J','A','K','L','B'],
    label: 'Information storage & processing',
  },
  'Cellular processes & signaling': {
    color: '#2e7d32',
    codes: ['D','Y','V','T','M','N','Z','W','U','O','X'],
    label: 'Cellular processes & signaling',
  },
  'Metabolism': {
    color: '#e65100',
    codes: ['C','G','E','F','H','I','P','Q'],
    label: 'Metabolism',
  },
  'Poorly characterized': {
    color: '#6a1e6e',
    codes: ['R','S'],
    label: 'Poorly characterized',
  },
}

const COG_NAME: Record<string, string> = {
  J:'Translation, ribosomal structure & biogenesis',
  A:'RNA processing and modification',
  K:'Transcription',
  L:'Replication, recombination and repair',
  B:'Chromatin structure and dynamics',
  D:'Cell cycle control, cell division, chromosome partitioning',
  Y:'Nuclear structure',
  V:'Defense mechanisms',
  T:'Signal transduction mechanisms',
  M:'Cell wall/membrane/envelope biogenesis',
  N:'Cell motility',
  Z:'Cytoskeleton',
  W:'Extracellular structures',
  U:'Intracellular trafficking, secretion, and vesicular transport',
  O:'Posttranslational modification, protein turnover, chaperones',
  X:'Mobilome: prophages, transposons',
  C:'Energy production and conversion',
  G:'Carbohydrate transport and metabolism',
  E:'Amino acid transport and metabolism',
  F:'Nucleotide transport and metabolism',
  H:'Coenzyme transport and metabolism',
  I:'Lipid transport and metabolism',
  P:'Inorganic ion transport and metabolism',
  Q:'Secondary metabolites biosynthesis, transport and catabolism',
  R:'General function prediction only',
  S:'Function unknown',
}

const COG_COLOR: Record<string, string> = {
  // Information (blues)
  J:'#1976d2', A:'#42a5f5', K:'#0d47a1', L:'#1565c0', B:'#82b1ff',
  // Cellular (greens)
  D:'#2e7d32', Y:'#66bb6a', V:'#1b5e20', T:'#43a047', M:'#81c784',
  N:'#a5d6a7', Z:'#388e3c', W:'#4caf50', U:'#00695c', O:'#26a69a', X:'#80cbc4',
  // Metabolism (oranges)
  C:'#e65100', G:'#ff6d00', E:'#f57c00', F:'#ff8f00', H:'#ffa000',
  I:'#ffb300', P:'#ffc107', Q:'#ffca28',
  // Poorly (purples)
  R:'#6a1e6e', S:'#ab47bc',
}

function buildCogSunburst(): any[] {
  if (!props.stats?.cog_categories?.length) return []

  const byCode = new Map(props.stats.cog_categories.map(c => [c.code, c.count]))

  return Object.entries(COG_SUPER).map(([superName, meta]) => {
    const children = meta.codes
        .filter(code => byCode.has(code))
        .map(code => ({
          name:  `${code} – ${COG_NAME[code] ?? code}`,
          value: byCode.get(code)!,
          itemStyle: { color: COG_COLOR[code] ?? '#888' },
        }))
    const total = children.reduce((s, c) => s + c.value, 0)
    if (total === 0) return null
    return {
      name:      superName,
      value:     total,
      itemStyle: { color: meta.color },
      children,
    }
  }).filter(Boolean)
}

async function renderCog() {
  if (!cogContainer.value) return
  const ec = await loadECharts()
  if (!cogChart) cogChart = ec.init(cogContainer.value, null, { renderer: 'svg' })

  const data = buildCogSunburst()
  if (!data.length) return

  cogChart.setOption({
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'item',
      formatter: (p: any) => {
        const pct = props.stats
            ? ((p.value / props.stats.cog_categories.reduce((s, c) => s + c.count, 0)) * 100).toFixed(1)
            : '?'
        return `<b>${p.name}</b><br/>${p.value.toLocaleString()} sequences (${pct}%)`
      },
    },
    series: [{
      type:           'sunburst',
      data,
      radius:         ['15%', '85%'],
      sort:           undefined,
      emphasis:       { focus: 'ancestor' },
      label: {
        show:      true,
        rotate:    'tangential',
        fontSize:  11,
        formatter: (p: any) => p.treeDepth === 0 ? '' : p.name.split(' – ')[0],
      },
      levels: [
        {},
        { r0: '15%', r: '45%', label: { rotate: 'tangential', fontSize: 12 } },
        { r0: '46%', r: '85%', label: { rotate: 'tangential', fontSize: 10, align: 'right' } },
      ],
    }],
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// GO Sunburst
// ─────────────────────────────────────────────────────────────────────────────
const goLabels      = ref<Record<string, string>>({})
const goLabelsLoading = ref(false)

const GO_ROOT_COLOR: Record<string, string> = {
  'Biological Process': '#00a896',
  'Molecular Function': '#05668d',
  'Cellular Component': '#6b5b95',
}

async function fetchGoLabels(ids: string[]) {
  if (!ids.length) return
  goLabelsLoading.value = true
  try {
    for (let i = 0; i < ids.length; i += 200) {
      const chunk = ids.slice(i, i + 200).join(',')
      const res = await fetch(
          `https://www.ebi.ac.uk/QuickGO/services/ontology/go/terms/${encodeURIComponent(chunk)}`,
          { headers: { Accept: 'application/json' } }
      )
      if (!res.ok) continue
      const data = await res.json()
      for (const term of data.results ?? []) goLabels.value[term.id] = term.name
    }
  } catch { /* label resolution is best-effort */ }
  finally { goLabelsLoading.value = false }
}

function buildGoSunburst(): any[] {
  if (!props.stats?.go_terms) return []
  const { biological_process, molecular_function, cellular_component } = props.stats.go_terms

  const toChildren = (items: { name: string; count: number }[]) =>
      items.slice(0, 40).map(item => ({
        name:  goLabels.value[item.name] ? `${item.name}\n${goLabels.value[item.name]}` : item.name,
        value: item.count,
      }))

  const roots = [
    { label: 'Biological Process', items: biological_process },
    { label: 'Molecular Function', items: molecular_function },
    { label: 'Cellular Component', items: cellular_component },
  ].filter(r => r.items.length > 0)

  return roots.map(r => ({
    name:      r.label,
    value:     r.items.reduce((s, i) => s + i.count, 0),
    itemStyle: { color: GO_ROOT_COLOR[r.label] },
    children:  toChildren(r.items),
  }))
}

async function renderGo() {
  if (!goContainer.value) return
  const ec = await loadECharts()
  if (!goChart) goChart = ec.init(goContainer.value, null, { renderer: 'svg' })

  const data = buildGoSunburst()
  if (!data.length) return

  const totalGo = data.reduce((s, d) => s + (d.value as number), 0)

  goChart.setOption({
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'item',
      formatter: (p: any) => {
        const pct = ((p.value / totalGo) * 100).toFixed(1)
        const nameParts = String(p.name).split('\n')
        const id  = nameParts[0]
        const lbl = nameParts[1] ?? ''
        const link = id.startsWith('GO:')
            ? `<a href="https://www.ebi.ac.uk/QuickGO/term/${id}" target="_blank" style="color:#4caf50">${id}</a>`
            : `<b>${id}</b>`
        return `${link}${lbl ? `<br/><i>${lbl}</i>` : ''}<br/>${p.value.toLocaleString()} sequences (${pct}%)`
      },
    },
    series: [{
      type:     'sunburst',
      data,
      radius:   ['15%', '85%'],
      sort:     undefined,
      emphasis: { focus: 'ancestor' },
      label: {
        show:      true,
        rotate:    'tangential',
        fontSize:  10,
        formatter: (p: any) => {
          if (p.treeDepth === 0) return ''
          const name = String(p.name).split('\n')[0]
          return name.length > 20 ? name.slice(0, 18) + '…' : name
        },
      },
      levels: [
        {},
        { r0: '15%', r: '42%', label: { fontSize: 13, fontWeight: 'bold' } },
        { r0: '43%', r: '85%', label: { fontSize: 9, rotate: 'tangential', align: 'right' } },
      ],
    }],
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// EC Sunburst  (2-level: class → sub-class from allSequences[].ec_ids)
// ─────────────────────────────────────────────────────────────────────────────
const EC_CLASS: Record<string, { name: string; color: string }> = {
  '1': { name: 'Oxidoreductases',  color: '#c62828' },
  '2': { name: 'Transferases',     color: '#ad1457' },
  '3': { name: 'Hydrolases',       color: '#6a1b9a' },
  '4': { name: 'Lyases',           color: '#283593' },
  '5': { name: 'Isomerases',       color: '#00695c' },
  '6': { name: 'Ligases',          color: '#e65100' },
  '7': { name: 'Translocases',     color: '#558b2f' },
}

function buildEcSunburst(): any[] {
  // Collect EC IDs from allSequences
  const subClassCounts = new Map<string, number>()  // "1.1", "2.3" etc
  for (const seq of props.allSequences) {
    if (!seq.ec_ids) continue
    for (const ec of seq.ec_ids.split(',')) {
      const parts = ec.trim().split('.')
      if (parts.length < 2 || !parts[0] || !parts[1]) continue
      const sub = `${parts[0]}.${parts[1]}`
      subClassCounts.set(sub, (subClassCounts.get(sub) ?? 0) + 1)
    }
  }
  if (subClassCounts.size === 0) return []

  // Group sub-classes by top-level class
  const classMap = new Map<string, Map<string, number>>()
  for (const [sub, count] of subClassCounts) {
    const cls = sub.split('.')[0]
    if (!classMap.has(cls)) classMap.set(cls, new Map())
    classMap.get(cls)!.set(sub, count)
  }

  return [...classMap.entries()]
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([cls, subs]) => {
        const meta     = EC_CLASS[cls] ?? { name: `Class ${cls}`, color: '#78909c' }
        const children = [...subs.entries()]
            .sort((a, b) => b[1] - a[1])
            .map(([sub, count]) => ({
              name:  `EC ${sub}.-.-`,
              value: count,
            }))
        return {
          name:      `EC ${cls} – ${meta.name}`,
          value:     children.reduce((s, c) => s + c.value, 0),
          itemStyle: { color: meta.color },
          children,
        }
      })
}

async function renderEc() {
  if (!ecContainer.value) return
  const ec = await loadECharts()
  if (!ecChart) ecChart = ec.init(ecContainer.value, null, { renderer: 'svg' })

  const data = buildEcSunburst()
  if (!data.length) return

  const totalEc = data.reduce((s, d) => s + (d.value as number), 0)

  ecChart.setOption({
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'item',
      formatter: (p: any) => {
        const pct = ((p.value / totalEc) * 100).toFixed(1)
        return `<b>${p.name}</b><br/>${p.value.toLocaleString()} sequences (${pct}%)`
      },
    },
    series: [{
      type:     'sunburst',
      data,
      radius:   ['15%', '85%'],
      sort:     undefined,
      emphasis: { focus: 'ancestor' },
      label: {
        show:      true,
        rotate:    'tangential',
        fontSize:  11,
        formatter: (p: any) => p.treeDepth === 0 ? '' : String(p.name).replace('EC ', ''),
      },
      levels: [
        {},
        { r0: '15%', r: '45%', label: { fontSize: 13, fontWeight: 'bold' } },
        { r0: '46%', r: '85%', label: { fontSize: 10, rotate: 'tangential', align: 'right' } },
      ],
    }],
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tab switching + rendering
// ─────────────────────────────────────────────────────────────────────────────
async function switchTab(tab: ChartTab) {
  activeTab.value = tab
  await nextTick()
  if (tab === 'cog') renderCog()
  if (tab === 'go')  renderGo()
  if (tab === 'ec')  renderEc()
}

// Check data availability
const hasCog = computed(() => (props.stats?.cog_categories?.length ?? 0) > 0)
const hasGo  = computed(() => {
  const g = props.stats?.go_terms
  return g && (g.biological_process.length + g.molecular_function.length + g.cellular_component.length) > 0
})
const hasEc = computed(() => props.allSequences.some(s => s.ec_ids))

// Resize handler
function onResize() {
  cogChart?.resize()
  goChart?.resize()
  ecChart?.resize()
}

// ─────────────────────────────────────────────────────────────────────────────
// Lifecycle
// ─────────────────────────────────────────────────────────────────────────────
onMounted(async () => {
  if (props.jobStatus !== 'completed' || !props.stats) return

  // Resolve GO labels
  const ids = new Set<string>()
  const g = props.stats.go_terms
  for (const item of [...g.biological_process, ...g.molecular_function, ...g.cellular_component]) {
    if (!goLabels.value[item.name]) ids.add(item.name)
  }
  if (ids.size) fetchGoLabels([...ids])

  await nextTick()
  renderCog()
  window.addEventListener('resize', onResize)
})

watch(() => props.stats, async (s) => {
  if (!s) return
  const ids = new Set<string>()
  const g = s.go_terms
  for (const item of [...g.biological_process, ...g.molecular_function, ...g.cellular_component]) {
    if (!goLabels.value[item.name]) ids.add(item.name)
  }
  if (ids.size) fetchGoLabels([...ids])
  await nextTick()
  if (activeTab.value === 'cog') renderCog()
  if (activeTab.value === 'go')  renderGo()
  if (activeTab.value === 'ec')  renderEc()
})

watch(goLabels, async () => {
  if (activeTab.value === 'go') {
    await nextTick()
    renderGo()
  }
}, { deep: true })

onUnmounted(() => {
  window.removeEventListener('resize', onResize)
  cogChart?.dispose()
  goChart?.dispose()
  ecChart?.dispose()
})
</script>

<template>
  <div class="viz-tab">

    <!-- Not ready -->
    <div v-if="jobStatus !== 'completed' || !stats" class="viz-empty">
      <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24"
           fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3">
        <circle cx="12" cy="12" r="10"/><path d="M12 8v4m0 4h.01"/>
      </svg>
      <p>Visualization is available once the job is completed.</p>
    </div>

    <template v-else>
      <!-- Tab navigation -->
      <div class="viz-tabs">
        <button class="viz-tab-btn" :class="{ active: activeTab === 'cog', disabled: !hasCog }"
                :disabled="!hasCog" @click="hasCog && switchTab('cog')">
          COG Categories
        </button>
        <button class="viz-tab-btn" :class="{ active: activeTab === 'go', disabled: !hasGo }"
                :disabled="!hasGo" @click="hasGo && switchTab('go')">
          Gene Ontology (GO)
          <span v-if="goLabelsLoading" class="viz-tab-spinner"></span>
        </button>
        <button class="viz-tab-btn" :class="{ active: activeTab === 'ec', disabled: !hasEc }"
                :disabled="!hasEc" @click="hasEc && switchTab('ec')">
          Enzyme Classes (EC)
        </button>
      </div>

      <!-- COG -->
      <div v-show="activeTab === 'cog'" class="viz-chart-panel">
        <div v-if="!hasCog" class="viz-no-data">No COG annotations found in this job.</div>
        <template v-else>
          <p class="viz-hint">
            Click a super-category to expand. Hover for counts. COG assignments from
            <code>cog_category</code> field across {{ stats.total_sequences.toLocaleString() }} sequences.
          </p>
          <div ref="cogContainer" class="viz-chart"></div>
        </template>
      </div>

      <!-- GO -->
      <div v-show="activeTab === 'go'" class="viz-chart-panel">
        <div v-if="!hasGo" class="viz-no-data">No GO term annotations found in this job.</div>
        <template v-else>
          <p class="viz-hint">
            Three root ontologies (BP / MF / CC) shown as inner ring; individual GO terms as outer ring.
            Labels resolved via QuickGO. Click to drill down, hover for counts.
            <span v-if="goLabelsLoading" class="viz-hint-loading">Resolving GO labels…</span>
          </p>
          <div ref="goContainer" class="viz-chart"></div>
        </template>
      </div>

      <!-- EC -->
      <div v-show="activeTab === 'ec'" class="viz-chart-panel">
        <div v-if="!hasEc" class="viz-no-data">No EC number annotations found in this job.</div>
        <template v-else>
          <p class="viz-hint">
            Enzyme classes (EC 1–7) as inner ring; sub-classes (EC x.y) as outer ring.
            Counts from <code>ec_ids</code> across all annotated sequences.
          </p>
          <div ref="ecContainer" class="viz-chart"></div>
        </template>
      </div>
    </template>
  </div>
</template>

<style scoped>
.viz-tab { display: flex; flex-direction: column; gap: 1rem; }

.viz-empty { display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: .75rem; padding: 4rem 2rem; text-align: center; color: var(--color-text); opacity: .6; }

/* Tab bar */
.viz-tabs { display: flex; gap: .4rem; border-bottom: 1px solid var(--color-border); }
.viz-tab-btn { display: inline-flex; align-items: center; gap: .5rem; padding: .6rem 1.1rem;
  border: none; background: transparent; color: var(--color-text); font-size: .875rem;
  cursor: pointer; border-bottom: 2px solid transparent; margin-bottom: -1px;
  transition: all .15s; border-radius: 4px 4px 0 0; }
.viz-tab-btn:hover:not(.disabled) { color: hsla(160,100%,37%,1); }
.viz-tab-btn.active { color: hsla(160,100%,37%,1); border-bottom-color: hsla(160,100%,37%,1); font-weight: 600; }
.viz-tab-btn.disabled { opacity: .35; cursor: not-allowed; }
.viz-tab-spinner { width: 10px; height: 10px; border: 1.5px solid var(--color-border);
  border-top-color: hsla(160,100%,37%,1); border-radius: 50%; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

/* Chart panel */
.viz-chart-panel { display: flex; flex-direction: column; gap: .75rem; }
.viz-chart { width: 100%; height: 560px; }
.viz-no-data { padding: 3rem; text-align: center; color: var(--color-text); opacity: .5; font-size: .9rem; }

/* Hint text */
.viz-hint { margin: 0; font-size: .8rem; color: var(--color-text); opacity: .65; line-height: 1.5; }
.viz-hint code { font-family: monospace; background: var(--color-background-mute);
  padding: .1em .35em; border-radius: 3px; font-size: .85em; }
.viz-hint-loading { font-style: italic; opacity: .7; }
</style>