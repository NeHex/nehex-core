<template>
  <AdminLayout>
    <section v-if="isManageRoute" class="dailies-page">
      <header class="page-header">
        <div class="header-text">
          <h1>日常管理</h1>
          <p>读取日常记录并通过卡片快速编辑、删除和新增。</p>
        </div>
        <v-btn
          class="quick-create-btn"
          color="primary"
          prepend-icon="mdi-plus"
          variant="flat"
          @click="openCreatePage"
        >
          新增日常
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

      <div class="dailies-grid">
        <v-card
          v-for="daily in dailies"
          :key="daily.id"
          class="daily-card"
          rounded="xl"
        >
          <div class="daily-content">
            <div class="card-header">
              <div class="daily-title">{{ daily.title }}</div>

              <div class="card-actions">
                <v-btn
                  class="icon-btn"
                  color="white"
                  icon="mdi-pencil-outline"
                  size="small"
                  variant="text"
                  @click.stop="openEditPage(daily)"
                />
                <v-btn
                  class="icon-btn"
                  color="error"
                  icon="mdi-delete-outline"
                  size="small"
                  variant="text"
                  @click.stop="openDeleteDialog(daily)"
                />
              </div>
            </div>

            <div class="daily-body">
              {{ formatContentPreview(daily.content) }}
            </div>

            <div class="card-footer">
              <div class="daily-meta">
                <span>{{ formatCreateDate(daily.create_time) }}</span>
                <span v-if="daily.weather" class="daily-weather">{{ daily.weather }}</span>
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
          <div class="add-label">新增日常</div>
        </v-card>
      </div>
    </section>

    <v-dialog v-if="isManageRoute" v-model="deleteDialog" max-width="420">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title class="dialog-title">确认删除</v-card-title>
        <v-card-text>
          即将删除日常《{{ pendingDelete?.title || '' }}》，删除后不可恢复。
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
  deleteDaily,
  fetchDailies,
  type DailyItem,
} from '@/services/dailies'

const router = useRouter()
const route = useRoute()
const isManageRoute = computed(() => route.path === '/dailies')

const loading = ref(false)
const deleting = ref(false)
const errorMessage = ref('')

const dailies = ref<DailyItem[]>([])
const deleteDialog = ref(false)
const pendingDelete = ref<DailyItem | null>(null)

function openCreatePage(): void {
  void router.push('/dailies/new')
}

function openEditPage(daily: DailyItem): void {
  void router.push(`/dailies/edit/${daily.id}`)
}

function openDeleteDialog(daily: DailyItem): void {
  pendingDelete.value = daily
  deleteDialog.value = true
}

function closeDeleteDialog(force = false): void {
  if (deleting.value && !force) {
    return
  }
  deleteDialog.value = false
  pendingDelete.value = null
}

function formatCreateDate(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }
  return date.toLocaleDateString('zh-CN')
}

function formatContentPreview(value: string | null | undefined): string {
  const content = (value || '').trim()
  if (!content) {
    return '暂无内容'
  }
  return content
}

async function loadDailies(): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  try {
    dailies.value = await fetchDailies()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '加载日常失败'
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
    await deleteDaily(pendingDelete.value.id)
    closeDeleteDialog(true)
    await loadDailies()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '删除日常失败'
  } finally {
    deleting.value = false
  }
}

onMounted(async () => {
  if (isManageRoute.value) {
    await loadDailies()
  }
})

watch(isManageRoute, async (active, previous) => {
  if (active && !previous) {
    await loadDailies()
  }
})
</script>

<style scoped>
.dailies-page {
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

.dailies-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 16px;
}

.daily-card,
.add-card {
  position: relative;
  min-height: 220px;
  border: 1px solid rgba(255, 255, 255, 0.12);
}

.daily-card {
  background: transparent;
  transition:
    transform 0.24s ease,
    border-color 0.24s ease;
}

.daily-card:hover {
  transform: translateY(-3px);
  border-color: rgba(255, 255, 255, 0.28);
}

.daily-content {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 12px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 8px;
}

.card-actions {
  display: flex;
  gap: 4px;
}

.icon-btn {
  border-radius: 10px;
}

.daily-title {
  font-size: 18px;
  font-weight: 700;
  color: #ffffff;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.daily-body {
  margin-top: 10px;
  color: #d2ddf6;
  font-size: 14px;
  line-height: 1.65;
  white-space: pre-wrap;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 6;
  -webkit-box-orient: vertical;
}

.card-footer {
  margin-top: auto;
  padding-top: 10px;
}

.daily-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #c6d3ef;
  font-size: 13px;
}

.daily-weather {
  padding: 2px 8px;
  border-radius: 999px;
  background: rgba(124, 151, 207, 0.28);
  border: 1px solid rgba(158, 180, 233, 0.38);
  color: #e7efff;
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
  background: transparent;
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
