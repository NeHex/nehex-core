<template>
  <AdminLayout>
    <section v-if="isManageRoute" class="pages-page">
      <header class="page-header">
        <div class="header-text">
          <h1>独立页管理</h1>
          <p>读取独立页并通过卡片快速编辑、删除和新增。</p>
        </div>
        <v-btn
          class="quick-create-btn"
          color="primary"
          prepend-icon="mdi-plus"
          variant="flat"
          @click="openCreatePage"
        >
          新增页面
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

      <div class="pages-grid">
        <v-card
          v-for="page in pages"
          :key="page.id"
          class="page-card"
          :style="getPageCardStyle(page)"
          rounded="xl"
        >
          <div class="page-overlay">
            <div class="card-actions">
              <v-btn
                class="icon-btn"
                color="white"
                icon="mdi-pencil-outline"
                size="small"
                variant="text"
                @click.stop="openEditPage(page)"
              />
              <v-btn
                class="icon-btn"
                color="error"
                icon="mdi-delete-outline"
                size="small"
                variant="text"
                @click.stop="openDeleteDialog(page)"
              />
            </div>

            <div class="card-footer">
              <div class="page-title">{{ page.title }}</div>
              <div class="page-meta">
                <span>/{{ page.page_key }}</span>
                <span>{{ page.status > 0 ? '启用' : '禁用' }}</span>
              </div>
            </div>
          </div>
        </v-card>

        <v-card
          class="add-card"
          rounded="xl"
          @click="openCreatePage"
        >
          <v-icon class="add-icon" icon="mdi-plus-circle-outline" size="40" />
          <div class="add-label">新增页面</div>
        </v-card>
      </div>
    </section>

    <v-dialog v-if="isManageRoute" v-model="deleteDialog" max-width="420">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title class="dialog-title">确认删除</v-card-title>
        <v-card-text>
          即将删除独立页《{{ pendingDelete?.title || '' }}》，删除后不可恢复。
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
import { computed, onMounted, ref, watch } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import {
  deleteStandalonePage,
  fetchStandalonePages,
  type StandalonePageItem,
} from '@/services/pages'

const router = useRouter()
const route = useRoute()
const isManageRoute = computed(() => route.path === '/pages')

const loading = ref(false)
const deleting = ref(false)
const errorMessage = ref('')

const pages = ref<StandalonePageItem[]>([])
const deleteDialog = ref(false)
const pendingDelete = ref<StandalonePageItem | null>(null)

function openCreatePage(): void {
  void router.push('/pages/new')
}

function openEditPage(page: StandalonePageItem): void {
  void router.push(`/pages/edit/${page.id}`)
}

function openDeleteDialog(page: StandalonePageItem): void {
  pendingDelete.value = page
  deleteDialog.value = true
}

function closeDeleteDialog(force = false): void {
  if (deleting.value && !force) {
    return
  }
  deleteDialog.value = false
  pendingDelete.value = null
}

async function loadPages(): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  try {
    pages.value = await fetchStandalonePages()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '加载独立页失败'
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
    await deleteStandalonePage(pendingDelete.value.id)
    closeDeleteDialog(true)
    await loadPages()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '删除独立页失败'
  } finally {
    deleting.value = false
  }
}

function getPageCardStyle(page: StandalonePageItem): Record<string, string> {
  const raw = (page.cover_image || '').trim()
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
    await loadPages()
  }
})

watch(isManageRoute, async (active, previous) => {
  if (active && !previous) {
    await loadPages()
  }
})
</script>

<style scoped>
.pages-page {
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

.pages-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 16px;
}

.page-card,
.add-card {
  position: relative;
  min-height: 240px;
  border: 1px solid rgba(255, 255, 255, 0.12);
}

.page-card {
  overflow: hidden;
  transition:
    transform 0.24s ease,
    box-shadow 0.24s ease;
}

.page-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 16px 30px rgba(0, 0, 0, 0.28);
}

.page-overlay {
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
}

.icon-btn {
  border-radius: 10px;
}

.card-footer {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
}

.page-title {
  font-size: 20px;
  font-weight: 700;
  color: #ffffff;
  text-shadow: 0 2px 12px rgba(0, 0, 0, 0.65);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.page-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #d4dfff;
  font-size: 13px;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.6);
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
