import { ref, computed, watch, type Ref } from 'vue'
import { filterOptions, cogCategories, ecClasses } from '../constants/sequences.ts'
import type { SequenceFilter, FilteredDownloadFormat, PaginatedJobResponse } from '../api/jobs.ts'

const PER_PAGE = 20

/**
 * Provides all client-side filtering, pagination, and download logic for the
 * sequence table. Accepts `allSequences` as a reactive reference so it stays
 * in sync with the data source (the polling composable).
 */
export function useSequenceFilters(
  allSequences: Ref<any[]>,
  job: Ref<PaginatedJobResponse | null>,
) {
  // ── Filter state ───────────────────────────────────────────────────────────

  const currentFilter    = ref<SequenceFilter>('all')
  const searchText       = ref('')
  const debouncedSearch  = ref('')
  const minLength        = ref<number | undefined>(undefined)
  const maxLength        = ref<number | undefined>(undefined)
  const selectedCog      = ref('')
  const selectedEcClass  = ref('')
  const hasGeneOnly      = ref(false)
  const hasProductOnly   = ref(false)
  const showAdvancedFilters = ref(false)
  const currentPage      = ref(1)

  // ── Debounce search ────────────────────────────────────────────────────────

  let searchTimeout: number | null = null
  watch(searchText, (val) => {
    if (searchTimeout) clearTimeout(searchTimeout)
    searchTimeout = window.setTimeout(() => { debouncedSearch.value = val }, 80)
  })

  // ── Derived data ───────────────────────────────────────────────────────────

  const filteredSequences = computed(() => {
    if (!allSequences.value.length) return []

    return allSequences.value.filter(seq => {
      // Status filter
      if (currentFilter.value === 'hash_match') {
        if (!seq.annotation_source) return false
      } else if (currentFilter.value === 'bakta_db') {
        if (seq.annotation_source !== 'bakta_db') return false
      } else if (currentFilter.value === 'aidb_db') {
        if (seq.annotation_source !== 'aidb_db') return false
      } else if (currentFilter.value === 'none') {
        if (seq.annotation_source) return false
      }

      // Text search
      if (debouncedSearch.value) {
        const s = debouncedSearch.value.toLowerCase()
        if (
          !seq.id?.toLowerCase().includes(s) &&
          !seq.gene?.toLowerCase().includes(s) &&
          !seq.product?.toLowerCase().includes(s)
        ) return false
      }

      // Length
      if (minLength.value !== undefined && seq.length < minLength.value) return false
      if (maxLength.value !== undefined && seq.length > maxLength.value) return false

      // COG
      if (selectedCog.value && (!seq.cog_category || !seq.cog_category.includes(selectedCog.value))) return false

      // EC
      if (selectedEcClass.value) {
        if (!seq.ec_ids) return false
        const hasEc = seq.ec_ids.split(',').some((e: string) => e.trim().startsWith(selectedEcClass.value))
        if (!hasEc) return false
      }

      // Boolean flags
      if (hasGeneOnly.value && (!seq.gene || seq.gene === '')) return false
      if (hasProductOnly.value && (!seq.product || seq.product === '' || seq.product === 'hypothetical protein')) return false

      return true
    })
  })

  const paginatedSequences = computed(() => {
    const start = (currentPage.value - 1) * PER_PAGE
    return filteredSequences.value.slice(start, start + PER_PAGE)
  })

  const pagination = computed(() => {
    const total      = filteredSequences.value.length
    const totalPages = Math.ceil(total / PER_PAGE) || 1
    return {
      page:        currentPage.value,
      per_page:    PER_PAGE,
      total_items: total,
      total_pages: totalPages,
      has_next:    currentPage.value < totalPages,
      has_prev:    currentPage.value > 1,
    }
  })

  const pageNumbers = computed(() => {
    const total   = pagination.value.total_pages
    const current = pagination.value.page
    let start = Math.max(1, current - 2)
    let end   = Math.min(total, current + 2)
    if (current <= 3)          end   = Math.min(5, total)
    if (current >= total - 2)  start = Math.max(1, total - 4)
    const pages: number[] = []
    for (let i = start; i <= end; i++) pages.push(i)
    return pages
  })

  const hasActiveFilters = computed(() =>
    debouncedSearch.value !== '' ||
    minLength.value !== undefined ||
    maxLength.value !== undefined ||
    selectedCog.value !== '' ||
    selectedEcClass.value !== '' ||
    hasGeneOnly.value ||
    hasProductOnly.value ||
    currentFilter.value !== 'all',
  )

  const activeFilterBadges = computed(() => {
    const badges: { label: string; type: string }[] = []
    if (currentFilter.value !== 'all') {
      const found = filterOptions.find(o => o.value === currentFilter.value)
      badges.push({ label: found?.label ?? currentFilter.value, type: 'status' })
    }
    if (debouncedSearch.value)
      badges.push({ label: `"${debouncedSearch.value}"`, type: 'search' })
    if (minLength.value !== undefined)
      badges.push({ label: `≥ ${minLength.value} aa`, type: 'length' })
    if (maxLength.value !== undefined)
      badges.push({ label: `≤ ${maxLength.value} aa`, type: 'length' })
    if (selectedCog.value) {
      const found = cogCategories.find(c => c.value === selectedCog.value)
      badges.push({ label: `COG ${found?.value ?? selectedCog.value}`, type: 'cog' })
    }
    if (selectedEcClass.value) {
      const found = ecClasses.find(e => e.value === selectedEcClass.value)
      badges.push({ label: found ? `EC ${found.value}` : `EC ${selectedEcClass.value}`, type: 'ec' })
    }
    if (hasGeneOnly.value)    badges.push({ label: 'Has Gene',     type: 'flag' })
    if (hasProductOnly.value) badges.push({ label: 'Has Function', type: 'flag' })
    return badges
  })

  // ── Actions ────────────────────────────────────────────────────────────────

  // Reset to page 1 whenever any filter changes
  watch(
    [debouncedSearch, currentFilter, minLength, maxLength, selectedCog, selectedEcClass, hasGeneOnly, hasProductOnly],
    () => { currentPage.value = 1 },
  )

  function goToPage(page: number) {
    if (page >= 1 && page <= pagination.value.total_pages) currentPage.value = page
  }

  function clearFilters() {
    currentFilter.value   = 'all'
    searchText.value      = ''
    debouncedSearch.value = ''
    minLength.value       = undefined
    maxLength.value       = undefined
    selectedCog.value     = ''
    selectedEcClass.value = ''
    hasGeneOnly.value     = false
    hasProductOnly.value  = false
    currentPage.value     = 1
  }

  function downloadFilteredSequences(format: FilteredDownloadFormat) {
    const seqs = filteredSequences.value
    if (!seqs.length) return

    const baseName = job.value?.filename
      ? job.value.filename.replace(/\.[^.]+$/, '')
      : job.value?.job_id ?? 'sequences'

    let content  = ''
    let mimeType = 'text/plain'

    if (format === 'tsv' || format === 'csv') {
      const sep = format === 'tsv' ? '\t' : ','
      const esc = (v: unknown) => {
        if (v == null) return ''
        const s = String(v)
        if (format === 'csv' && (s.includes(',') || s.includes('"') || s.includes('\n')))
          return '"' + s.replace(/"/g, '""') + '"'
        return s
      }
      const cols: [string, (s: any) => unknown][] = [
        ['ID',                s => s.id],
        ['Length',            s => s.length],
        ['Gene',              s => s.gene ?? ''],
        ['Product',           s => s.product ?? ''],
        ['COG Category',      s => s.cog_category ?? ''],
        ['EC Numbers',        s => s.ec_ids ?? ''],
        ['GO Terms',          s => s.go_ids ?? ''],
        ['Annotation Source', s => s.annotation_source ?? ''],
        ['UniRef100 ID',      s => s.uniref100_id ?? ''],
        ['UniParc ID',        s => s.uniparc_id ?? ''],
        ['NCBI NRP ID',       s => s.ncbi_nrp_id ?? ''],
      ]
      content  = cols.map(([h]) => esc(h)).join(sep) + '\n'
      content += seqs.map(s => cols.map(([, fn]) => esc(fn(s))).join(sep)).join('\n')
      mimeType = format === 'tsv' ? 'text/tab-separated-values' : 'text/csv'
    } else if (format === 'fasta') {
      content = seqs.map(s => {
        const parts = [`>${s.id}`]
        if (s.gene)              parts.push(`gene=${s.gene}`)
        if (s.product)           parts.push(`product=${s.product}`)
        if (s.cog_category)      parts.push(`COG=${s.cog_category}`)
        if (s.ec_ids)            parts.push(`EC=${s.ec_ids}`)
        if (s.annotation_source) parts.push(`source=${s.annotation_source}`)
        parts.push(`length=${s.length}`)
        const body = s.sequence
          ? (s.sequence.match(/.{1,60}/g) ?? [s.sequence]).join('\n')
          : '; sequence not available'
        return `${parts.join(' ')}\n${body}`
      }).join('\n')
    } else {
      content  = JSON.stringify(seqs, null, 2)
      mimeType = 'application/json'
    }

    const blob = new Blob([content], { type: `${mimeType};charset=utf-8;` })
    const url  = URL.createObjectURL(blob)
    const a    = Object.assign(document.createElement('a'), { href: url, download: `${baseName}_filtered.${format}` })
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    setTimeout(() => URL.revokeObjectURL(url), 1_000)
  }

  return {
    // State
    currentFilter, searchText, debouncedSearch,
    minLength, maxLength, selectedCog, selectedEcClass,
    hasGeneOnly, hasProductOnly, showAdvancedFilters, currentPage,
    // Derived
    filteredSequences, paginatedSequences, pagination, pageNumbers,
    hasActiveFilters, activeFilterBadges,
    // Actions
    goToPage, clearFilters, downloadFilteredSequences,
  }
}
