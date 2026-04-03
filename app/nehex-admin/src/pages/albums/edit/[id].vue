<template>
  <AlbumEditorWorkspace :album-id="albumId" />
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import AlbumEditorWorkspace from '@/components/admin/AlbumEditorWorkspace.vue'

const route = useRoute()

const albumId = computed<number | null>(() => {
  const rawParam = 'id' in route.params ? route.params.id : undefined
  const normalized = Array.isArray(rawParam) ? rawParam[0] : rawParam
  const raw = Number(normalized)
  if (!Number.isFinite(raw) || raw <= 0) {
    return null
  }
  return raw
})
</script>
