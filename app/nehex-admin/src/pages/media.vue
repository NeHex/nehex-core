<template>
  <AdminLayout>
    <section class="media-page">
      <header class="page-header">
        <div class="header-text">
          <h1>媒体库</h1>
          <p>统一管理已上传资源，支持图片/视频/音频/文件分类、拖拽归类、批量操作与链接复制。</p>
        </div>
      </header>

      <v-progress-linear
        v-if="loading"
        color="primary"
        indeterminate
      />

      <v-card class="folders-card" rounded="xl">
        <v-card-title class="folders-head">分类文件夹（可拖拽资源到文件夹）</v-card-title>
        <v-card-text>
          <div class="folders-row">
            <article
              class="folder-item"
              :class="{ 'folder-item--active': currentFolderId === null }"
              role="button"
              tabindex="0"
              @click="selectFolder(null)"
              @keydown.enter.prevent="selectFolder(null)"
              @dragover.prevent="handleFolderDragOver(null, $event)"
              @drop.prevent="handleFolderDrop(null)"
            >
              <div class="folder-name">未分类</div>
              <div class="folder-count">{{ uncategorizedImages.length }} 个资源</div>
            </article>

            <article
              v-for="folder in folders"
              :key="folder.id"
              class="folder-item"
              :class="{ 'folder-item--active': currentFolderId === folder.id }"
              role="button"
              tabindex="0"
              @click="selectFolder(folder.id)"
              @keydown.enter.prevent="selectFolder(folder.id)"
              @dragover.prevent="handleFolderDragOver(folder.id, $event)"
              @drop.prevent="handleFolderDrop(folder.id)"
            >
              <div class="folder-top">
                <div class="folder-name">{{ folder.name }}</div>
                <div class="folder-actions">
                  <v-btn
                    icon="mdi-pencil-outline"
                    size="x-small"
                    variant="text"
                    @click.stop="openRenameDialog(folder)"
                  />
                  <v-btn
                    color="error"
                    icon="mdi-delete-outline"
                    size="x-small"
                    variant="text"
                    @click.stop="openDeleteFolderDialog(folder)"
                  />
                </div>
              </div>
              <div class="folder-count">{{ folder.image_count }} 个资源</div>
            </article>

            <article class="folder-create-card">
              <div class="folder-create-title">新建文件夹</div>
              <v-text-field
                v-model="newFolderName"
                class="folder-create-input"
                density="comfortable"
                hide-details
                label="文件夹名称"
                placeholder="例如：Banner、封面图"
                variant="outlined"
                @keydown.enter.prevent="submitCreateFolder"
              />
              <v-btn
                color="primary"
                prepend-icon="mdi-folder-plus-outline"
                :loading="creatingFolder"
                @click="submitCreateFolder"
              >
                创建
              </v-btn>
            </article>
          </div>
        </v-card-text>
      </v-card>

      <v-card class="images-card" rounded="xl">
        <v-card-title class="images-head">
          <div class="images-title">{{ currentFolderLabel }}</div>
          <div class="images-actions">
            <v-select
              v-model="batchTargetFolderId"
              class="batch-select"
              density="compact"
              hide-details
              item-title="title"
              item-value="value"
              :items="batchFolderOptions"
              label="批量归类到"
              variant="outlined"
            />
            <v-btn
              color="primary"
              prepend-icon="mdi-folder-move-outline"
              size="small"
              :disabled="selectedImageIds.length <= 0"
              :loading="movingImages"
              @click="submitBatchMove"
            >
              批量归类（{{ selectedImageIds.length }}）
            </v-btn>
            <v-btn
              size="small"
              variant="text"
              :disabled="selectedImageIds.length <= 0"
              @click="clearSelection"
            >
              取消选择
            </v-btn>
          </div>
        </v-card-title>

        <v-card-text>
          <v-progress-linear
            v-if="loadingFolderImages"
            class="mb-3"
            color="primary"
            indeterminate
          />

          <div class="media-type-filter">
            <v-btn
              v-for="item in mediaTypeFilters"
              :key="item.value"
              class="type-filter-btn"
              :color="mediaTypeFilter === item.value ? 'primary' : undefined"
              size="small"
              :variant="mediaTypeFilter === item.value ? 'flat' : 'tonal'"
              @click="setMediaTypeFilter(item.value)"
            >
              {{ item.label }}
            </v-btn>
          </div>

          <div v-if="displayedImages.length > 0" class="images-grid">
            <article
              v-for="image in displayedImages"
              :key="image.id"
              class="image-item"
              :class="{ 'image-item--selected': selectedImageIds.includes(image.id) }"
              draggable="true"
              @click="openPreview(image)"
              @dragstart="handleImageDragStart(image.id, $event)"
              @dragend="handleImageDragEnd"
            >
              <img
                v-if="isImageAsset(image)"
                :src="image.url"
                alt="media image"
              >
              <video
                v-else-if="isVideoAsset(image)"
                class="video-thumb"
                :src="image.url"
                muted
                preload="metadata"
              />
              <div v-else class="file-thumb">
                <v-icon :icon="getAssetIcon(image)" size="34" />
                <div class="file-thumb-name">{{ getAssetFileName(image) }}</div>
              </div>

              <div class="asset-type-badge">{{ getAssetTypeLabel(image) }}</div>

              <div class="image-actions">
                <v-btn
                  :color="selectedImageIds.includes(image.id) ? 'primary' : undefined"
                  :icon="selectedImageIds.includes(image.id) ? 'mdi-checkbox-marked-circle' : 'mdi-checkbox-blank-circle-outline'"
                  size="x-small"
                  variant="text"
                  @click.stop="toggleImageSelection(image.id)"
                />
              </div>
            </article>
          </div>
          <div v-else class="images-empty">当前分类暂无符合筛选条件的资源。</div>
        </v-card-text>
      </v-card>

      <ImageUploadHintCard
        class="upload-card"
        :loading="uploadingImages"
        :multiple="true"
        accept="*/*"
        file-filter-mode="any"
        title="上传资源到媒体库"
        hint="拖动文件到卡片，或点击选择（支持图片、视频、音频与常见文档）"
        @select-files="handleUploadCardFiles"
      />
    </section>

    <v-dialog v-model="renameDialog" max-width="460">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title>重命名分类文件夹</v-card-title>
        <v-card-text>
          <v-text-field
            v-model="renameFolderName"
            autofocus
            density="comfortable"
            hide-details
            label="新名称"
            variant="outlined"
            @keydown.enter.prevent="submitRenameFolder"
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="closeRenameDialog">取消</v-btn>
          <v-btn color="primary" :loading="renamingFolder" @click="submitRenameFolder">
            确认
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="deleteFolderDialog" max-width="460">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title>删除分类文件夹</v-card-title>
        <v-card-text>
          将删除“{{ deleteFolderTarget?.name || '' }}”文件夹。文件夹中的
          {{ deleteFolderTarget?.image_count || 0 }} 个资源会自动归到“未分类”。确定继续吗？
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="closeDeleteFolderDialog">取消</v-btn>
          <v-btn color="error" :loading="deletingFolder" @click="submitDeleteFolder">
            确认删除
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="previewDialog" max-width="980">
      <v-card class="preview-dialog-card" rounded="xl">
        <v-card-text class="preview-body">
          <img
            v-if="previewImage && isImageAsset(previewImage)"
            class="preview-image"
            :src="previewImage.url"
            alt="preview image"
          >
          <video
            v-else-if="previewImage && isVideoAsset(previewImage)"
            class="preview-video"
            :src="previewImage.url"
            controls
            preload="metadata"
          />
          <audio
            v-else-if="previewImage && isAudioAsset(previewImage)"
            class="preview-audio"
            :src="previewImage.url"
            controls
            preload="metadata"
          />
          <div v-else-if="previewImage" class="preview-file-box">
            <v-icon :icon="getAssetIcon(previewImage)" size="42" />
            <div class="preview-file-name">{{ getAssetFileName(previewImage) }}</div>
            <v-btn
              color="primary"
              prepend-icon="mdi-open-in-new"
              :href="previewImage.url"
              rel="noopener"
              target="_blank"
              variant="tonal"
              @click.stop
            >
              打开文件
            </v-btn>
          </div>
        </v-card-text>
        <v-card-actions class="preview-actions">
          <v-spacer />
          <v-btn variant="tonal" @click="copyPreviewAs('url')">复制 URL</v-btn>
          <v-btn variant="tonal" @click="copyPreviewAs('markdown')">复制 Markdown</v-btn>
          <v-btn variant="tonal" @click="copyPreviewAs('html')">复制 HTML</v-btn>
          <v-btn variant="tonal" @click="copyPreviewAs('bbcode')">复制 BBCode</v-btn>
          <v-btn color="error" :loading="deletingImage" @click="deletePreviewImage">删除</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </AdminLayout>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref, watch } from 'vue'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import ImageUploadHintCard from '@/components/admin/ImageUploadHintCard.vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import {
  createMediaFolder,
  deleteMediaFolder,
  deleteMediaImage,
  fetchMediaImagesByFolder,
  fetchMediaLibrary,
  moveMediaImages,
  renameMediaFolder,
  uploadMediaImage,
  type MediaFolderItem,
  type MediaImageItem,
} from '@/services/media-library'

type CopyMode = 'url' | 'markdown' | 'html' | 'bbcode'
type MediaTypeFilter = 'all' | 'image' | 'video' | 'audio' | 'file'

const {
  showGlobalSuccess,
  showGlobalError,
  showGlobalProgress,
  updateGlobalProgress,
  hideGlobalSnackbar,
} = useGlobalSnackbar()

const loading = ref(false)
const loadingFolderImages = ref(false)
const creatingFolder = ref(false)
const renamingFolder = ref(false)
const deletingFolder = ref(false)
const movingImages = ref(false)
const uploadingImages = ref(false)
const deletingImage = ref(false)
const errorMessage = ref('')

const folders = ref<MediaFolderItem[]>([])
const uncategorizedImages = ref<MediaImageItem[]>([])
const currentFolderImages = ref<MediaImageItem[]>([])
const currentFolderId = ref<number | null>(null)
const selectedImageIds = ref<number[]>([])

const newFolderName = ref('')
const batchTargetFolderId = ref(-1)
const draggingImageId = ref<number | null>(null)

const renameDialog = ref(false)
const renameFolderTarget = ref<MediaFolderItem | null>(null)
const renameFolderName = ref('')
const deleteFolderDialog = ref(false)
const deleteFolderTarget = ref<MediaFolderItem | null>(null)

const previewDialog = ref(false)
const previewImage = ref<MediaImageItem | null>(null)
const mediaTypeFilter = ref<MediaTypeFilter>('all')
const mediaTypeFilters: Array<{ value: MediaTypeFilter, label: string }> = [
  { value: 'all', label: '全部' },
  { value: 'image', label: '图片' },
  { value: 'video', label: '视频' },
  { value: 'audio', label: '音频' },
  { value: 'file', label: '文件' },
]

const currentFolderLabel = computed(() => {
  if (currentFolderId.value === null) {
    return '未分类资源'
  }
  const matched = folders.value.find((item) => item.id === currentFolderId.value)
  return matched ? `分类：${matched.name}` : '分类资源'
})

const currentImages = computed(() => {
  if (currentFolderId.value === null) {
    return uncategorizedImages.value
  }
  return currentFolderImages.value
})

const displayedImages = computed(() => {
  if (mediaTypeFilter.value === 'all') {
    return currentImages.value
  }
  return currentImages.value.filter((item) => item.media_type === mediaTypeFilter.value)
})

const batchFolderOptions = computed(() => {
  return [
    { title: '未分类', value: -1 },
    ...folders.value.map((item) => ({
      title: item.name,
      value: item.id,
    })),
  ]
})

function normalizeFolderName(value: string): string {
  return value.trim()
}

function setMediaTypeFilter(value: MediaTypeFilter): void {
  mediaTypeFilter.value = value
}

function clearSelection(): void {
  selectedImageIds.value = []
}

function toggleImageSelection(imageId: number): void {
  if (selectedImageIds.value.includes(imageId)) {
    selectedImageIds.value = selectedImageIds.value.filter((item) => item !== imageId)
    return
  }
  selectedImageIds.value = [...selectedImageIds.value, imageId]
}

function isImageAsset(item: MediaImageItem): boolean {
  return item.media_type === 'image'
}

function isVideoAsset(item: MediaImageItem): boolean {
  return item.media_type === 'video'
}

function isAudioAsset(item: MediaImageItem): boolean {
  return item.media_type === 'audio'
}

function getAssetTypeLabel(item: MediaImageItem): string {
  if (item.media_type === 'image') {
    return '图片'
  }
  if (item.media_type === 'video') {
    return '视频'
  }
  if (item.media_type === 'audio') {
    return '音频'
  }
  return '文件'
}

function getAssetIcon(item: MediaImageItem): string {
  if (item.media_type === 'video') {
    return 'mdi-filmstrip-box-multiple'
  }
  if (item.media_type === 'audio') {
    return 'mdi-music-note'
  }

  const name = getAssetFileName(item).toLowerCase()
  if (name.endsWith('.pdf')) {
    return 'mdi-file-pdf-box'
  }
  if (name.endsWith('.zip') || name.endsWith('.rar') || name.endsWith('.7z') || name.endsWith('.tar')) {
    return 'mdi-folder-zip-outline'
  }
  if (name.endsWith('.doc') || name.endsWith('.docx')) {
    return 'mdi-file-word-box'
  }
  if (name.endsWith('.xls') || name.endsWith('.xlsx') || name.endsWith('.csv')) {
    return 'mdi-file-excel-box'
  }
  if (name.endsWith('.ppt') || name.endsWith('.pptx')) {
    return 'mdi-file-powerpoint-box'
  }
  return 'mdi-file-outline'
}

function getAssetFileName(item: MediaImageItem): string {
  const rawName = (item.file_name || '').trim()
  if (rawName) {
    return rawName
  }

  try {
    const pathname = new URL(item.url, window.location.origin).pathname
    const parts = pathname.split('/').filter(Boolean)
    const last = parts.length > 0 ? parts[parts.length - 1] : ''
    if (last) {
      return decodeURIComponent(last)
    }
  } catch {
    // Keep fallback below.
  }
  return `asset-${item.id}`
}

async function loadLibrary(): Promise<void> {
  const payload = await fetchMediaLibrary()
  folders.value = payload.folders
  uncategorizedImages.value = payload.uncategorized

  if (
    currentFolderId.value !== null
    && !folders.value.some((item) => item.id === currentFolderId.value)
  ) {
    currentFolderId.value = null
    currentFolderImages.value = []
  }
}

async function loadCurrentFolderImages(): Promise<void> {
  if (currentFolderId.value === null) {
    currentFolderImages.value = []
    return
  }

  loadingFolderImages.value = true
  try {
    currentFolderImages.value = await fetchMediaImagesByFolder(currentFolderId.value)
  } finally {
    loadingFolderImages.value = false
  }
}

async function reloadAll(): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  try {
    await loadLibrary()
    await loadCurrentFolderImages()
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载媒体库失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    loading.value = false
  }
}

async function selectFolder(folderId: number | null): Promise<void> {
  if (currentFolderId.value === folderId) {
    return
  }
  currentFolderId.value = folderId
  batchTargetFolderId.value = folderId ?? -1
  clearSelection()
  await loadCurrentFolderImages()
}

async function submitCreateFolder(): Promise<void> {
  const folderName = normalizeFolderName(newFolderName.value)
  if (!folderName || creatingFolder.value) {
    return
  }

  creatingFolder.value = true
  errorMessage.value = ''
  try {
    await createMediaFolder(folderName)
    newFolderName.value = ''
    showGlobalSuccess('分类文件夹创建成功')
    await reloadAll()
  } catch (error) {
    const message = error instanceof Error ? error.message : '新建分类文件夹失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    creatingFolder.value = false
  }
}

function openRenameDialog(folder: MediaFolderItem): void {
  renameFolderTarget.value = folder
  renameFolderName.value = folder.name
  renameDialog.value = true
}

function closeRenameDialog(): void {
  renameDialog.value = false
  renameFolderTarget.value = null
  renameFolderName.value = ''
}

function openDeleteFolderDialog(folder: MediaFolderItem): void {
  deleteFolderTarget.value = folder
  deleteFolderDialog.value = true
}

function closeDeleteFolderDialog(): void {
  deleteFolderDialog.value = false
  deleteFolderTarget.value = null
}

async function submitRenameFolder(): Promise<void> {
  if (!renameFolderTarget.value || renamingFolder.value) {
    return
  }
  const folderName = normalizeFolderName(renameFolderName.value)
  if (!folderName) {
    return
  }

  renamingFolder.value = true
  errorMessage.value = ''
  try {
    await renameMediaFolder(renameFolderTarget.value.id, folderName)
    closeRenameDialog()
    showGlobalSuccess('分类文件夹重命名成功')
    await reloadAll()
  } catch (error) {
    const message = error instanceof Error ? error.message : '重命名分类文件夹失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    renamingFolder.value = false
  }
}

async function submitDeleteFolder(): Promise<void> {
  if (!deleteFolderTarget.value || deletingFolder.value) {
    return
  }

  deletingFolder.value = true
  errorMessage.value = ''
  try {
    const movedCount = deleteFolderTarget.value.image_count
    await deleteMediaFolder(deleteFolderTarget.value.id)
    closeDeleteFolderDialog()
    showGlobalSuccess(`文件夹已删除，${movedCount} 个资源已归入未分类`)
    await reloadAll()
  } catch (error) {
    const message = error instanceof Error ? error.message : '删除分类文件夹失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    deletingFolder.value = false
  }
}

async function moveImageIdsToFolder(imageIds: number[], folderId: number | null): Promise<void> {
  if (imageIds.length <= 0) {
    return
  }

  movingImages.value = true
  errorMessage.value = ''
  try {
    await moveMediaImages(imageIds, folderId)
    clearSelection()
    showGlobalSuccess(`成功归类 ${imageIds.length} 个资源`)
    await reloadAll()
  } catch (error) {
    const message = error instanceof Error ? error.message : '批量归类失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    movingImages.value = false
  }
}

async function submitBatchMove(): Promise<void> {
  const folderId = batchTargetFolderId.value > 0 ? batchTargetFolderId.value : null
  await moveImageIdsToFolder(selectedImageIds.value, folderId)
}

function handleImageDragStart(imageId: number, event: DragEvent): void {
  draggingImageId.value = imageId
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.setData('text/plain', String(imageId))
  }
}

function handleImageDragEnd(): void {
  draggingImageId.value = null
}

function handleFolderDragOver(_: number | null, event: DragEvent): void {
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'move'
  }
}

async function handleFolderDrop(folderId: number | null): Promise<void> {
  const imageId = draggingImageId.value
  draggingImageId.value = null
  if (imageId === null) {
    return
  }
  await moveImageIdsToFolder([imageId], folderId)
}

function openPreview(image: MediaImageItem): void {
  previewImage.value = image
  previewDialog.value = true
}

function buildCopyText(mode: CopyMode, item: MediaImageItem): string {
  const url = item.url
  const fileName = getAssetFileName(item)
  if (mode === 'markdown') {
    if (item.media_type === 'image') {
      return `![${fileName}](${url})`
    }
    return `[${fileName}](${url})`
  }
  if (mode === 'html') {
    if (item.media_type === 'image') {
      return `<img src="${url}" alt="${fileName}" />`
    }
    if (item.media_type === 'video') {
      return `<video controls src="${url}"></video>`
    }
    if (item.media_type === 'audio') {
      return `<audio controls src="${url}"></audio>`
    }
    return `<a href="${url}">${fileName}</a>`
  }
  if (mode === 'bbcode') {
    if (item.media_type === 'image') {
      return `[img]${url}[/img]`
    }
    return `[url=${url}]${fileName}[/url]`
  }
  return url
}

async function copyText(text: string): Promise<void> {
  if (navigator.clipboard && typeof navigator.clipboard.writeText === 'function') {
    await navigator.clipboard.writeText(text)
    return
  }

  const input = document.createElement('textarea')
  input.value = text
  input.style.position = 'fixed'
  input.style.opacity = '0'
  document.body.appendChild(input)
  input.focus()
  input.select()
  document.execCommand('copy')
  document.body.removeChild(input)
}

async function copyPreviewAs(mode: CopyMode): Promise<void> {
  const item = previewImage.value
  if (!item?.url) {
    return
  }

  try {
    await copyText(buildCopyText(mode, item))
    const messageMap: Record<CopyMode, string> = {
      url: '已复制 URL',
      markdown: '已复制 Markdown',
      html: '已复制 HTML',
      bbcode: '已复制 BBCode',
    }
    showGlobalSuccess(messageMap[mode])
  } catch {
    errorMessage.value = '复制失败，请检查浏览器权限'
    showGlobalError('复制失败，请检查浏览器权限')
  }
}

async function deletePreviewImage(): Promise<void> {
  const item = previewImage.value
  if (!item || deletingImage.value) {
    return
  }

  deletingImage.value = true
  errorMessage.value = ''
  try {
    await deleteMediaImage(item.id)
    previewDialog.value = false
    previewImage.value = null
    showGlobalSuccess('图片删除成功')
    await reloadAll()
  } catch (error) {
    const message = error instanceof Error ? error.message : '删除图片失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    deletingImage.value = false
  }
}

async function uploadImages(files: File[]): Promise<void> {
  if (files.length <= 0 || uploadingImages.value) {
    return
  }

  uploadingImages.value = true
  errorMessage.value = ''
  let uploadedCount = 0
  let failedCount = 0

  try {
    showGlobalProgress(`资源上传中 0/${files.length} (0%)`, 0)
    for (let index = 0; index < files.length; index += 1) {
      const file = files[index]
      if (!file) {
        continue
      }
      const finishedBeforeCurrent = index
      try {
        await uploadMediaImage(file, {
          onProgress: ({ percent }) => {
            const overall = ((finishedBeforeCurrent + (percent / 100)) / files.length) * 100
            updateGlobalProgress(
              `资源上传中 ${finishedBeforeCurrent + 1}/${files.length} (${Math.round(overall)}%)`,
              overall,
            )
          },
        })
        uploadedCount += 1
      } catch {
        failedCount += 1
      }

      const finished = index + 1
      const overall = (finished / files.length) * 100
      updateGlobalProgress(`资源上传中 ${finished}/${files.length} (${Math.round(overall)}%)`, overall)
    }

    if (uploadedCount > 0) {
      showGlobalSuccess(`成功上传 ${uploadedCount} 个资源`)
      await reloadAll()
    } else {
      hideGlobalSnackbar()
    }

    if (failedCount > 0) {
      const message = `有 ${failedCount} 个资源上传失败`
      errorMessage.value = message
      showGlobalError(message)
    }
  } finally {
    uploadingImages.value = false
  }
}

async function handleUploadCardFiles(files: File[]): Promise<void> {
  await uploadImages(files)
}

onMounted(async () => {
  await reloadAll()
})

watch(mediaTypeFilter, () => {
  clearSelection()
})
</script>

<style scoped>
.media-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.page-header {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}

.header-text h1 {
  margin: 0;
  font-size: 28px;
  color: var(--admin-text-heading);
}

.header-text p {
  margin: 6px 0 0;
  color: var(--admin-text-muted);
}

.folders-card,
.images-card {
  border: 1px solid var(--admin-border);
  background: var(--admin-card-bg-strong);
}

.folders-head,
.images-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  flex-wrap: wrap;
}

.folders-row {
  display: flex;
  align-items: stretch;
  gap: 10px;
  overflow-x: auto;
  padding-bottom: 4px;
}

.folder-item {
  min-width: 168px;
  min-height: 94px;
  border: 1px solid var(--admin-accent-border);
  border-radius: 12px;
  background: var(--admin-accent-bg-soft);
  padding: 10px;
  cursor: pointer;
  transition:
    border-color 0.2s ease,
    background 0.2s ease,
    transform 0.2s ease;
}

.folder-item:hover {
  border-color: var(--admin-accent-border-strong);
  background: var(--admin-accent-bg);
  transform: translateY(-1px);
}

.folder-item--active {
  border-color: var(--admin-accent-border-strong);
  background: var(--admin-accent-bg);
}

.folder-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.folder-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.folder-name {
  color: var(--admin-accent-text);
  font-size: 14px;
  font-weight: 700;
  line-height: 1.3;
}

.folder-count {
  margin-top: 4px;
  color: var(--admin-text-faint);
  font-size: 12px;
}

.folder-create-card {
  min-width: 260px;
  border: 1px dashed var(--admin-accent-border-strong);
  border-radius: 12px;
  background: var(--admin-accent-bg-soft);
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.folder-create-title {
  color: var(--admin-accent-text);
  font-size: 13px;
  font-weight: 700;
}

.folder-create-input {
  min-width: 220px;
}

.images-title {
  color: var(--admin-text-heading);
  font-size: 16px;
  font-weight: 700;
}

.images-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.batch-select {
  width: 210px;
}

.media-type-filter {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 10px;
}

.type-filter-btn {
  min-width: 60px;
}

.images-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 10px;
}

.image-item {
  position: relative;
  border: 1px solid var(--admin-border);
  border-radius: 12px;
  overflow: hidden;
  background: var(--admin-overlay-panel-soft);
  cursor: pointer;
  transition:
    transform 0.2s ease,
    border-color 0.2s ease;
}

.image-item:hover {
  transform: translateY(-2px);
  border-color: var(--admin-accent-border);
}

.image-item--selected {
  border-color: var(--admin-accent-border-strong);
  box-shadow: 0 0 0 1px var(--admin-accent-bg);
}

.image-item img {
  display: block;
  width: 100%;
  height: 150px;
  object-fit: cover;
  background: var(--admin-surface-2);
}

.video-thumb {
  display: block;
  width: 100%;
  height: 150px;
  object-fit: cover;
  background: var(--admin-surface-2);
}

.file-thumb {
  height: 150px;
  display: grid;
  align-content: center;
  justify-items: center;
  gap: 8px;
  padding: 10px;
  background: var(--admin-card-bg-strong);
  color: var(--admin-text-secondary);
}

.file-thumb-name {
  width: 100%;
  font-size: 12px;
  text-align: center;
  color: var(--admin-accent-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.asset-type-badge {
  position: absolute;
  left: 6px;
  top: 6px;
  border-radius: 999px;
  padding: 2px 8px;
  font-size: 11px;
  line-height: 1.4;
  color: var(--admin-text-heading);
  background: var(--admin-overlay-mask);
}

.image-actions {
  position: absolute;
  top: 5px;
  right: 5px;
  border-radius: 999px;
  background: var(--admin-overlay-mask);
}

.images-empty {
  color: var(--admin-text-faint);
  font-size: 14px;
}

.upload-card {
  width: 100%;
}

.upload-card :deep(.image-upload-card) {
  min-height: 92px;
  padding: 16px 14px;
  border-radius: 14px;
}

.preview-dialog-card {
  overflow: hidden;
}

.preview-body {
  background: var(--admin-surface-3);
  padding: 0;
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 220px;
}

.preview-image {
  width: 100%;
  max-height: 70vh;
  object-fit: contain;
}

.preview-video {
  width: 100%;
  max-height: 70vh;
  background: var(--admin-surface-3);
}

.preview-audio {
  width: min(560px, calc(100% - 30px));
}

.preview-file-box {
  width: 100%;
  min-height: 260px;
  display: grid;
  justify-items: center;
  align-content: center;
  gap: 14px;
  color: var(--admin-text-secondary);
  padding: 24px;
}

.preview-file-name {
  max-width: 100%;
  font-size: 14px;
  word-break: break-all;
  text-align: center;
}

.preview-actions {
  gap: 6px;
  justify-content: flex-end;
  flex-wrap: wrap;
}

@media (max-width: 980px) {
  .page-header {
    align-items: flex-start;
  }

  .batch-select {
    width: 100%;
    min-width: 180px;
  }
}

@media (max-width: 700px) {
  .images-grid {
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
  }

  .image-item img,
  .video-thumb,
  .file-thumb {
    height: 120px;
  }
}
</style>
