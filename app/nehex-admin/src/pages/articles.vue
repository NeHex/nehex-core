<template>
  <AdminLayout>
    <section v-if="isManageRoute" class="articles-page">
      <header class="page-header">
        <div class="header-text">
          <h1>文章管理</h1>
          <p>读取文章并通过卡片快速编辑、删除和新增。</p>
        </div>
        <v-btn
          class="quick-create-btn"
          color="primary"
          prepend-icon="mdi-plus"
          variant="flat"
          @click="openCreatePage"
        >
          新增文章
        </v-btn>
      </header>

      <v-alert
        v-if="errorMessage"
        class="mb-4"
        density="comfortable"
        type="error"
        variant="tonal"
      >
        {{ errorMessage }}
      </v-alert>

      <v-progress-linear
        v-if="loading"
        class="mb-4"
        color="primary"
        indeterminate
      />

      <div class="list-head">
        <span>共 {{ totalArticles }} 篇文章</span>
      </div>

      <div class="articles-grid">
        <v-card
          v-for="article in articles"
          :key="article.id"
          class="article-card"
          :style="getArticleCardStyle(article)"
          rounded="xl"
        >
          <div class="article-overlay">
            <div class="card-actions">
              <v-btn
                class="icon-btn"
                color="white"
                icon="mdi-pencil-outline"
                size="small"
                variant="text"
                @click.stop="openEditPage(article)"
              />
              <v-btn
                class="icon-btn"
                color="error"
                icon="mdi-delete-outline"
                size="small"
                variant="text"
                @click.stop="openDeleteDialog(article)"
              />
            </div>

            <div class="card-footer">
              <div class="article-title">{{ article.title }}</div>
            </div>
          </div>
        </v-card>

        <v-card
          class="add-card"
          rounded="xl"
          @click="openCreatePage"
        >
          <v-icon class="add-icon" icon="mdi-plus-circle-outline" size="40" />
          <div class="add-label">新增文章</div>
        </v-card>
      </div>

      <div v-if="totalPages > 1" class="pagination-row">
        <v-pagination
          v-model="currentPage"
          :disabled="loading"
          :length="totalPages"
          density="comfortable"
          rounded="circle"
          :total-visible="7"
        />
      </div>
    </section>

    <v-dialog v-if="isManageRoute" v-model="deleteDialog" max-width="420">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title class="dialog-title">确认删除</v-card-title>
        <v-card-text>
          即将删除文章《{{ pendingDelete?.title || '' }}》，删除后不可恢复。
        </v-card-text>
        <v-card-actions class="dialog-actions">
          <v-spacer />
          <v-btn variant="text" @click="closeDeleteDialog">取消</v-btn>
          <v-btn color="error" :loading="deleting" @click="confirmDelete">
            确认删除
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <RouterView v-else />
  </AdminLayout>
</template>

<script lang="ts" setup>
import AdminLayout from '@/components/admin/AdminLayout.vue'
import { computed, onMounted, ref, watch } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import {
  deleteArticle,
  fetchArticles,
  type ArticleItem,
} from '@/services/articles'

const router = useRouter()
const route = useRoute()
const isManageRoute = computed(() => route.path === '/articles')

const loading = ref(false)
const deleting = ref(false)
const errorMessage = ref('')
const currentPage = ref(1)
const pageSize = 24
const totalArticles = ref(0)
const totalPages = ref(0)

const articles = ref<ArticleItem[]>([])
const deleteDialog = ref(false)
const pendingDelete = ref<ArticleItem | null>(null)

function openCreatePage(): void {
  void router.push('/articles/new')
}

function openEditPage(article: ArticleItem): void {
  void router.push(`/articles/edit/${article.id}`)
}

function openDeleteDialog(article: ArticleItem): void {
  pendingDelete.value = article
  deleteDialog.value = true
}

function closeDeleteDialog(force = false): void {
  if (deleting.value && !force) {
    return
  }
  deleteDialog.value = false
  pendingDelete.value = null
}

async function loadArticles(targetPage = currentPage.value): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  try {
    const result = await fetchArticles(targetPage, pageSize)
    articles.value = result.items
    totalArticles.value = result.pagination.total
    totalPages.value = result.pagination.total_pages
    currentPage.value = result.pagination.page

    if (
      result.items.length === 0
      && result.pagination.total_pages > 0
      && targetPage > result.pagination.total_pages
    ) {
      currentPage.value = result.pagination.total_pages
      return
    }
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '加载文章失败'
  } finally {
    loading.value = false
  }
}

async function confirmDelete(): Promise<void> {
  if (!pendingDelete.value) {
    return
  }

  deleting.value = true
  errorMessage.value = ''
  try {
    await deleteArticle(pendingDelete.value.id)
    closeDeleteDialog(true)
    await loadArticles(currentPage.value)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '删除文章失败'
  } finally {
    deleting.value = false
  }
}

function getArticleCardStyle(article: ArticleItem): Record<string, string> {
  const raw = (article.articleTopImage || '').trim()
  if (!raw) {
    return {
      background: 'linear-gradient(140deg, #1e2433 0%, #131827 100%)',
    }
  }

  const safeUrl = raw.replace(/"/g, '\\"')
  return {
    backgroundImage: `url("${safeUrl}")`,
    backgroundPosition: 'center',
    backgroundRepeat: 'no-repeat',
    backgroundSize: 'cover',
  }
}

onMounted(async () => {
  if (isManageRoute.value) {
    await loadArticles(1)
  }
})

watch(isManageRoute, async (active, previous) => {
  if (active && !previous) {
    currentPage.value = 1
    await loadArticles(1)
  }
})

watch(currentPage, async (page, previous) => {
  if (page === previous || loading.value || !isManageRoute.value) {
    return
  }
  await loadArticles(page)
})
</script>

<style scoped>
.articles-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.page-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 14px;
}

.header-text h1 {
  margin: 0;
  font-size: 28px;
  color: #f1f4ff;
}

.header-text p {
  margin: 6px 0 0;
  color: #aeb8cc;
}

.quick-create-btn {
  flex-shrink: 0;
}

.list-head {
  color: #aeb8cc;
  font-size: 14px;
}

.articles-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 16px;
}

.article-card,
.add-card {
  position: relative;
  min-height: 240px;
  border: 1px solid rgba(255, 255, 255, 0.12);
}

.article-card {
  overflow: hidden;
  transition:
    transform 0.24s ease,
    box-shadow 0.24s ease;
}

.article-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 16px 30px rgba(0, 0, 0, 0.28);
}

.article-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 12px;
}

.card-actions {
  display: flex;
  justify-content: flex-end;
  gap: 4px;
  position: relative;
  z-index: 1;
}

.icon-btn {
  backdrop-filter: blur(2px);
  border-radius: 10px;
}

.card-footer {
  display: flex;
  align-items: flex-end;
  position: relative;
  z-index: 1;
}

.article-title {
  font-size: 18px;
  font-weight: 700;
  color: #ffffff;
  text-shadow: 0 2px 12px rgba(0, 0, 0, 0.65);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pagination-row {
  display: flex;
  justify-content: center;
  padding: 4px 0 8px;
}

.add-card {
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 8px;
  border-style: dashed;
  color: #b8c5e6;
  background: linear-gradient(140deg, rgba(28, 34, 47, 0.92), rgba(19, 24, 36, 0.92));
  transition:
    border-color 0.2s ease,
    color 0.2s ease,
    filter 0.2s ease;
}

.add-card:hover {
  border-color: rgba(255, 255, 255, 0.34);
  color: #e8efff;
  filter: brightness(1.04);
}

.add-icon {
  opacity: 0.95;
  font-size: 52px !important;
}

.add-label {
  font-size: 18px;
  font-weight: 600;
}

.dialog-card {
  background: linear-gradient(180deg, #151c2a, #121826);
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.dialog-title {
  font-size: 20px;
  font-weight: 700;
}

.dialog-actions {
  padding: 12px 16px 16px;
}

@media (max-width: 900px) {
  .page-header {
    align-items: flex-start;
    flex-direction: column;
  }

  .quick-create-btn {
    width: 100%;
  }
}
</style>
