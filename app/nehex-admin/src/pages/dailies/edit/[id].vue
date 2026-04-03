<template>
  <DailyEditorWorkspace :daily-id="dailyId" />
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import DailyEditorWorkspace from '@/components/admin/DailyEditorWorkspace.vue'

const route = useRoute()

const dailyId = computed<number | null>(() => {
  const rawParam = 'id' in route.params ? route.params.id : undefined
  const normalized = Array.isArray(rawParam) ? rawParam[0] : rawParam
  const raw = Number(normalized)
  if (!Number.isFinite(raw) || raw <= 0) {
    return null
  }
  return raw
})
</script>
