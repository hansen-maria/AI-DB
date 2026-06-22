<script setup lang="ts">
/**
 * VisualizationTab.vue
 *
 * IGV Genome Browser – loads FASTA + per-type GFF3 tracks as Blob-URLs
 * from AI-DB download endpoints. Tracks with 0 features are skipped.
 */
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'

const props = defineProps<{
  jobId:     string
  jobStatus: string
}>()

const IGV_CDN  = 'https://cdn.jsdelivr.net/npm/igv@2.15.11/dist/igv.min.js'
const API_BASE = '/api'

// ─────────────────────────────────────────────────────────────────────────────
// Fetch helpers
// ─────────────────────────────────────────────────────────────────────────────
async function fetchText(format: 'gff3' | 'fasta'): Promise<string> {
  const r = await fetch(`${API_BASE}/job/${props.jobId}/download/${format}`, { credentials: 'include' })
  if (!r.ok) throw new Error(`HTTP ${r.status} fetching ${format}`)
  return r.text()
}

async function textToBlob(text: string): Promise<string> {
  return URL.createObjectURL(new Blob([text], { type: 'text/plain' }))
}

async function loadScript(src: string, globalKey: string): Promise<void> {
  if ((window as any)[globalKey]) return
  return new Promise((resolve, reject) => {
    if (document.querySelector(`script[src="${src}"]`)) {
      const poll = () => (window as any)[globalKey] ? resolve() : setTimeout(poll, 40)
      poll(); return
    }
    const s = document.createElement('script')
    s.src = src; s.onload = () => resolve(); s.onerror = () => reject(new Error(`Failed: ${src}`))
    document.head.appendChild(s)
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// GFF3 parser
// ─────────────────────────────────────────────────────────────────────────────
interface GffFeature {
  seqid:  string
  type:   string
  start:  number
  end:    number
  strand: '+' | '-' | '.'
  attrs:  Record<string, string>
}
interface GenomeInfo {
  contigs:  Map<string, number>
  features: GffFeature[]
  lines:    string[]
}

function parseGff3(text: string): GenomeInfo {
  const features: GffFeature[] = []
  const contigs  = new Map<string, number>()
  const lines: string[] = []

  for (const raw of text.split('\n')) {
    const line = raw.trim()
    if (!line) continue
    if (line.startsWith('##sequence-region')) {
      const p = line.split(/\s+/)
      if (p.length >= 4) contigs.set(p[1], parseInt(p[3], 10))
      continue
    }
    if (line.startsWith('#')) continue
    const cols = line.split('\t')
    if (cols.length < 9) continue
    lines.push(line)

    const seqid  = cols[0]
    const type   = cols[2]
    const start  = parseInt(cols[3], 10)
    const end    = parseInt(cols[4], 10)
    const strand = cols[6] === '+' ? '+' : cols[6] === '-' ? '-' : '.' as any

    if (end > (contigs.get(seqid) ?? 0)) contigs.set(seqid, end)

    const attrs: Record<string, string> = {}
    for (const pair of cols[8].split(';')) {
      const eq = pair.indexOf('=')
      if (eq > 0) attrs[pair.slice(0, eq).trim()] = decodeURIComponent(pair.slice(eq + 1).trim())
    }
    features.push({ seqid, type, start, end, strand, attrs })
  }
  return { contigs, features, lines }
}

function filterGff3Lines(lines: string[], types: Set<string>): string {
  return '##gff-version 3\n' + lines.filter(l => {
    const c = l.split('\t'); return c.length >= 3 && types.has(c[2])
  }).join('\n')
}

// ─────────────────────────────────────────────────────────────────────────────
// IGV
// ─────────────────────────────────────────────────────────────────────────────
const igvContainer = ref<HTMLDivElement | null>(null)
const igvLoading   = ref(false)
const igvError     = ref('')
const igvMounted   = ref(false)

let igvBrowser: any = null
const trackBlobUrls: string[] = []
let   fastaBlobUrl = ''
let   cachedGenome: GenomeInfo | null = null

const TRACK_GROUPS: Array<{ name: string; color: string; types: string[] }> = [
  { name: 'All Annotations',     color: '#607d8b', types: [] },
  { name: 'CDS / sORF',          color: '#4caf50', types: ['CDS', 'cds', 'sORF'] },
  { name: 'tRNA / tmRNA / rRNA', color: '#2196f3', types: ['tRNA', 'tmRNA', 'rRNA'] },
  { name: 'ncRNA',               color: '#9c27b0', types: ['ncRNA'] },
  { name: 'ncRNA-region',        color: '#ce93d8', types: ['ncRNA_region', 'ncRNA-region'] },
  { name: 'CRISPR',              color: '#f44336', types: ['CRISPR', 'crispr', 'repeat_region'] },
  { name: 'Gap',                 color: '#9e9e9e', types: ['gap'] },
  { name: 'oriC / oriV / oriT',  color: '#ff9800', types: ['oriC', 'oriV', 'oriT'] },
]

async function initIGV() {
  if (!igvContainer.value) return
  igvLoading.value = true
  igvError.value   = ''

  try {
    await loadScript(IGV_CDN, 'igv')
    const igv = (window as any).igv

    if (igvBrowser) {
      try { igvBrowser.removeAllTracks?.() } catch { /**/ }
      igvContainer.value.innerHTML = ''
      igvBrowser = null
    }
    trackBlobUrls.forEach(u => URL.revokeObjectURL(u))
    trackBlobUrls.length = 0
    if (fastaBlobUrl) { URL.revokeObjectURL(fastaBlobUrl); fastaBlobUrl = '' }

    const [fastaText, gff3Text] = await Promise.all([fetchText('fasta'), fetchText('gff3')])
    if (!cachedGenome) cachedGenome = parseGff3(gff3Text)
    const genome = cachedGenome

    fastaBlobUrl = await textToBlob(fastaText)
    trackBlobUrls.push(fastaBlobUrl)

    const tracks: any[] = []
    for (const grp of TRACK_GROUPS) {
      const filtered = grp.types.length === 0
          ? gff3Text
          : filterGff3Lines(genome.lines, new Set(grp.types))

      if (grp.types.length > 0) {
        const featureCount = genome.features.filter(f => grp.types.includes(f.type)).length
        if (featureCount === 0) continue
      }

      const url = await textToBlob(filtered)
      trackBlobUrls.push(url)
      tracks.push({
        type:             'annotation',
        format:           'gff3',
        url,
        name:             grp.name,
        color:            grp.color,
        displayMode:      grp.name === 'All Annotations' ? 'SQUISHED' : 'EXPANDED',
        indexed:          false,
        visibilityWindow: -1,
      })
    }

    igvBrowser = await igv.createBrowser(igvContainer.value, {
      reference: {
        id: props.jobId, name: 'AI-DB Assembly',
        fastaURL: fastaBlobUrl, indexed: false,
      },
      locus:  [...genome.contigs.keys()][0] || undefined,
      tracks,
      showNavigation: true, showRuler: true,
      showCenterGuide: false, showCursorTrackingGuide: false,
    })

    igvMounted.value = true
  } catch (e) {
    igvError.value = e instanceof Error ? e.message : 'IGV init failed'
  } finally {
    igvLoading.value = false
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Lifecycle
// ─────────────────────────────────────────────────────────────────────────────
onMounted(() => {
  if (props.jobStatus === 'completed') nextTick(() => initIGV())
})

watch(() => props.jobId, () => {
  cachedGenome     = null
  igvMounted.value = false
  igvError.value   = ''
  igvBrowser       = null
  nextTick(() => initIGV())
})

onUnmounted(() => {
  try { igvBrowser?.removeAllTracks?.() } catch { /**/ }
  trackBlobUrls.forEach(u => URL.revokeObjectURL(u))
  if (fastaBlobUrl) URL.revokeObjectURL(fastaBlobUrl)
})
</script>

<template>
  <div class="viz-tab">

    <div v-if="jobStatus !== 'completed'" class="viz-empty">
      <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24"
           fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3">
        <circle cx="12" cy="12" r="10"/><path d="M12 8v4m0 4h.01"/>
      </svg>
      <p>Visualization is available once the job is completed.</p>
    </div>

    <template v-else>
      <div class="viz-igv-note">
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24"
             fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/><path d="M12 16v-4m0-4h.01"/>
        </svg>
        FASTA and GFF3 are loaded directly from AI-DB. Feature types with no annotations
        are automatically hidden. Large assemblies may take a few seconds to parse.
      </div>

      <div v-if="igvLoading" class="viz-loading">
        <div class="spinner"></div>
        <span>Fetching FASTA + GFF3 and building tracks…</span>
      </div>

      <div v-if="igvError" class="viz-error">
        <p>{{ igvError }}</p>
        <button class="viz-btn-secondary" @click="initIGV">Retry</button>
      </div>

      <div ref="igvContainer" class="viz-igv-container"></div>
    </template>
  </div>
</template>

<style scoped>
.viz-tab { display: flex; flex-direction: column; gap: 1.25rem; }

.viz-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: .75rem; padding: 4rem 2rem; text-align: center; color: var(--color-text); opacity: .6; }

.viz-loading { display: flex; align-items: center; gap: .75rem; padding: 2rem; color: var(--color-text); opacity: .8; }
.spinner { width: 22px; height: 22px; border: 2px solid var(--color-border); border-top-color: hsla(160,100%,37%,1); border-radius: 50%; animation: spin .8s linear infinite; flex-shrink: 0; }
@keyframes spin { to { transform: rotate(360deg); } }

.viz-error { background: rgba(244,67,54,.08); border: 1px solid rgba(244,67,54,.25); border-radius: 8px; padding: 1rem 1.25rem; color: var(--color-text); font-size: .875rem; }
.viz-error p { margin: 0 0 .35rem; }

.viz-btn-secondary { display: inline-flex; align-items: center; gap: .4rem; padding: .45rem .9rem; background: transparent; color: var(--color-text); border: 1px solid var(--color-border); border-radius: 7px; font-size: .82rem; cursor: pointer; transition: border-color .15s, color .15s; }
.viz-btn-secondary:hover { border-color: hsla(160,100%,37%,.6); color: hsla(160,100%,37%,1); }

.viz-igv-note { display: flex; align-items: flex-start; gap: .5rem; padding: .55rem .85rem; background: rgba(2,128,144,.07); border: 1px solid rgba(2,128,144,.2); border-radius: 7px; font-size: .8rem; color: var(--color-text); line-height: 1.5; }
.viz-igv-note svg { flex-shrink: 0; color: #028090; margin-top: 1px; }

.viz-igv-container { min-height: 520px; border: 1px solid var(--color-border); border-radius: 10px; overflow: hidden; background: var(--color-background); }
.viz-igv-container :deep(.igv-navbar) { border-radius: 10px 10px 0 0; }
</style>