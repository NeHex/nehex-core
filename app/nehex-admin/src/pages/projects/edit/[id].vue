<template>
  <ProjectEditorWorkspace :project-id="projectId" />
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import ProjectEditorWorkspace from '@/components/admin/ProjectEditorWorkspace.vue'

const route = useRoute()

const projectId = computed<number | null>(() => {
  const rawParam = 'id' in route.params ? route.params.id : undefined
  const normalized = Array.isArray(rawParam) ? rawParam[0] : rawParam
  const raw = Number(normalized)
  if (!Number.isFinite(raw) || raw <= 0) {
    return null
  }
  return raw
})
</script>
