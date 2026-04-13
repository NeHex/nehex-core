<template>
  <v-dialog
    :model-value="modelValue"
    max-width="1080"
    @update:model-value="handleModelUpdate"
  >
    <v-card class="media-picker-card" rounded="xl">
      <v-card-title class="media-picker-head">
        <div>
          <div class="media-picker-title">从媒体库选择图片</div>
          <div class="media-picker-subtitle">确认后会把图片插入到当前光标位置。</div>
        </div>
      </v-card-title>

      <v-card-text class="media-picker-body">
        <v-alert
          v-if="errorMessage"
          class="mb-3"
          density="comfortable"
          type="error"
          variant="tonal"
        >
          {{ errorMessage }}
        </v-alert>

        <v-progress-linear
          v-if="loadingLibrary"
          class="mb-3"
          color="primary"
          indeterminate
        />

        <div class="media-picker-layout">
          <aside class="folder-sidebar">
            <button
              v-for="folder in folderItems"
              :key="folder.key"
              class="folder-item"
              :class="{ 'folder-item--active': currentFolderId === folder.id }"
              type="button"
              @click="selectFolder(folder.id)"
            >
              <span class="folder-item-name">{{ folder.name }}</span>
              <span class="folder-item-count">{{ folder.count }}</span>
            </button>
          </aside>

          <section class="image-panel">
            <v-progress-linear
              v-if="loadingFolderImages"
              class="mb-3"
              color="primary"
              indeterminate
            />

            <div v-if="displayedImages.length > 0" class="image-grid">
              <button
                v-for="image in displayedImages"
                :key="image.id"
                class="image-item"
                :class="{ 'image-item--active': selectedImageId === image.id }"
                type="button"
                @click="selectImage(image.id)"
              >
                <img :src="image.url" :alt="resolveFileName(image)">
                <div class="image-item-name">{{ resolveFileName(image) }}</div>
              </button>
            </div>
            <div v-else class="image-empty">当前分类暂无可用图片</div>
          </section>
        </div>
      </v-card-text>

      <v-card-actions class="media-picker-actions">
        <v-spacer />
        <v-btn variant="text" @click="closeDialog">取消</v-btn>
        <v-btn
          color="primary"
          prepend-icon="mdi-image-plus-outline"
          :disabled="!selectedImage"
          @click="insertSelectedImage"
        >
          插入图片
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script lang="ts" setup>
import { computed, ref, watch } from 'vue'
import {
  fetchMediaImagesByFolder,
  fetchMediaLibrary,
  type MediaFolderItem,
  type MediaImageItem,
} from '@/services/media-library'

type PickerFolderItem = {
  key: string
  id: number | null
  name: string
  count: number
}

type SelectedImagePayload = {
  url: string
  fileName: string
}

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
  (event: 'select-image', payload: SelectedImagePayload): void
}>()

const loadingLibrary = ref(false)
const loadingFolderImages = ref(false)
const errorMessage = ref('')

const folders = ref<MediaFolderItem[]>([])
const uncategorizedImages = ref<MediaImageItem[]>([])
const currentFolderImages = ref<MediaImageItem[]>([])
const currentFolderId = ref<number | null>(null)
const selectedImageId = ref<number | null>(null)

const uncategorizedImageItems = computed(() => {
  return uncategorizedImages.value.filter((item) => item.media_type === 'image' && !!item.url)
})

const currentFolderImageItems = computed(() => {
  return currentFolderImages.value.filter((item) => item.media_type === 'image' && !!item.url)
})

const displayedImages = computed(() => {
  return currentFolderId.value === null
    ? uncategorizedImageItems.value
    : currentFolderImageItems.value
})

const folderItems = computed<PickerFolderItem[]>(() => {
  const items: PickerFolderItem[] = [
    {
      key: 'uncategorized',
      id: null,
      name: '未分类',
      count: uncategorizedImageItems.value.length,
    },
  ]

  for (const folder of folders.value) {
    items.push({
      key: `folder-${folder.id}`,
      id: folder.id,
      name: folder.name,
      count: Math.max(0, Number(folder.image_count) || 0),
    })
  }

  return items
})

const selectedImage = computed(() => {
  const id = selectedImageId.value
  if (!id) {
    return null
  }
  return displayedImages.value.find((item) => item.id === id) || null
})

watch(displayedImages, (items) => {
  if (!selectedImageId.value) {
    return
  }

  const stillExists = items.some((item) => item.id === selectedImageId.value)
  if (!stillExists) {
    selectedImageId.value = null
  }
})

watch(
  () => props.modelValue,
  (visible) => {
    if (!visible) {
      return
    }
    void loadLibrary()
  },
)

function handleModelUpdate(value: boolean): void {
  emit('update:modelValue', value)
}

function closeDialog(): void {
  emit('update:modelValue', false)
}

function resolveFileName(image: MediaImageItem): string {
  const directName = (image.file_name || '').trim()
  if (directName) {
    return directName
  }

  const keyName = (image.key || '').trim().split('/').filter(Boolean).pop() || ''
  if (keyName) {
    return decodeURIComponentSafely(keyName)
  }

  return extractFileNameFromUrl(image.url) || `image-${image.id}`
}

function extractFileNameFromUrl(url: string): string {
  const raw = (url || '').trim()
  if (!raw) {
    return ''
  }

  try {
    const pathname = new URL(raw).pathname
    const name = pathname.split('/').filter(Boolean).pop() || ''
    return decodeURIComponentSafely(name)
  } catch {
    const name = raw.split('?')[0]?.split('/').filter(Boolean).pop() || ''
    return decodeURIComponentSafely(name)
  }
}

function decodeURIComponentSafely(value: string): string {
  try {
    return decodeURIComponent(value)
  } catch {
    return value
  }
}

function selectImage(imageId: number): void {
  selectedImageId.value = imageId
}

async function loadLibrary(): Promise<void> {
  loadingLibrary.value = true
  errorMessage.value = ''
  selectedImageId.value = null

  try {
    const library = await fetchMediaLibrary()
    folders.value = library.folders
    uncategorizedImages.value = library.uncategorized
    currentFolderImages.value = []
    currentFolderId.value = null
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载媒体库失败'
    errorMessage.value = message
  } finally {
    loadingLibrary.value = false
  }
}

async function selectFolder(folderId: number | null): Promise<void> {
  if (currentFolderId.value === folderId) {
    return
  }

  currentFolderId.value = folderId
  selectedImageId.value = null

  if (folderId === null) {
    currentFolderImages.value = []
    return
  }

  loadingFolderImages.value = true
  errorMessage.value = ''

  try {
    currentFolderImages.value = await fetchMediaImagesByFolder(folderId)
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载分类图片失败'
    errorMessage.value = message
    currentFolderImages.value = []
  } finally {
    loadingFolderImages.value = false
  }
}

function insertSelectedImage(): void {
  const image = selectedImage.value
  if (!image) {
    return
  }

  emit('select-image', {
    url: image.url,
    fileName: resolveFileName(image),
  })
  closeDialog()
}
</script>

<style scoped>
.media-picker-card {
  background: #111826;
  color: #edf3ff;
}

.media-picker-head {
  padding: 14px 16px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.media-picker-title {
  font-size: 18px;
  font-weight: 700;
  color: #edf3ff;
}

.media-picker-subtitle {
  margin-top: 4px;
  font-size: 13px;
  color: #9fb0d4;
}

.media-picker-body {
  padding: 12px 16px 8px;
}

.media-picker-layout {
  display: grid;
  grid-template-columns: 210px minmax(0, 1fr);
  gap: 12px;
  min-height: 420px;
}

.folder-sidebar {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 12px;
  padding: 8px;
  background: rgba(255, 255, 255, 0.03);
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.folder-item {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.04);
  color: #d9e6ff;
  padding: 8px 10px;
  text-align: left;
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  transition: border-color 0.2s ease, background 0.2s ease;
}

.folder-item:hover {
  border-color: rgba(160, 191, 255, 0.6);
}

.folder-item--active {
  border-color: rgba(160, 191, 255, 0.85);
  background: rgba(116, 147, 216, 0.2);
}

.folder-item-name {
  max-width: 128px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.folder-item-count {
  font-size: 12px;
  color: #9fb0d4;
}

.image-panel {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 12px;
  padding: 10px;
  background: rgba(255, 255, 255, 0.02);
  overflow: auto;
}

.image-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 10px;
}

.image-item {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  background: #0f1624;
  padding: 8px;
  text-align: left;
  cursor: pointer;
  transition: border-color 0.2s ease, transform 0.2s ease;
}

.image-item:hover {
  border-color: rgba(160, 191, 255, 0.6);
  transform: translateY(-1px);
}

.image-item--active {
  border-color: rgba(160, 191, 255, 0.95);
  box-shadow: 0 0 0 1px rgba(160, 191, 255, 0.25);
}

.image-item img {
  width: 100%;
  aspect-ratio: 4 / 3;
  object-fit: cover;
  border-radius: 8px;
  display: block;
}

.image-item-name {
  margin-top: 6px;
  font-size: 12px;
  color: #c7d7f5;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.image-empty {
  min-height: 220px;
  display: grid;
  place-items: center;
  color: #9fb0d4;
  font-size: 13px;
}

.media-picker-actions {
  padding: 8px 16px 14px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

@media (max-width: 900px) {
  .media-picker-layout {
    grid-template-columns: 1fr;
    min-height: 0;
  }

  .folder-sidebar {
    max-height: 180px;
  }
}
</style>
