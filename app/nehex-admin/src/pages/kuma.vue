<template>
  <AdminLayout>
    <template #secondary-nav>
      <div class="kuma-subnav">
        <div class="subnav-title">Kuma</div>
        <v-list class="subnav-list" density="comfortable" nav>
          <v-list-item
            v-for="item in sections"
            :key="item.key"
            class="subnav-item"
            :active="activeSection === item.key"
            :prepend-icon="item.icon"
            rounded="lg"
            :title="item.label"
            @click="activeSection = item.key"
          />
        </v-list>
      </div>
    </template>

    <section class="kuma-page">
      <header class="page-header">
        <div class="header-text">
          <h1>{{ activeSectionMeta.label }}</h1>
          <p>{{ activeSectionMeta.description }}</p>
        </div>
      </header>

      <v-window v-model="activeSection" :touch="false" class="section-window">
        <v-window-item value="movie">
          <div class="movie-cards-grid">
            <v-card class="movie-card movie-card--creator" rounded="xl">
              <div class="movie-cover movie-cover--placeholder">
                <v-icon icon="mdi-movie-open-plus-outline" size="46" />
                <span>新建电影卡片</span>
              </div>

              <v-card-text class="movie-card-body">
                <div class="movie-input-column">
                  <v-text-field
                    v-model="movieIdInput"
                    density="comfortable"
                    hide-details
                    label="电影 ID"
                    placeholder="例如：1292052 / 278"
                    variant="outlined"
                    @keydown.enter.prevent="createMovieCard"
                  />
                  <v-select
                    v-model="movieProvider"
                    :items="movieProviderOptions"
                    density="comfortable"
                    hide-details
                    item-title="label"
                    item-value="value"
                    label="来源"
                    variant="outlined"
                  />
                  <v-select
                    v-model="movieWatchStatus"
                    :items="movieWatchStatusOptions"
                    density="comfortable"
                    hide-details
                    item-title="label"
                    item-value="value"
                    label="状态"
                    variant="outlined"
                  >
                    <template #item="{ props, item }">
                      <v-list-item v-bind="props">
                        <template #prepend>
                          <v-icon :icon="item.raw.icon" size="18" />
                        </template>
                      </v-list-item>
                    </template>
                    <template #selection="{ item }">
                      <div class="movie-status-selection">
                        <v-icon :icon="item.raw.icon" size="16" />
                        <span>{{ item.raw.label }}</span>
                      </div>
                    </template>
                  </v-select>
                </div>

                <v-btn
                  block
                  color="primary"
                  prepend-icon="mdi-plus"
                  :loading="movieCreating"
                  @click="createMovieCard"
                >
                  新建
                </v-btn>

                <v-alert
                  v-if="movieCreateError"
                  class="mt-2"
                  density="comfortable"
                  type="error"
                  variant="tonal"
                >
                  {{ movieCreateError }}
                </v-alert>
              </v-card-text>
            </v-card>

            <v-card
              v-for="card in movieCards"
              :key="card.id"
              class="movie-card"
              rounded="xl"
            >
              <v-img
                v-if="card.cover"
                :src="card.cover"
                class="movie-cover"
                cover
                height="320"
              />
              <div v-else class="movie-cover movie-cover--empty">
                <v-icon icon="mdi-image-off-outline" size="42" />
                <span>无封面</span>
              </div>

              <v-card-text class="movie-card-body">
                <div class="movie-meta-row">
                  <v-chip size="small" variant="tonal">
                    {{ card.provider.toUpperCase() }}
                  </v-chip>
                  <v-chip size="small" variant="tonal">#{{ card.movie_id }}</v-chip>
                  <v-chip size="small" variant="tonal">
                    <v-icon
                      start
                      :icon="getMovieWatchStatusMeta(card.watch_status).icon"
                      size="14"
                    />
                    {{ getMovieWatchStatusMeta(card.watch_status).label }}
                  </v-chip>
                  <v-chip
                    v-if="card.score"
                    color="warning"
                    size="small"
                    variant="tonal"
                  >
                    评分 {{ card.score }}
                  </v-chip>
                </div>

                <div class="movie-title">
                  {{ card.title }}
                  <span v-if="card.years" class="movie-years">{{ card.years }}</span>
                </div>

                <p class="movie-desc">{{ card.desc || '暂无简介' }}</p>

                <div class="movie-action-row">
                  <v-btn
                    v-if="card.url"
                    class="movie-link-btn"
                    color="primary"
                    :href="card.url"
                    prepend-icon="mdi-open-in-new"
                    rel="noopener noreferrer"
                    target="_blank"
                    variant="text"
                  >
                    打开详情
                  </v-btn>
                  <span v-else class="movie-action-placeholder" />

                  <v-btn
                    class="movie-edit-btn"
                    color="info"
                    prepend-icon="mdi-pencil-outline"
                    :loading="isMovieEditing(card.id)"
                    variant="text"
                    @click="openEditMovieDialog(card)"
                  >
                    编辑
                  </v-btn>
                  <v-btn
                    class="movie-delete-btn"
                    color="error"
                    prepend-icon="mdi-delete-outline"
                    :disabled="isMovieEditing(card.id)"
                    :loading="isMovieDeleting(card.id)"
                    variant="text"
                    @click="openDeleteMovieDialog(card)"
                  >
                    删除
                  </v-btn>
                </div>
              </v-card-text>
            </v-card>
          </div>
        </v-window-item>

        <v-window-item value="music">
          <v-card class="section-card" rounded="xl">
            <v-card-title>音乐配置（预留）</v-card-title>
            <v-card-text>
              音乐模块配置页面预留中，后续将在这里接入 Kuma 音乐相关配置。
            </v-card-text>
          </v-card>
        </v-window-item>
      </v-window>
    </section>

    <v-dialog
      v-model="movieEditDialog"
      :persistent="isEditDialogSubmitting"
      max-width="500"
    >
      <v-card class="movie-edit-dialog-card" rounded="xl">
        <v-card-title class="movie-edit-dialog-title">
          <v-icon color="info" icon="mdi-pencil-outline" />
          <span>编辑电影卡片</span>
        </v-card-title>
        <v-card-text class="movie-edit-dialog-text">
          <div class="movie-input-column">
            <v-text-field
              v-model="movieEditIdInput"
              density="comfortable"
              hide-details
              label="电影 ID"
              placeholder="例如：1292052 / 278"
              variant="outlined"
              @keydown.enter.prevent="confirmEditMovieCard"
            />
            <v-select
              v-model="movieEditProvider"
              :items="movieProviderOptions"
              density="comfortable"
              hide-details
              item-title="label"
              item-value="value"
              label="来源"
              variant="outlined"
            />
            <v-select
              v-model="movieEditWatchStatus"
              :items="movieWatchStatusOptions"
              density="comfortable"
              hide-details
              item-title="label"
              item-value="value"
              label="状态"
              variant="outlined"
            >
              <template #item="{ props, item }">
                <v-list-item v-bind="props">
                  <template #prepend>
                    <v-icon :icon="item.raw.icon" size="18" />
                  </template>
                </v-list-item>
              </template>
              <template #selection="{ item }">
                <div class="movie-status-selection">
                  <v-icon :icon="item.raw.icon" size="16" />
                  <span>{{ item.raw.label }}</span>
                </div>
              </template>
            </v-select>
            <v-text-field
              v-model="movieEditTitle"
              density="comfortable"
              hide-details
              label="标题"
              placeholder="电影标题"
              variant="outlined"
              @keydown.enter.prevent="confirmEditMovieCard"
            />
            <v-text-field
              v-model="movieEditYears"
              density="comfortable"
              hide-details
              label="年份"
              placeholder="例如：2024"
              variant="outlined"
            />
            <v-text-field
              v-model="movieEditScore"
              density="comfortable"
              hide-details
              label="评分"
              placeholder="例如：8.6"
              variant="outlined"
            />
            <v-text-field
              v-model="movieEditCover"
              density="comfortable"
              hide-details
              label="封面地址"
              placeholder="https://..."
              variant="outlined"
            />
            <v-text-field
              v-model="movieEditUrl"
              density="comfortable"
              hide-details
              label="详情链接"
              placeholder="https://..."
              variant="outlined"
            />
            <v-textarea
              v-model="movieEditDesc"
              auto-grow
              density="comfortable"
              hide-details
              label="简介"
              placeholder="电影简介"
              rows="3"
              variant="outlined"
            />
          </div>
          <v-alert
            v-if="movieEditError"
            class="mt-3"
            density="comfortable"
            type="error"
            variant="tonal"
          >
            {{ movieEditError }}
          </v-alert>
        </v-card-text>
        <v-card-actions class="movie-edit-dialog-actions">
          <v-spacer />
          <v-btn
            :disabled="isEditDialogSubmitting"
            variant="text"
            @click="closeEditMovieDialog"
          >
            取消
          </v-btn>
          <v-btn
            color="info"
            prepend-icon="mdi-content-save-outline"
            :loading="isEditDialogSubmitting"
            variant="flat"
            @click="confirmEditMovieCard"
          >
            保存
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog
      v-model="movieDeleteDialog"
      :persistent="isDeleteDialogSubmitting"
      max-width="500"
    >
      <v-card class="movie-delete-dialog-card" rounded="xl">
        <v-card-title class="movie-delete-dialog-title">
          <v-icon color="error" icon="mdi-alert-circle-outline" />
          <span>删除电影卡片</span>
        </v-card-title>
        <v-card-text class="movie-delete-dialog-text">
          <p>
            确定要删除
            <strong>「{{ movieCardPendingDelete?.title || '-' }}」</strong>
            吗？此操作不可撤销。
          </p>
        </v-card-text>
        <v-card-actions class="movie-delete-dialog-actions">
          <v-spacer />
          <v-btn
            :disabled="isDeleteDialogSubmitting"
            variant="text"
            @click="closeDeleteMovieDialog"
          >
            取消
          </v-btn>
          <v-btn
            color="error"
            prepend-icon="mdi-delete-outline"
            :loading="isDeleteDialogSubmitting"
            variant="flat"
            @click="confirmDeleteMovieCard"
          >
            确认删除
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </AdminLayout>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref, watch } from 'vue'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import {
  createAdminKumaMovie,
  deleteAdminKumaMovie,
  fetchAdminKumaMovies,
  updateAdminKumaMovie,
  type KumaMovieCard,
  type KumaMovieProvider,
  type KumaMovieWatchStatus,
} from '@/services/kuma'

type KumaSectionKey = 'movie' | 'music'

type SectionMeta = {
  key: KumaSectionKey
  label: string
  icon: string
  description: string
}

const sections: SectionMeta[] = [
  {
    key: 'movie',
    label: '电影',
    icon: 'mdi-movie-open-outline',
    description: '管理 Kuma 电影相关配置（待实现）。',
  },
  {
    key: 'music',
    label: '音乐',
    icon: 'mdi-music-note-outline',
    description: '管理 Kuma 音乐相关配置（待实现）。',
  },
]

const activeSection = ref<KumaSectionKey>('movie')
const activeSectionMeta = computed(() => {
  return sections.find((item) => item.key === activeSection.value) || sections[0]!
})

const movieProviderOptions: Array<{ label: string, value: KumaMovieProvider }> = [
  { label: '豆瓣', value: 'douban' },
  { label: 'TMDB', value: 'tmdb' },
]
const movieWatchStatusOptions: Array<{
  label: string
  value: KumaMovieWatchStatus
  icon: string
}> = [
  { label: '想看', value: 'want', icon: 'mdi-star-outline' },
  { label: '看过', value: 'watched', icon: 'mdi-star' },
  { label: '喜欢', value: 'liked', icon: 'mdi-heart' },
]

const movieIdInput = ref('')
const movieProvider = ref<KumaMovieProvider>('douban')
const movieWatchStatus = ref<KumaMovieWatchStatus>('want')
const movieCreating = ref(false)
const movieCreateError = ref('')
const movieCards = ref<KumaMovieCard[]>([])
const deletingMovieIds = ref<number[]>([])
const editingMovieIds = ref<number[]>([])
const movieEditDialog = ref(false)
const movieCardPendingEdit = ref<KumaMovieCard | null>(null)
const movieEditIdInput = ref('')
const movieEditProvider = ref<KumaMovieProvider>('douban')
const movieEditWatchStatus = ref<KumaMovieWatchStatus>('want')
const movieEditCover = ref('')
const movieEditTitle = ref('')
const movieEditYears = ref('')
const movieEditScore = ref('')
const movieEditDesc = ref('')
const movieEditUrl = ref('')
const movieEditError = ref('')
const movieDeleteDialog = ref(false)
const movieCardPendingDelete = ref<KumaMovieCard | null>(null)
const isEditDialogSubmitting = computed(() => {
  const id = movieCardPendingEdit.value?.id
  if (typeof id !== 'number') {
    return false
  }
  return isMovieEditing(id)
})
const isDeleteDialogSubmitting = computed(() => {
  const id = movieCardPendingDelete.value?.id
  if (typeof id !== 'number') {
    return false
  }
  return isMovieDeleting(id)
})

function normalizeMovieProvider(raw: string): KumaMovieProvider {
  return raw === 'tmdb' ? 'tmdb' : 'douban'
}

function normalizeMovieWatchStatus(raw: string | undefined): KumaMovieWatchStatus {
  if (raw === 'watched' || raw === 'liked') {
    return raw
  }
  return 'want'
}

function getMovieWatchStatusMeta(raw: string): {
  label: string
  value: KumaMovieWatchStatus
  icon: string
} {
  const normalized = normalizeMovieWatchStatus(raw)
  return movieWatchStatusOptions.find((item) => item.value === normalized) || movieWatchStatusOptions[0]!
}

async function createMovieCard(): Promise<void> {
  const movieId = movieIdInput.value.trim()
  if (!movieId) {
    movieCreateError.value = '请先输入电影 ID'
    return
  }

  movieCreating.value = true
  movieCreateError.value = ''
  try {
    const item = await createAdminKumaMovie({
      provider: movieProvider.value,
      movie_id: movieId,
      watch_status: movieWatchStatus.value,
    })

    movieCards.value = [
      item,
      ...movieCards.value.filter((card) => card.id !== item.id),
    ]
    movieCards.value.sort((a, b) => b.id - a.id)
    movieIdInput.value = ''
  } catch (error) {
    movieCreateError.value = error instanceof Error ? error.message : '创建电影卡片失败'
  } finally {
    movieCreating.value = false
  }
}

function isMovieDeleting(id: number): boolean {
  return deletingMovieIds.value.includes(id)
}

function isMovieEditing(id: number): boolean {
  return editingMovieIds.value.includes(id)
}

function openEditMovieDialog(card: KumaMovieCard): void {
  if (isMovieDeleting(card.id) || isMovieEditing(card.id)) {
    return
  }

  movieCardPendingEdit.value = card
  movieEditProvider.value = normalizeMovieProvider(card.provider)
  movieEditWatchStatus.value = normalizeMovieWatchStatus(card.watch_status)
  movieEditIdInput.value = card.movie_id
  movieEditCover.value = card.cover || ''
  movieEditTitle.value = card.title || ''
  movieEditYears.value = card.years || ''
  movieEditScore.value = card.score || ''
  movieEditDesc.value = card.desc || ''
  movieEditUrl.value = card.url || ''
  movieEditError.value = ''
  movieEditDialog.value = true
}

function closeEditMovieDialog(): void {
  if (isEditDialogSubmitting.value) {
    return
  }
  movieEditDialog.value = false
  movieCardPendingEdit.value = null
  movieEditError.value = ''
}

async function confirmEditMovieCard(): Promise<void> {
  const card = movieCardPendingEdit.value
  if (!card || isMovieEditing(card.id)) {
    return
  }

  const movieId = movieEditIdInput.value.trim()
  const title = movieEditTitle.value.trim()
  if (!movieId) {
    movieEditError.value = '请先输入电影 ID'
    return
  }
  if (!title) {
    movieEditError.value = '请先输入标题'
    return
  }

  editingMovieIds.value = [...editingMovieIds.value, card.id]
  movieEditError.value = ''
  movieCreateError.value = ''
  try {
    const updated = await updateAdminKumaMovie(card.id, {
      provider: movieEditProvider.value,
      movie_id: movieId,
      watch_status: movieEditWatchStatus.value,
      cover: movieEditCover.value.trim(),
      title,
      years: movieEditYears.value.trim(),
      score: movieEditScore.value.trim(),
      desc: movieEditDesc.value.trim(),
      url: movieEditUrl.value.trim(),
    })
    movieCards.value = movieCards.value.map((item) => {
      if (item.id === updated.id) {
        return updated
      }
      return item
    })
    movieEditDialog.value = false
    movieCardPendingEdit.value = null
  } catch (error) {
    movieEditError.value = error instanceof Error ? error.message : '更新电影卡片失败'
  } finally {
    editingMovieIds.value = editingMovieIds.value.filter((id) => id !== card.id)
  }
}

function openDeleteMovieDialog(card: KumaMovieCard): void {
  if (isMovieDeleting(card.id) || isMovieEditing(card.id)) {
    return
  }
  movieCardPendingDelete.value = card
  movieDeleteDialog.value = true
}

function closeDeleteMovieDialog(): void {
  if (isDeleteDialogSubmitting.value) {
    return
  }
  movieDeleteDialog.value = false
  movieCardPendingDelete.value = null
}

async function confirmDeleteMovieCard(): Promise<void> {
  const card = movieCardPendingDelete.value
  if (!card || isMovieDeleting(card.id)) {
    return
  }

  deletingMovieIds.value = [...deletingMovieIds.value, card.id]
  movieCreateError.value = ''
  try {
    await deleteAdminKumaMovie(card.id)
    movieCards.value = movieCards.value.filter((item) => item.id !== card.id)
    movieDeleteDialog.value = false
    movieCardPendingDelete.value = null
  } catch (error) {
    movieCreateError.value = error instanceof Error ? error.message : '删除电影卡片失败'
  } finally {
    deletingMovieIds.value = deletingMovieIds.value.filter((id) => id !== card.id)
  }
}

watch(movieDeleteDialog, (visible) => {
  if (!visible && !isDeleteDialogSubmitting.value) {
    movieCardPendingDelete.value = null
  }
})

watch(movieEditDialog, (visible) => {
  if (!visible && !isEditDialogSubmitting.value) {
    movieCardPendingEdit.value = null
    movieEditError.value = ''
  }
})

onMounted(async () => {
  try {
    movieCards.value = await fetchAdminKumaMovies()
  } catch (error) {
    movieCreateError.value = error instanceof Error ? error.message : '加载电影卡片失败'
  }
})
</script>

<style scoped>
.kuma-subnav {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.subnav-title {
  padding: 8px 8px 0;
  font-size: 16px;
  font-weight: 700;
  color: #f2f5ff;
  letter-spacing: 0.3px;
}

.subnav-list {
  padding: 4px 0 0;
  background: transparent;
}

:deep(.subnav-item) {
  min-height: 42px;
  margin-bottom: 6px;
  color: #b6c3de;
  border: 1px solid transparent;
  transition:
    background 0.2s ease,
    color 0.2s ease;
}

:deep(.subnav-item:hover) {
  color: #eef3ff;
  background: linear-gradient(90deg, rgba(103, 121, 170, 0.14) 0%, rgba(112, 133, 186, 0.24) 100%);
}

:deep(.subnav-item.v-list-item--active) {
  color: #ffffff;
  background: linear-gradient(90deg, rgba(103, 121, 170, 0.28) 0%, rgba(112, 133, 186, 0.42) 100%);
}

.kuma-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: 12px;
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

.section-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, rgba(24, 30, 41, 0.96), rgba(19, 24, 34, 0.96));
  color: #edf1ff;
}

.movie-cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 12px;
}

.movie-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, rgba(24, 30, 41, 0.96), rgba(19, 24, 34, 0.96));
  color: #edf1ff;
  overflow: hidden;
}

.movie-cover {
  width: 100%;
  height: 320px;
}

.movie-cover--placeholder,
.movie-cover--empty {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 8px;
  color: #b7c4e4;
  background: linear-gradient(180deg, rgba(39, 47, 64, 0.8), rgba(31, 38, 53, 0.8));
}

.movie-card-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.movie-input-column {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.movie-status-selection {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.movie-meta-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.movie-title {
  font-size: 18px;
  font-weight: 700;
  color: #f2f5ff;
  line-height: 1.3;
}

.movie-years {
  margin-left: 6px;
  font-size: 14px;
  color: #aeb8cc;
  font-weight: 500;
}

.movie-desc {
  margin: 0;
  color: #d2dcf3;
  line-height: 1.6;
  font-size: 14px;
  display: -webkit-box;
  -webkit-line-clamp: 4;
  -webkit-box-orient: vertical;
  overflow: hidden;
  min-height: calc(1.6em * 4);
}

.movie-action-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.movie-action-placeholder {
  flex: 1 1 auto;
}

.movie-link-btn,
.movie-edit-btn,
.movie-delete-btn {
  margin-left: -8px;
}

.movie-edit-dialog-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, rgba(24, 30, 41, 0.98), rgba(19, 24, 34, 0.98));
  color: #edf1ff;
}

.movie-edit-dialog-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
}

.movie-edit-dialog-text {
  color: #d2dcf3;
  line-height: 1.7;
  max-height: 62vh;
  overflow: auto;
}

.movie-edit-dialog-actions {
  padding: 0 20px 18px;
}

.movie-delete-dialog-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, rgba(24, 30, 41, 0.98), rgba(19, 24, 34, 0.98));
  color: #edf1ff;
}

.movie-delete-dialog-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
}

.movie-delete-dialog-text {
  color: #d2dcf3;
  line-height: 1.7;
}

.movie-delete-dialog-text p {
  margin: 0;
}

.movie-delete-dialog-actions {
  padding: 0 20px 18px;
}

@media (max-width: 980px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
