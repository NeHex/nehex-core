<template>
  <div
    class="image-upload-card"
    :class="{
      'image-upload-card--active': dragOver,
      'image-upload-card--loading': loading,
      'image-upload-card--disabled': disabled,
    }"
    role="button"
    tabindex="0"
    @click="triggerPick"
    @keydown.enter.prevent="triggerPick"
    @keydown.space.prevent="triggerPick"
    @dragenter.prevent="onDragEnter"
    @dragover.prevent="onDragOver"
    @dragleave.prevent="onDragLeave"
    @drop.prevent="onDrop"
  >
    <input
      v-if="mode === 'upload'"
      ref="inputRef"
      :accept="accept"
      class="image-upload-input"
      :multiple="multiple"
      type="file"
      @change="onInputChange"
    >

    <div class="image-upload-icon">
      <v-progress-circular
        v-if="loading"
        color="primary"
        indeterminate
        size="18"
        width="2"
      />
      <v-icon
        v-else
        color="#9fb4de"
        :icon="resolvedIcon"
        size="18"
      />
    </div>

    <div class="image-upload-text">
      <div class="image-upload-title">{{ loading ? loadingTitle : title }}</div>
      <div class="image-upload-hint">{{ hint }}</div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'

const props = withDefaults(defineProps<{
  title?: string
  hint?: string
  loading?: boolean
  loadingTitle?: string
  icon?: string
  mode?: 'upload' | 'action'
  multiple?: boolean
  disabled?: boolean
  accept?: string
  fileFilterMode?: 'image' | 'any'
}>(), {
  title: '上传图片',
  hint: '拖动图片到卡片，或点击选择图片',
  loading: false,
  loadingTitle: '正在上传图片...',
  icon: '',
  mode: 'upload',
  multiple: false,
  disabled: false,
  accept: 'image/*',
  fileFilterMode: 'image',
})

const emit = defineEmits<{
  (event: 'select-files', files: File[]): void
  (event: 'activate'): void
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const dragOver = ref(false)
const dragDepth = ref(0)
const IMAGE_EXTENSION_PATTERN = /\.(jpg|jpeg|png|webp|gif|svg|bmp|avif)$/i

const resolvedIcon = computed(() => {
  if (props.icon) {
    return props.icon
  }
  return props.fileFilterMode === 'any' ? 'mdi-upload-outline' : 'mdi-image-plus-outline'
})

function canHandleDrop(): boolean {
  return props.mode === 'upload' && !props.loading && !props.disabled
}

function isImageFile(file: File): boolean {
  if (file.type.startsWith('image/')) {
    return true
  }
  const name = (file.name || '').trim().toLowerCase()
  return IMAGE_EXTENSION_PATTERN.test(name)
}

function pickFiles(files: FileList | null): File[] {
  if (!files || files.length <= 0) {
    return []
  }

  const selected = Array.from(files)
  if (props.fileFilterMode === 'any') {
    return selected
  }
  return selected.filter((file) => isImageFile(file))
}

function triggerPick(): void {
  if (props.loading || props.disabled) {
    return
  }
  if (props.mode === 'action') {
    emit('activate')
    return
  }
  inputRef.value?.click()
}

function onInputChange(event: Event): void {
  const target = event.target as HTMLInputElement | null
  const files = pickFiles(target?.files || null)
  if (target) {
    target.value = ''
  }
  if (files.length <= 0) {
    return
  }
  emit('select-files', files)
}

function onDragEnter(): void {
  if (!canHandleDrop()) {
    return
  }
  dragDepth.value += 1
  dragOver.value = true
}

function onDragOver(event: DragEvent): void {
  if (!canHandleDrop()) {
    return
  }
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy'
  }
  dragOver.value = true
}

function onDragLeave(): void {
  dragDepth.value = Math.max(0, dragDepth.value - 1)
  if (dragDepth.value === 0) {
    dragOver.value = false
  }
}

function onDrop(event: DragEvent): void {
  dragDepth.value = 0
  dragOver.value = false
  if (!canHandleDrop()) {
    return
  }

  const files = pickFiles(event.dataTransfer?.files || null)
  if (files.length <= 0) {
    return
  }
  emit('select-files', files)
}
</script>

<style scoped>
.image-upload-card {
  display: flex;
  align-items: center;
  gap: 10px;
  border: 1px dashed rgba(171, 192, 245, 0.55);
  border-radius: 12px;
  background: rgba(144, 166, 219, 0.08);
  padding: 10px 12px;
  cursor: pointer;
  user-select: none;
  transition:
    border-color 0.2s ease,
    background 0.2s ease,
    transform 0.2s ease;
}

.image-upload-card:hover {
  border-color: rgba(191, 210, 255, 0.9);
  background: rgba(144, 166, 219, 0.16);
  transform: translateY(-1px);
}

.image-upload-card--active {
  border-color: rgba(191, 210, 255, 0.95);
  background: rgba(144, 166, 219, 0.22);
}

.image-upload-card--loading {
  cursor: wait;
}

.image-upload-card--disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.image-upload-input {
  display: none;
}

.image-upload-icon {
  width: 20px;
  height: 20px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
}

.image-upload-text {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.image-upload-title {
  color: #dbe7ff;
  font-size: 13px;
  font-weight: 700;
  line-height: 1.2;
}

.image-upload-hint {
  color: #a6b8dc;
  font-size: 12px;
  line-height: 1.3;
}
</style>
