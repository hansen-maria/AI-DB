<script setup lang="ts">
// Link to the de.NBI user survey. Every placement is tracked separately in Plausible,
// so the service report can show where users find the survey.
import { DENBI_SURVEY_URL } from '@/constants/contact'
import { trackSurveyClick, type SurveyPlacement } from '@/api/analytics'

const props = defineProps<{
  placement: SurveyPlacement
}>()
</script>

<template>
  <a
    :href="DENBI_SURVEY_URL"
    target="_blank"
    rel="noopener noreferrer"
    @click="trackSurveyClick(props.placement)"
  ><slot>short user survey</slot></a>
</template>
