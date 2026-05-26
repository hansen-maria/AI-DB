<script setup lang="ts">
import type { FunctionalStats } from '../../api/jobs.ts'
import { getSequentialColor, getCategoricalColor } from '../../constants/sequences.ts'

const props = defineProps<{
  loading: boolean
  stats:   FunctionalStats | null
}>()

const annotationRate = computed(() => {
  if (!props.stats || props.stats.total_sequences === 0) return 0
  return Math.round((props.stats.annotated_sequences / props.stats.total_sequences) * 100)
})

import { computed } from 'vue'
</script>

<template>
  <div class="tab-panel">
    <div v-if="loading" class="loading-stats">
      <div class="spinner"></div> Loading functional analysis...
    </div>

    <div v-else-if="stats" class="analysis-section">
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

        <!-- GO Terms -->
        <div v-if="stats.go_terms.molecular_function.length > 0" class="chart-card chart-card-wide">
          <h4>Gene Ontology (GO) Terms</h4>
          <div class="go-items">
            <div v-for="item in stats.go_terms.molecular_function.slice(0, 15)" :key="item.name" class="go-item">
              <span class="go-id">{{ item.name }}</span>
              <span class="go-count">{{ item.count }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .loading-stats { display: flex; align-items: center; gap: 0.75rem; padding: 2rem; color: var(--color-text); }
  .spinner { width: 24px; height: 24px; border: 2px solid var(--color-border); border-top-color: hsla(160,100%,37%,1); border-radius: 50%; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
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
  .go-items { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 0.4rem; }
  .go-item { display: flex; justify-content: space-between; padding: 0.35rem 0.6rem; background: var(--color-background); border-radius: 5px; font-size: 0.8rem; }
  .go-id { color: var(--color-text); font-family: monospace; }
  .go-count { font-weight: 600; color: var(--color-heading); }
  @media (max-width: 900px) { .charts-grid { grid-template-columns: 1fr; } }
  @media (max-width: 600px) {
    .annotation-rate { flex-direction: column; text-align: center; }
    .bar-item { grid-template-columns: 1fr; gap: 0.25rem; }
  }
</style>
