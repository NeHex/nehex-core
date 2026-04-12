<template>
  <AdminLayout>
    <section v-if="isManageRoute" class="albums-page">
      <header class="page-header">
        <div class="header-text">
          <h1>相册管理</h1>
          <p>通过卡片管理相册，支持编辑、删除和新增。</p>
        </div>
        <v-btn
          class="quick-create-btn"
          color="primary"
          prepend-icon="mdi-plus"
          variant="flat"
          @click="openCreatePage"
        >
          新增相册
        </v-btn>
      </header>

      <v-progress-linear
        v-if="loading"
        class="mb-4"
        color="primary"
        indeterminate
      />

      <div class="albums-grid">
        <v-card
          v-for="album in albums"
          :key="album.id"
          class="album-card"
          :style="getAlbumCardStyle(album)"
          rounded="xl"
        >
          <div class="album-overlay">
            <div class="card-actions">
              <v-btn
                class="icon-btn"
                color="white"
                icon="mdi-pencil-outline"
                size="small"
                variant="text"
                @click.stop="openEditPage(album)"
              />
              <v-btn
                class="icon-btn"
                color="error"
                icon="mdi-delete-outline"
                size="small"
                variant="text"
                @click.stop="openDeleteDialog(album)"
              />
            </div>

            <div class="card-footer">
              <div class="album-title">{{ album.title }}</div>
            </div>
          </div>
        </v-card>

        <v-card
          class="add-card"
          rounded="xl"
          @click="openCreatePage"
        >
          <v-icon class="add-icon" icon="mdi-plus-circle-outline" size="40" />
          <div class="add-label">新增相册</div>
        </v-card>
      </div>
    </section>

    <v-dialog v-if="isManageRoute" v-model="deleteDialog" max-width="420">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title class="dialog-title">确认删除</v-card-title>
        <v-card-text>
          即将删除相册《{{ pendingDelete?.title || '' }}》，删除后不可恢复。
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
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import {
  deleteAlbum,
  fetchAlbums,
  type AlbumItem,
} from '@/services/albums'

const router = useRouter()
const route = useRoute()
const isManageRoute = computed(() => route.path === '/albums')
const { showGlobalSuccess, showGlobalError } = useGlobalSnackbar()

const loading = ref(false)
const deleting = ref(false)
const errorMessage = ref('')

const albums = ref<AlbumItem[]>([])
const deleteDialog = ref(false)
const pendingDelete = ref<AlbumItem | null>(null)

function openCreatePage(): void {
  void router.push('/albums/new')
}

function openEditPage(album: AlbumItem): void {
  void router.push(`/albums/edit/${album.id}`)
}

function openDeleteDialog(album: AlbumItem): void {
  pendingDelete.value = album
  deleteDialog.value = true
}

function closeDeleteDialog(force = false): void {
  if (deleting.value && !force) {
    return
  }
  deleteDialog.value = false
  pendingDelete.value = null
}

async function loadAlbums(): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  try {
    albums.value = await fetchAlbums()
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载相册失败'
    errorMessage.value = message
    showGlobalError(message)
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
    await deleteAlbum(pendingDelete.value.id)
    closeDeleteDialog(true)
    showGlobalSuccess('相册已删除')
    await loadAlbums()
  } catch (error) {
    const message = error instanceof Error ? error.message : '删除相册失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    deleting.value = false
  }
}

function getAlbumCardStyle(album: AlbumItem): Record<string, string> {
  const cover = (album.cover || '').trim()
  if (!cover) {
    return {
      background: 'linear-gradient(140deg, #1e2433 0%, #131827 100%)',
    }
  }

  const safeUrl = cover.replace(/"/g, '\\"')
  return {
    backgroundImage: `url("${safeUrl}")`,
    backgroundPosition: 'center',
    backgroundRepeat: 'no-repeat',
    backgroundSize: 'cover',
  }
}

onMounted(async () => {
  if (isManageRoute.value) {
    await loadAlbums()
  }
})

watch(isManageRoute, async (active, previous) => {
  if (active && !previous) {
    await loadAlbums()
  }
})
</script>

<style scoped>
.albums-page {
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

.albums-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 16px;
}

.album-card,
.add-card {
  position: relative;
  min-height: 240px;
  border: 1px solid rgba(255, 255, 255, 0.12);
}

.album-card {
  overflow: hidden;
  transition:
    transform 0.24s ease,
    box-shadow 0.24s ease;
}

.album-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 16px 30px rgba(0, 0, 0, 0.28);
}

.album-overlay {
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
  align-items: flex-end;
}

.album-title {
  font-size: 20px;
  font-weight: 700;
  color: #ffffff;
  text-shadow: 0 2px 12px rgba(0, 0, 0, 0.68);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
