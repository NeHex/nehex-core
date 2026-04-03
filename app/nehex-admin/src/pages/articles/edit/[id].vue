<template>
  <ArticleEditorWorkspace :article-id="articleId" />
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import ArticleEditorWorkspace from '@/components/admin/ArticleEditorWorkspace.vue'

const route = useRoute()

const articleId = computed<number | null>(() => {
  const rawParam = 'id' in route.params ? route.params.id : undefined
  const normalized = Array.isArray(rawParam) ? rawParam[0] : rawParam
  const raw = Number(normalized)
  if (!Number.isFinite(raw) || raw <= 0) {
    return null
  }
  return raw
})
</script>
