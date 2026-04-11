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
      ref="inputRef"
      accept="image/*"
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
      <v-icon v-else color="#9fb4de" icon="mdi-image-plus-outline" size="18" />
    </div>

    <div class="image-upload-text">
      <div class="image-upload-title">{{ loading ? loadingTitle : title }}</div>
      <div class="image-upload-hint">{{ hint }}</div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { ref } from 'vue'

const props = withDefaults(defineProps<{
  title?: string
  hint?: string
  loading?: boolean
  loadingTitle?: string
  multiple?: boolean
  disabled?: boolean
}>(), {
  title: '上传图片',
  hint: '拖动图片到卡片，或点击选择图片',
  loading: false,
  loadingTitle: '正在上传图片...',
  multiple: false,
  disabled: false,
})

const emit = defineEmits<{
  (event: 'select-files', files: File[]): void
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const dragOver = ref(false)
const dragDepth = ref(0)

function pickImages(files: FileList | null): File[] {
  if (!files || files.length <= 0) {
    return []
  }

  return Array.from(files).filter((file) => file.type.startsWith('image/'))
}

function triggerPick(): void {
  if (props.loading || props.disabled) {
    return
  }
  inputRef.value?.click()
}

function onInputChange(event: Event): void {
  const target = event.target as HTMLInputElement | null
  const files = pickImages(target?.files || null)
  if (target) {
    target.value = ''
  }
  if (files.length <= 0) {
    return
  }
  emit('select-files', files)
}

function onDragEnter(): void {
  if (props.loading || props.disabled) {
    return
  }
  dragDepth.value += 1
  dragOver.value = true
}

function onDragOver(event: DragEvent): void {
  if (props.loading || props.disabled) {
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
  if (props.loading || props.disabled) {
    return
  }

  const files = pickImages(event.dataTransfer?.files || null)
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
