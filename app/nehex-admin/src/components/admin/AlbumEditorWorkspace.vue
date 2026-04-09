<template>
  <section class="album-editor-page">
    <header class="editor-header">
      <div class="header-text">
        <h1>{{ isEditing ? '编辑相册' : '新增相册' }}</h1>
        <p>左侧维护基础信息和图片链接，右侧实时预览相册图片。</p>
      </div>
      <div class="header-actions">
        <v-btn
          prepend-icon="mdi-arrow-left"
          variant="text"
          @click="goManage"
        >
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

    <v-alert
      v-if="successMessage"
      class="mb-4"
      density="comfortable"
      type="success"
      variant="tonal"
    >
      {{ successMessage }}
    </v-alert>

    <v-progress-linear
      v-if="loading"
      class="mb-4"
      color="primary"
      indeterminate
    />

    <div class="editor-grid">
      <section class="left-panel">
        <v-card class="panel-card info-card" rounded="xl">
          <v-card-title>基础信息</v-card-title>
          <v-card-text class="panel-body">
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
              variant="outlined"
            />

            <v-text-field
              v-model.number="editorForm.likeCount"
              label="点赞数（like_count）"
              min="0"
              type="number"
              variant="outlined"
            />

            <div class="image-url-editor">
              <div class="image-url-head">
                <span>图片链接（竖向滑动）</span>
                <span>{{ imageUrlEntries.length > 0 ? activeImageUrlIndex + 1 : 0 }} / {{ imageUrlEntries.length }}</span>
              </div>

              <div class="image-url-actions">
                <v-btn
                  icon="mdi-chevron-up"
                  size="small"
                  variant="text"
                  :disabled="activeImageUrlIndex <= 0"
                  @click="moveActiveImageUrl(-1)"
                />
                <v-btn
                  icon="mdi-chevron-down"
                  size="small"
                  variant="text"
                  :disabled="activeImageUrlIndex >= imageUrlEntries.length - 1"
                  @click="moveActiveImageUrl(1)"
                />
                <v-btn
                  prepend-icon="mdi-plus"
                  size="small"
                  variant="text"
                  @click="addImageUrlEntry"
                >
                  新增
                </v-btn>
                <v-btn
                  color="error"
                  prepend-icon="mdi-delete-outline"
                  size="small"
                  variant="text"
                  :disabled="imageUrlEntries.length <= 1 && !currentImageUrlText"
                  @click="removeCurrentImageUrlEntry"
                >
                  删除当前
                </v-btn>
              </div>

              <div ref="imageUrlViewportRef" class="image-url-viewport">
                <div
                  v-for="(_, index) in imageUrlEntries"
                  :key="`image-url-${index}`"
                  class="image-url-row"
                  :class="{ 'image-url-row--active': index === activeImageUrlIndex }"
                  :data-image-url-row="index"
                  @click="activeImageUrlIndex = index"
                >
                  <span class="image-url-row-index">{{ index + 1 }}</span>
                  <v-text-field
                    v-model="imageUrlEntries[index]"
                    density="compact"
                    hide-details
                    placeholder="https://example.com/image-1.jpg"
                    variant="outlined"
                  />
                </div>
              </div>
            </div>
          </v-card-text>
        </v-card>

        <v-card class="panel-card upload-card" rounded="xl">
          <v-card-title>上传图片</v-card-title>
          <v-card-text>
            <div class="upload-actions">
              <input
                ref="imageInputRef"
                accept="image/*"
                class="upload-input"
                multiple
                type="file"
                @change="handleImageInputChange"
              >
              <v-btn
                color="primary"
                density="comfortable"
                prepend-icon="mdi-image-plus-outline"
                size="small"
                :loading="uploadingImage"
                @click="triggerImageSelect"
              >
                选择图片上传
              </v-btn>
            </div>
            <div
              class="upload-dropzone"
              :class="{ 'upload-dropzone--active': uploadZoneActive }"
              @dragenter.prevent="uploadZoneActive = true"
              @dragover.prevent="uploadZoneActive = true"
              @dragleave.prevent="uploadZoneActive = false"
              @drop.prevent="handleDropFiles"
            >
              <v-icon icon="mdi-cloud-upload-outline" size="36" />
              <div class="dropzone-title">拖拽图片到这里上传</div>
              <div class="dropzone-desc">支持多图上传，成功后将自动追加到“图片链接”输入框。</div>
            </div>
          </v-card-text>
        </v-card>
      </section>

      <section class="right-panel">
        <v-card class="panel-card preview-card" rounded="xl">
          <v-card-title>图片预览</v-card-title>
          <v-card-text class="preview-stage-wrap">
            <div class="preview-stage">
              <img
                v-if="currentPreviewUrl"
                :key="currentPreviewUrl"
                :src="currentPreviewUrl"
                alt="album preview"
                class="preview-image"
              >
              <div v-else class="preview-empty">暂无可预览图片</div>

              <v-btn
                class="nav-btn nav-btn-left"
                icon="mdi-chevron-left"
                size="small"
                variant="tonal"
                :disabled="previewImages.length <= 1"
                @click="showPrevImage"
              />
              <v-btn
                class="nav-btn nav-btn-right"
                icon="mdi-chevron-right"
                size="small"
                variant="tonal"
                :disabled="previewImages.length <= 1"
                @click="showNextImage"
              />
            </div>

            <div class="preview-meta">
              <span>共 {{ previewImages.length }} 张</span>
              <span v-if="previewImages.length > 0">
                第 {{ activePreviewIndex + 1 }} 张
              </span>
            </div>
          </v-card-text>
        </v-card>

        <v-card class="panel-card thumbs-card" rounded="xl">
          <v-card-title>预览图</v-card-title>
          <v-card-text class="thumbs-wrap">
            <div v-if="previewImages.length > 0" class="thumbs-list">
              <button
                v-for="(url, index) in previewImages"
                :key="`${url}-${index}`"
                class="thumb-item"
                :class="{ 'thumb-item--active': index === activePreviewIndex }"
                type="button"
                @click="selectPreview(index)"
              >
                <img :src="url" alt="thumb">
              </button>
            </div>
            <div v-else class="thumbs-empty">暂无预览图</div>
          </v-card-text>
        </v-card>
      </section>
    </div>
  </section>
</template>

<script lang="ts" setup>
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue'
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

const props = defineProps<{
  albumId?: number | null
}>()

const router = useRouter()

type EditorForm = {
  title: string
  className: string
  cover: string
  likeCount: number
}

const loading = ref(false)
const submitting = ref(false)
const uploadingImage = ref(false)
const uploadZoneActive = ref(false)
const errorMessage = ref('')
const successMessage = ref('')
const activePreviewIndex = ref(0)
const activeImageUrlIndex = ref(0)
const imageUrlEntries = ref<string[]>([''])
const imageInputRef = ref<HTMLInputElement | null>(null)
const imageUrlViewportRef = ref<HTMLElement | null>(null)
const { showGlobalSuccess } = useGlobalSnackbar()

const editorForm = reactive<EditorForm>({
  title: '',
  className: 'default',
  cover: '',
  likeCount: 0,
})

const isEditing = computed(() => Number.isFinite(props.albumId))

const normalizedImageUrls = computed(() => parseAlbumImageUrls(joinAlbumImageUrls(imageUrlEntries.value)))
const currentImageUrlText = computed(() => imageUrlEntries.value[activeImageUrlIndex.value]?.trim() || '')

const previewImages = computed(() => {
  if (normalizedImageUrls.value.length > 0) {
    return normalizedImageUrls.value
  }
  const cover = editorForm.cover.trim()
  if (cover) {
    return [cover]
  }
  return []
})

const currentPreviewUrl = computed(() => {
  if (previewImages.value.length === 0) {
    return ''
  }
  return previewImages.value[activePreviewIndex.value] || previewImages.value[0]
})

watch(previewImages, (items) => {
  if (items.length === 0) {
    activePreviewIndex.value = 0
    return
  }
  if (activePreviewIndex.value >= items.length) {
    activePreviewIndex.value = items.length - 1
  }
})

watch(imageUrlEntries, (items) => {
  if (items.length <= 0) {
    imageUrlEntries.value = ['']
    activeImageUrlIndex.value = 0
    return
  }
  if (activeImageUrlIndex.value >= items.length) {
    activeImageUrlIndex.value = items.length - 1
  }
}, { deep: true })

watch(activeImageUrlIndex, async () => {
  await nextTick()
  const viewport = imageUrlViewportRef.value
  if (!viewport) {
    return
  }
  const row = viewport.querySelector<HTMLElement>(`[data-image-url-row="${activeImageUrlIndex.value}"]`)
  row?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
})

function normalizeNumber(value: number): number {
  if (!Number.isFinite(value)) {
    return 0
  }
  return Math.max(0, Math.floor(value))
}

function buildPayload(): AlbumUpsertPayload | null {
  const title = editorForm.title.trim()
  const className = editorForm.className.trim()
  const cover = editorForm.cover.trim()
  const parsedImages = normalizedImageUrls.value
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
  imageUrlEntries.value = parseAlbumImageUrls(album.img_urls)
  if (imageUrlEntries.value.length <= 0) {
    imageUrlEntries.value = ['']
  }
  activeImageUrlIndex.value = 0
}

function ensureImageUrlEntries(): void {
  if (imageUrlEntries.value.length <= 0) {
    imageUrlEntries.value = ['']
    activeImageUrlIndex.value = 0
  }
}

function addImageUrlEntry(): void {
  ensureImageUrlEntries()
  const targetIndex = activeImageUrlIndex.value + 1
  imageUrlEntries.value.splice(targetIndex, 0, '')
  activeImageUrlIndex.value = targetIndex
}

function removeCurrentImageUrlEntry(): void {
  ensureImageUrlEntries()
  if (imageUrlEntries.value.length <= 1) {
    imageUrlEntries.value[0] = ''
    activeImageUrlIndex.value = 0
    return
  }
  imageUrlEntries.value.splice(activeImageUrlIndex.value, 1)
  if (activeImageUrlIndex.value >= imageUrlEntries.value.length) {
    activeImageUrlIndex.value = imageUrlEntries.value.length - 1
  }
}

function moveActiveImageUrl(step: -1 | 1): void {
  const total = imageUrlEntries.value.length
  if (total <= 0) {
    activeImageUrlIndex.value = 0
    return
  }
  activeImageUrlIndex.value = Math.min(total - 1, Math.max(0, activeImageUrlIndex.value + step))
}

function selectPreview(index: number): void {
  if (index < 0 || index >= previewImages.value.length) {
    return
  }
  activePreviewIndex.value = index
}

function showPrevImage(): void {
  const total = previewImages.value.length
  if (total <= 1) {
    return
  }
  activePreviewIndex.value = (activePreviewIndex.value - 1 + total) % total
}

function showNextImage(): void {
  const total = previewImages.value.length
  if (total <= 1) {
    return
  }
  activePreviewIndex.value = (activePreviewIndex.value + 1) % total
}

function triggerImageSelect(): void {
  imageInputRef.value?.click()
}

function _pickImageFiles(files: FileList | null): File[] {
  if (!files || files.length <= 0) {
    return []
  }
  return Array.from(files).filter((file) => file.type.startsWith('image/'))
}

function _appendUploadedUrls(urls: string[]): void {
  if (urls.length <= 0) {
    return
  }

  const existing = normalizedImageUrls.value
  const merged = new Set(existing)
  urls.forEach((item) => merged.add(item))

  const nextUrls = Array.from(merged)
  imageUrlEntries.value = nextUrls.length > 0 ? nextUrls : ['']

  if (!editorForm.cover.trim()) {
    editorForm.cover = urls[0] || ''
  }

  const firstInserted = nextUrls.findIndex((item) => item === urls[0])
  if (firstInserted >= 0) {
    activeImageUrlIndex.value = firstInserted
    activePreviewIndex.value = firstInserted
  }
}

async function _uploadImages(files: File[]): Promise<void> {
  if (files.length <= 0 || uploadingImage.value) {
    return
  }

  uploadingImage.value = true
  successMessage.value = ''
  errorMessage.value = ''

  const uploadedUrls: string[] = []
  const failedFiles: string[] = []

  try {
    for (const file of files) {
      try {
        const url = await uploadMarkdownImage(file)
        uploadedUrls.push(url)
      } catch {
        failedFiles.push(file.name || 'unknown')
      }
    }

    if (uploadedUrls.length > 0) {
      _appendUploadedUrls(uploadedUrls)
      successMessage.value = `成功上传 ${uploadedUrls.length} 张图片`
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

async function handleImageInputChange(event: Event): Promise<void> {
  const target = event.target as HTMLInputElement | null
  const files = _pickImageFiles(target?.files || null)
  if (target) {
    target.value = ''
  }
  await _uploadImages(files)
}

async function handleDropFiles(event: DragEvent): Promise<void> {
  uploadZoneActive.value = false
  const files = _pickImageFiles(event.dataTransfer?.files || null)
  await _uploadImages(files)
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
  successMessage.value = ''
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
  display: grid;
  grid-template-columns: minmax(320px, 44%) minmax(0, 56%);
  gap: 14px;
  flex: 1;
  min-height: 0;
}

.left-panel,
.right-panel {
  min-width: 0;
  min-height: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.panel-card {
  display: flex;
  flex-direction: column;
  min-height: 0;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: linear-gradient(180deg, #151c2a, #121826);
}

.info-card {
  flex: 1.15;
}

.panel-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.image-url-editor {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 12px;
  background: rgba(146, 168, 223, 0.08);
}

.image-url-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  color: #d5e2ff;
}

.image-url-actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
}

.image-url-viewport {
  max-height: 220px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-right: 2px;
}

.image-url-row {
  display: grid;
  grid-template-columns: 30px minmax(0, 1fr);
  gap: 8px;
  align-items: center;
  padding: 6px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.03);
  transition:
    border-color 0.2s ease,
    background 0.2s ease;
}

.image-url-row-index {
  text-align: center;
  font-size: 12px;
  color: #c7d6f4;
}

.image-url-row--active {
  border-color: rgba(204, 220, 255, 0.82);
  background: rgba(157, 185, 255, 0.2);
}

.upload-card {
  flex: 0.85;
  min-height: 0;
}

.upload-actions {
  display: flex;
  justify-content: flex-start;
  margin-bottom: 10px;
}

.upload-input {
  display: none;
}

.upload-dropzone {
  min-height: 150px;
  border: 1px dashed rgba(171, 192, 245, 0.55);
  border-radius: 14px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: 8px;
  color: #d5e2ff;
  background: rgba(144, 166, 219, 0.08);
  transition:
    border-color 0.2s ease,
    background 0.2s ease;
}

.upload-dropzone--active {
  border-color: rgba(191, 210, 255, 0.9);
  background: rgba(144, 166, 219, 0.16);
}

.dropzone-title {
  font-size: 16px;
  font-weight: 700;
}

.dropzone-desc {
  font-size: 13px;
  color: #a6b3cf;
}

.preview-card {
  flex: 1;
  min-height: 0;
}

.preview-stage-wrap {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex: 1;
  min-height: 0;
}

.preview-stage {
  position: relative;
  flex: 1;
  min-height: 0;
  border-radius: 16px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  overflow: auto;
  background: #0f1624;
  display: flex;
  align-items: center;
  justify-content: center;
}

.preview-image {
  width: 100%;
  height: 100%;
  object-fit: contain;
  object-position: center;
}

.preview-empty {
  color: #93a4c7;
  font-size: 14px;
}

.nav-btn {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
}

.nav-btn-left {
  left: 12px;
}

.nav-btn-right {
  right: 12px;
}

.preview-meta {
  display: flex;
  justify-content: space-between;
  color: #adbada;
  font-size: 13px;
}

.thumbs-card {
  flex: 0 0 210px;
}

.thumbs-wrap {
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.thumbs-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
  gap: 8px;
}

.thumb-item {
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 10px;
  overflow: hidden;
  padding: 0;
  background: transparent;
  cursor: pointer;
  transition:
    border-color 0.2s ease,
    transform 0.2s ease;
}

.thumb-item img {
  width: 100%;
  height: 70px;
  object-fit: cover;
  display: block;
}

.thumb-item:hover {
  transform: translateY(-1px);
  border-color: rgba(205, 218, 255, 0.6);
}

.thumb-item--active {
  border-color: rgba(205, 218, 255, 0.88);
  box-shadow: 0 0 0 2px rgba(174, 196, 255, 0.24);
}

.thumbs-empty {
  color: #93a4c7;
  font-size: 14px;
}

@media (max-width: 1100px) {
  .album-editor-page {
    height: auto;
    overflow: visible;
  }

  .editor-grid {
    grid-template-columns: 1fr;
    flex: none;
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
}
</style>
