<template>
  <section class="album-editor-page" @paste="handlePasteUpload">
    <header class="editor-header">
      <div class="header-text">
        <h1>{{ isEditing ? '编辑相册' : '新增相册' }}</h1>
        <p>左侧维护图片链接与拖拽排序，右侧维护基础信息。</p>
      </div>
      <div class="header-actions">
        <v-btn prepend-icon="mdi-arrow-left" variant="text" @click="goManage">
          返回管理
        </v-btn>
        <v-btn
          color="primary"
          prepend-icon="mdi-content-save-outline"
          :loading="submitting"
          @click="submitEditor"
        >
          {{ isEditing ? '保存修改' : '创建相册' }}
        </v-btn>
      </div>
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

    <div class="editor-grid">
      <section class="left-panel">
        <v-card class="panel-card link-editor-card" rounded="xl">
          <v-card-title class="card-title-row">
            <span>图片链接编辑（一行一个）</span>
            <v-btn
              color="primary"
              prepend-icon="mdi-plus"
              size="small"
              variant="text"
              @click="addImageUrlRow"
            >
              新增一行
            </v-btn>
          </v-card-title>
          <v-card-text class="link-editor-body">
            <ImageUploadHintCard
              class="album-upload-card"
              :loading="uploadingImage"
              :multiple="true"
              title="上传图片到相册"
              hint="拖动图片到卡片，或点击选择图片（支持多选与粘贴）"
              @select-files="handleUploadCardFiles"
            />

            <div class="url-rows">
              <div
                v-for="(_, index) in imageUrlEntries"
                :key="`image-url-${index}`"
                class="url-row"
              >
                <span class="url-index">{{ index + 1 }}</span>
                <v-text-field
                  v-model="imageUrlEntries[index]"
                  density="comfortable"
                  hide-details
                  placeholder="https://example.com/image.jpg"
                  variant="outlined"
                />
                <v-btn
                  color="error"
                  icon="mdi-delete-outline"
                  size="small"
                  variant="text"
                  :disabled="imageUrlEntries.length <= 1"
                  @click="removeImageUrlRow(index)"
                />
              </div>
            </div>
          </v-card-text>
        </v-card>

        <v-card class="panel-card preview-card" rounded="xl">
          <v-card-title class="card-title-row">
            <span>图片预览与排序（拖动卡片调整顺序）</span>
            <span class="preview-count">共 {{ previewImages.length }} 张</span>
          </v-card-title>
          <v-card-text class="preview-body">
            <div v-if="previewImages.length > 0" class="preview-grid">
              <article
                v-for="(url, index) in previewImages"
                :key="`${url}-${index}`"
                class="preview-item"
                :class="{
                  'preview-item--dragging': draggingPreviewIndex === index,
                  'preview-item--drop-target': hoverPreviewIndex === index,
                }"
                draggable="true"
                @dragstart="handlePreviewDragStart(index, $event)"
                @dragover="handlePreviewDragOver(index, $event)"
                @drop="handlePreviewDrop(index, $event)"
                @dragend="handlePreviewDragEnd"
              >
                <img :src="url" alt="album preview">
                <div class="preview-item-meta">
                  <span>#{{ index + 1 }}</span>
                  <v-btn
                    class="cover-btn"
                    size="x-small"
                    variant="tonal"
                    @click="editorForm.cover = url"
                  >
                    {{ editorForm.cover.trim() === url ? '当前封面' : '设为封面' }}
                  </v-btn>
                  <v-btn
                    color="error"
                    icon="mdi-delete-outline"
                    size="x-small"
                    variant="text"
                    @click="removeImageByValue(url)"
                  />
                </div>
              </article>
            </div>
            <div v-else class="preview-empty">暂无图片，可在上方粘贴、拖拽或输入链接。</div>
          </v-card-text>
        </v-card>
      </section>

      <aside class="right-panel">
        <v-card class="panel-card info-card" rounded="xl">
          <v-card-title>基础信息</v-card-title>
          <v-card-text class="info-form">
            <v-text-field
              v-model="editorForm.title"
              label="相册标题"
              variant="outlined"
            />

            <v-text-field
              v-model="editorForm.className"
              label="相册分类"
              variant="outlined"
            />

            <v-text-field
              v-model="editorForm.cover"
              label="封面链接（可选）"
              placeholder="默认使用第一张图片"
              variant="outlined"
            />

            <v-text-field
              v-model.number="editorForm.likeCount"
              label="点赞数（like_count）"
              min="0"
              type="number"
              variant="outlined"
            />

            <v-alert density="comfortable" type="info" variant="tonal">
              如果未填写封面链接，保存时会自动使用图片列表中的第一张。
            </v-alert>
          </v-card-text>
        </v-card>
      </aside>
    </div>
  </section>
</template>

<script lang="ts" setup>
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import {
  createAlbum,
  fetchAlbumById,
  joinAlbumImageUrls,
  parseAlbumImageUrls,
  updateAlbum,
  type AlbumUpsertPayload,
} from '@/services/albums'
import { uploadMarkdownImage } from '@/services/storage'
import ImageUploadHintCard from '@/components/admin/ImageUploadHintCard.vue'

const props = defineProps<{
  albumId?: number | null
}>()

const router = useRouter()
const {
  showGlobalSuccess,
  showGlobalProgress,
  updateGlobalProgress,
  hideGlobalSnackbar,
} = useGlobalSnackbar()

type EditorForm = {
  title: string
  className: string
  cover: string
  likeCount: number
}

const loading = ref(false)
const submitting = ref(false)
const uploadingImage = ref(false)
const errorMessage = ref('')
const imageUrlEntries = ref<string[]>([''])
const draggingPreviewIndex = ref<number | null>(null)
const hoverPreviewIndex = ref<number | null>(null)

const editorForm = reactive<EditorForm>({
  title: '',
  className: 'default',
  cover: '',
  likeCount: 0,
})

const isEditing = computed(() => Number.isFinite(props.albumId))
const previewImages = computed(() => parseAlbumImageUrls(joinAlbumImageUrls(imageUrlEntries.value)))

watch(imageUrlEntries, (items) => {
  if (items.length <= 0) {
    imageUrlEntries.value = ['']
  }
}, { deep: true })

function normalizeNumber(value: number): number {
  if (!Number.isFinite(value)) {
    return 0
  }
  return Math.max(0, Math.floor(value))
}

function ensureRows(): void {
  if (imageUrlEntries.value.length <= 0) {
    imageUrlEntries.value = ['']
  }
}

function addImageUrlRow(): void {
  ensureRows()
  imageUrlEntries.value.push('')
}

function removeImageUrlRow(index: number): void {
  if (imageUrlEntries.value.length <= 1) {
    imageUrlEntries.value[0] = ''
    return
  }
  imageUrlEntries.value.splice(index, 1)
  ensureRows()
}

function setPreviewUrls(urls: string[]): void {
  const unique = parseAlbumImageUrls(joinAlbumImageUrls(urls))
  imageUrlEntries.value = unique.length > 0 ? unique : ['']

  const cover = editorForm.cover.trim()
  if (cover && !unique.includes(cover)) {
    editorForm.cover = unique[0] || ''
  }
}

function removeImageByValue(url: string): void {
  const next = previewImages.value.filter((item) => item !== url)
  setPreviewUrls(next)
}

function buildPayload(): AlbumUpsertPayload | null {
  const title = editorForm.title.trim()
  const className = editorForm.className.trim()
  const cover = editorForm.cover.trim()
  const parsedImages = previewImages.value
  const imgUrls = joinAlbumImageUrls(parsedImages)

  if (!title) {
    errorMessage.value = '相册标题不能为空'
    return null
  }
  if (!className) {
    errorMessage.value = '相册分类不能为空'
    return null
  }

  return {
    title,
    class: className,
    like_count: normalizeNumber(editorForm.likeCount),
    cover: cover || parsedImages[0] || null,
    img_urls: imgUrls,
  }
}

function fillEditorForm(album: {
  title?: string | null
  class?: string | null
  cover?: string | null
  like_count?: number | null
  img_urls?: string | null
}): void {
  editorForm.title = album.title?.trim() || ''
  editorForm.className = album.class?.trim() || 'default'
  editorForm.cover = album.cover?.trim() || ''
  editorForm.likeCount = Number.isFinite(album.like_count) ? Number(album.like_count) : 0

  const parsed = parseAlbumImageUrls(album.img_urls)
  imageUrlEntries.value = parsed.length > 0 ? parsed : ['']
}

function pickClipboardImages(event: ClipboardEvent): File[] {
  const items = event.clipboardData?.items
  if (!items || items.length <= 0) {
    return []
  }

  const files: File[] = []
  for (const item of Array.from(items)) {
    if (item.kind !== 'file' || !item.type.startsWith('image/')) {
      continue
    }
    const file = item.getAsFile()
    if (file) {
      files.push(file)
    }
  }
  return files
}

function appendUploadedUrls(urls: string[]): void {
  if (urls.length <= 0) {
    return
  }

  const merged = [...previewImages.value, ...urls]
  setPreviewUrls(merged)

  if (!editorForm.cover.trim()) {
    editorForm.cover = previewImages.value[0] || urls[0] || ''
  }
}

async function uploadImages(files: File[]): Promise<void> {
  if (files.length <= 0 || uploadingImage.value) {
    return
  }

  uploadingImage.value = true
  errorMessage.value = ''

  const uploadedUrls: string[] = []
  const failedFiles: string[] = []
  const total = files.length

  try {
    showGlobalProgress(`图片上传中 0/${total} (0%)`, 0)

    for (let index = 0; index < files.length; index += 1) {
      const file = files[index]
      if (!file) {
        continue
      }
      const finishedBeforeCurrent = index

      try {
        const url = await uploadMarkdownImage(file, {
          onProgress: ({ percent }) => {
            const overall = ((finishedBeforeCurrent + (percent / 100)) / total) * 100
            updateGlobalProgress(
              `图片上传中 ${finishedBeforeCurrent + 1}/${total} (${Math.round(overall)}%)`,
              overall,
            )
          },
        })
        uploadedUrls.push(url)
      } catch {
        failedFiles.push(file.name || 'unknown')
      }

      const finished = index + 1
      const overall = (finished / total) * 100
      updateGlobalProgress(`图片上传中 ${finished}/${total} (${Math.round(overall)}%)`, overall)
    }

    if (uploadedUrls.length > 0) {
      appendUploadedUrls(uploadedUrls)
      showGlobalSuccess(`成功上传 ${uploadedUrls.length} 张图片`)
    } else {
      hideGlobalSnackbar()
    }

    if (failedFiles.length > 0) {
      errorMessage.value = `有 ${failedFiles.length} 张图片上传失败`
    }

    if (uploadedUrls.length <= 0 && failedFiles.length <= 0) {
      errorMessage.value = '未检测到可上传的图片文件'
    }
  } finally {
    uploadingImage.value = false
  }
}

async function handleUploadCardFiles(files: File[]): Promise<void> {
  await uploadImages(files)
}

async function handlePasteUpload(event: ClipboardEvent): Promise<void> {
  const files = pickClipboardImages(event)
  if (files.length <= 0) {
    return
  }
  event.preventDefault()
  await uploadImages(files)
}

function handlePreviewDragStart(index: number, event: DragEvent): void {
  draggingPreviewIndex.value = index
  hoverPreviewIndex.value = null
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.setData('text/plain', String(index))
  }
}

function handlePreviewDragOver(index: number, event: DragEvent): void {
  if (draggingPreviewIndex.value === null || draggingPreviewIndex.value === index) {
    return
  }
  event.preventDefault()
  hoverPreviewIndex.value = index
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'move'
  }
}

function handlePreviewDrop(index: number, event: DragEvent): void {
  event.preventDefault()
  const from = draggingPreviewIndex.value
  if (from === null || from === index) {
    handlePreviewDragEnd()
    return
  }

  const next = [...previewImages.value]
  const [moved] = next.splice(from, 1)
  if (!moved) {
    handlePreviewDragEnd()
    return
  }
  next.splice(index, 0, moved)
  setPreviewUrls(next)
  handlePreviewDragEnd()
}

function handlePreviewDragEnd(): void {
  draggingPreviewIndex.value = null
  hoverPreviewIndex.value = null
}

async function loadAlbumDetail(): Promise<void> {
  if (!isEditing.value || !props.albumId) {
    return
  }

  loading.value = true
  errorMessage.value = ''
  try {
    const album = await fetchAlbumById(props.albumId)
    fillEditorForm(album)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '加载相册详情失败'
  } finally {
    loading.value = false
  }
}

async function submitEditor(): Promise<void> {
  errorMessage.value = ''

  const payload = buildPayload()
  if (!payload) {
    return
  }

  submitting.value = true
  try {
    if (isEditing.value && props.albumId) {
      await updateAlbum(props.albumId, payload)
    } else {
      await createAlbum(payload)
    }
    showGlobalSuccess('相册发布成功')
    await router.push('/albums')
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '保存相册失败'
  } finally {
    submitting.value = false
  }
}

async function goManage(): Promise<void> {
  await router.push('/albums')
}

onMounted(async () => {
  await loadAlbumDetail()
})
</script>

<style scoped>
.album-editor-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
  height: calc(100dvh - 120px);
  min-height: 0;
  overflow: hidden;
}

.editor-header {
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

.header-actions {
  display: flex;
  gap: 10px;
  flex-shrink: 0;
}

.editor-grid {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(280px, 360px);
  gap: 14px;
}

.left-panel,
.right-panel {
  min-height: 0;
  min-width: 0;
}

.left-panel {
  display: grid;
  grid-template-rows: minmax(250px, 48%) minmax(0, 52%);
  gap: 14px;
}

.panel-card {
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: linear-gradient(180deg, #151c2a, #121826);
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.card-title-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.link-editor-body,
.preview-body,
.info-form {
  min-height: 0;
  flex: 1;
}

.link-editor-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.album-upload-card {
  width: 100%;
}

.url-rows {
  min-height: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-right: 2px;
}

.url-row {
  display: grid;
  grid-template-columns: 26px minmax(0, 1fr) 32px;
  align-items: center;
  gap: 6px;
}

.url-index {
  color: #c7d6f4;
  font-size: 13px;
  text-align: center;
}

.preview-count {
  color: #9fb0d4;
  font-size: 13px;
}

.preview-body {
  overflow: auto;
}

.preview-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 10px;
}

.preview-item {
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 12px;
  overflow: hidden;
  background: rgba(10, 14, 22, 0.6);
  cursor: grab;
  transition: border-color 0.2s ease, transform 0.2s ease;
}

.preview-item img {
  display: block;
  width: 100%;
  height: 110px;
  object-fit: cover;
  background: #0f1624;
}

.preview-item-meta {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px;
  color: #c8d6f3;
  font-size: 12px;
}

.cover-btn {
  margin-left: auto;
}

.preview-item:hover {
  transform: translateY(-1px);
  border-color: rgba(205, 218, 255, 0.6);
}

.preview-item--dragging {
  opacity: 0.45;
}

.preview-item--drop-target {
  border-color: rgba(205, 218, 255, 0.9);
  box-shadow: 0 0 0 2px rgba(157, 185, 255, 0.28);
}

.preview-empty {
  color: #93a4c7;
  font-size: 14px;
}

.info-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

@media (max-width: 1100px) {
  .album-editor-page {
    height: auto;
    overflow: visible;
  }

  .editor-grid {
    grid-template-columns: 1fr;
  }

  .left-panel {
    grid-template-rows: auto auto;
  }
}

@media (max-width: 900px) {
  .album-editor-page {
    min-height: auto;
  }

  .editor-header {
    flex-direction: column;
    align-items: stretch;
  }

  .header-actions {
    width: 100%;
  }

  .header-actions :deep(.v-btn) {
    flex: 1;
  }

  .preview-grid {
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  }
}
</style>
