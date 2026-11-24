<script setup lang="ts">
// de.NBI logo linked to the de.NBI website.
import { DENBI_URL, DENBI_NAME } from '@/constants/contact'
import logoLight from '@/assets/denbi-logo.png'
import logoDark from '@/assets/denbi-logo-dark.png'

withDefaults(defineProps<{
  /** Rendered logo height in px */
  height?: number
}>(), {
  height: 40,
})
</script>

<template>
  <a :href="DENBI_URL" target="_blank" rel="noopener noreferrer" class="denbi-logo" :title="DENBI_NAME">
    <img :src="logoLight" :alt="DENBI_NAME" class="denbi-logo__img denbi-logo__img--light" :style="{ height: `${height}px` }" />
    <img :src="logoDark" alt="" aria-hidden="true" class="denbi-logo__img denbi-logo__img--dark" :style="{ height: `${height}px` }" />
  </a>
</template>

<style scoped>
.denbi-logo {
  display: inline-block;
  line-height: 0;
  flex-shrink: 0;
}

.denbi-logo__img {
  width: auto;
}

.denbi-logo__img--dark {
  display: none;
}

:global(html.dark) .denbi-logo__img--light {
  display: none;
}

:global(html.dark) .denbi-logo__img--dark {
  display: inline;
}
</style>
