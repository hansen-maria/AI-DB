<script setup lang="ts">
import { onMounted } from 'vue'
import { RouterLink, useRoute } from 'vue-router'
import { trackNotFound } from '../api/analytics.ts'

const route = useRoute()

// The SPA always answers with HTTP 200, so Plausible can't detect 404s by itself.
onMounted(() => trackNotFound(route.path))
</script>

<template>
  <div class="not-found-page">
    <h2>Page not found</h2>
    <p>The page <code>{{ route.path }}</code> does not exist.</p>
    <RouterLink to="/" class="btn btn-primary">Back to start page</RouterLink>
  </div>
</template>

<style scoped>
.not-found-page {
  max-width: 640px;
  margin: 3rem auto;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 1rem;
}

.not-found-page h2 {
  font-size: 1.75rem;
  font-weight: 700;
  color: var(--color-heading);
}
</style>
