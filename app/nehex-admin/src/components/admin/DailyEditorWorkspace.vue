<template>
  <section class="daily-editor-page">
    <header class="editor-header">
      <div class="header-text">
        <h1>{{ isEditing ? '编辑日常' : '新增日常' }}</h1>
        <p>左侧编辑 Markdown，右侧实时预览；拖动中间分割线可调整宽度。</p>
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
          {{ isEditing ? '保存修改' : '创建日常' }}
        </v-btn>
      </div>
    </header>

    <v-progress-linear
      v-if="loading"
      class="mb-4"
      color="primary"
      indeterminate
    />

    <div class="meta-grid">
      <v-text-field
        v-model="editorForm.title"
        label="日常标题"
        variant="outlined"
      />

      <v-select
        v-model="editorForm.dailyType"
        :items="dailyTypeOptions"
        item-title="label"
        item-value="value"
        label="类型"
        variant="outlined"
      />

      <v-select
        v-model="editorForm.weather"
        :items="weatherOptions"
        label="天气（可选）"
        variant="outlined"
        clearable
      />

      <v-select
        v-if="isReviewType"
        v-model="editorForm.kumaMovieId"
        :items="kumaMovieOptions"
        item-title="label"
        item-value="value"
        label="影评电影（从 Kuma 选择）"
        :loading="kumaMoviesLoading"
        variant="outlined"
      />
    </div>

    <v-card
      v-if="isReviewType && selectedKumaMovie"
      class="selected-movie-card"
      rounded="lg"
      variant="tonal"
    >
      <div class="selected-movie-inner">
        <v-img
          v-if="selectedKumaMovie.cover"
          :src="selectedKumaMovie.cover"
          class="selected-movie-cover"
          cover
          height="96"
          width="72"
        />
        <div v-else class="selected-movie-cover selected-movie-cover--empty">
          <v-icon icon="mdi-image-off-outline" size="24" />
        </div>
        <div class="selected-movie-meta">
          <div class="selected-movie-title">
            {{ selectedKumaMovie.title }}
            <span v-if="selectedKumaMovie.years">({{ selectedKumaMovie.years }})</span>
          </div>
          <div class="selected-movie-sub">
            {{ selectedKumaMovie.provider.toUpperCase() }} #{{ selectedKumaMovie.movie_id }}
          </div>
        </div>
      </div>
    </v-card>

    <div class="split-panel" ref="splitPanelRef">
      <section
        class="panel panel-left panel-left-markdown"
        :style="{ width: `${leftPaneWidth}%` }"
        @dragenter.prevent="onDragEnter"
        @dragover.prevent="onDragOver"
        @dragleave.prevent="onDragLeave"
        @drop.prevent="onDropImage"
      >
        <header class="panel-head panel-head-main">
          <span>Markdown</span>
          <div class="panel-tools">
            <ImageUploadHintCard
              class="daily-upload-card daily-upload-card--picker"
              :disabled="uploadingImage"
              icon="mdi-folder-image"
              mode="action"
              title="从媒体库选择"
              hint="选择媒体库内已上传图片"
              @activate="openMediaLibraryPicker"
            />
            <ImageUploadHintCard
              class="daily-upload-card"
              :loading="uploadingImage"
              title="上传并插入图片"
              hint="拖到卡片或点击选择"
              @select-files="handleUploadCardFiles"
            />
          </div>
        </header>
        <textarea
          ref="markdownInputRef"
          v-model="editorForm.content"
          @paste="onPasteImage"
          class="markdown-input"
          placeholder="在这里输入 Markdown 内容..."
          spellcheck="false"
        />
        <div v-if="dragOver" class="drop-overlay">松开鼠标上传图片并插入 Markdown</div>
      </section>

      <div
        class="splitter"
        @pointerdown="startResize"
        @pointermove="moveResize"
        @pointerup="stopResize"
        @pointercancel="stopResize"
      >
        <div class="splitter-handle" />
      </div>

      <section class="panel panel-right" :style="{ width: `${100 - leftPaneWidth}%` }">
        <header class="panel-head">预览</header>
        <article class="markdown-preview" v-html="renderedMarkdown" />
      </section>
    </div>
    <MediaLibraryImagePicker
      v-model="mediaPickerVisible"
      @select-image="handleMediaLibrarySelect"
    />
    <UnsavedChangesLeaveDialog
      v-model="unsavedLeaveDialogVisible"
      @cancel="cancelUnsavedLeave"
      @confirm="confirmUnsavedLeave"
    />
  </section>
</template>

<script lang="ts" setup>
import MarkdownIt from 'markdown-it'
import { computed, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import UnsavedChangesLeaveDialog from '@/components/common/UnsavedChangesLeaveDialog.vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import { useUnsavedChangesGuard } from '@/composables/useUnsavedChangesGuard'
import {
  createDaily,
  fetchDailyById,
  updateDaily,
  type DailyUpsertPayload,
} from '@/services/dailies'
import {
  fetchAdminKumaMovies,
  type KumaMovieCard,
} from '@/services/kuma'
import { fetchDailyClassOptions, type ArticleClassOption } from '@/services/settings'
import { uploadMarkdownImage } from '@/services/storage'
import ImageUploadHintCard from '@/components/admin/ImageUploadHintCard.vue'
import MediaLibraryImagePicker from '@/components/admin/MediaLibraryImagePicker.vue'

const props = defineProps<{
  dailyId?: number | null
}>()

const router = useRouter()
const markdown = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
  typographer: true,
})

type EditorForm = {
  title: string
  dailyType: string
  kumaMovieId: number | null
  weather: string
  content: string
}

type SelectedMediaImage = {
  url: string
  fileName: string
}

type EditorSnapshot = {
  title: string
  dailyType: string
  kumaMovieId: number | null
  weather: string
  content: string
}

const loading = ref(false)
const submitting = ref(false)
const uploadingImage = ref(false)
const kumaMoviesLoading = ref(false)
const mediaPickerVisible = ref(false)
const errorMessage = ref('')
const successMessage = ref('')
const leftPaneWidth = ref(50)
const resizing = ref(false)
const dragOver = ref(false)
const dragDepth = ref(0)
const splitPanelRef = ref<HTMLElement | null>(null)
const markdownInputRef = ref<HTMLTextAreaElement | null>(null)
const savedSnapshot = ref('')
const {
  showGlobalSuccess,
  showGlobalError,
  showGlobalProgress,
  updateGlobalProgress,
  hideGlobalSnackbar,
} = useGlobalSnackbar()

const editorForm = reactive<EditorForm>({
  title: '',
  dailyType: 'note',
  kumaMovieId: null,
  weather: '',
  content: '',
})

const isEditing = computed(() => Number.isFinite(props.dailyId))
const isReviewType = computed(() => editorForm.dailyType === 'review')
const weatherOptions = ['cloud', 'rain', 'snow', 'sun', 'wind']
const weatherOptionSet = new Set<string>(weatherOptions)
const DEFAULT_DAILY_TYPE_OPTIONS: ArticleClassOption[] = [
  { label: '日常', value: 'note' },
  { label: '影评', value: 'review' },
]
const dailyTypeOptions = ref<ArticleClassOption[]>(DEFAULT_DAILY_TYPE_OPTIONS.map((item) => ({ ...item })))
const kumaMovieItems = ref<KumaMovieCard[]>([])
const kumaMovieOptions = computed(() => {
  return kumaMovieItems.value.map((item) => ({
    label: `${item.title}${item.years ? ` (${item.years})` : ''} · ${item.provider.toUpperCase()} #${item.movie_id}`,
    value: item.id,
  }))
})
const selectedKumaMovie = computed(() => {
  const targetId = editorForm.kumaMovieId
  if (typeof targetId !== 'number') {
    return null
  }
  return kumaMovieItems.value.find((item) => item.id === targetId) || null
})

const renderedMarkdown = computed(() => {
  const content = editorForm.content.trim()
  if (!content) {
    return '<p class="preview-empty">暂无内容，左侧输入 Markdown 开始编辑。</p>'
  }
  return markdown.render(content)
})

function buildEditorSnapshot(): EditorSnapshot {
  return {
    title: editorForm.title.trim(),
    dailyType: editorForm.dailyType,
    kumaMovieId: editorForm.kumaMovieId,
    weather: editorForm.weather.trim().toLowerCase(),
    content: editorForm.content,
  }
}

function serializeSnapshot(snapshot: EditorSnapshot): string {
  return JSON.stringify(snapshot)
}

function syncSavedSnapshot(): void {
  savedSnapshot.value = serializeSnapshot(buildEditorSnapshot())
}

const hasUnsavedChanges = computed(() => serializeSnapshot(buildEditorSnapshot()) !== savedSnapshot.value)
const {
  unsavedLeaveDialogVisible,
  confirmUnsavedLeave,
  cancelUnsavedLeave,
} = useUnsavedChangesGuard(hasUnsavedChanges)

function clampPercent(value: number): number {
  return Math.min(75, Math.max(25, value))
}

function updatePaneWidth(clientX: number): void {
  const panel = splitPanelRef.value
  if (!panel) {
    return
  }

  const rect = panel.getBoundingClientRect()
  if (rect.width <= 0) {
    return
  }

  const ratio = ((clientX - rect.left) / rect.width) * 100
  leftPaneWidth.value = clampPercent(ratio)
}

function startResize(event: PointerEvent): void {
  if (event.button !== 0) {
    return
  }

  const currentTarget = event.currentTarget as HTMLElement | null
  if (!currentTarget) {
    return
  }

  currentTarget.setPointerCapture(event.pointerId)
  resizing.value = true
  updatePaneWidth(event.clientX)
  event.preventDefault()
}

function moveResize(event: PointerEvent): void {
  if (!resizing.value) {
    return
  }
  updatePaneWidth(event.clientX)
}

function stopResize(): void {
  resizing.value = false
}

function buildPayload(): DailyUpsertPayload | null {
  const title = editorForm.title.trim()
  const dailyType = editorForm.dailyType.trim()
  if (!title) {
    errorMessage.value = '日常标题不能为空'
    showGlobalError('日常标题不能为空')
    return null
  }
  if (!dailyType) {
    errorMessage.value = '请选择日常分类'
    showGlobalError('请选择日常分类')
    return null
  }
  if (!dailyTypeOptions.value.some((item) => item.value === dailyType)) {
    errorMessage.value = '日常分类无效，请刷新后重试'
    showGlobalError('日常分类无效，请刷新后重试')
    return null
  }
  if (dailyType === 'review' && typeof editorForm.kumaMovieId !== 'number') {
    errorMessage.value = '影评类型必须选择电影'
    showGlobalError('影评类型必须选择电影')
    return null
  }

  const weather = editorForm.weather.trim().toLowerCase()
  return {
    title,
    daily_type: dailyType,
    kuma_movie_id: dailyType === 'review' ? editorForm.kumaMovieId : null,
    weather: weatherOptionSet.has(weather) ? weather : null,
    content: editorForm.content.trim() || null,
  }
}

function _escapeMarkdownText(value: string): string {
  return value.replace(/[\[\]\(\)]/g, '')
}

function _pickFirstImage(files: FileList | null): File | null {
  if (!files || files.length <= 0) {
    return null
  }
  for (const file of Array.from(files)) {
    if (file.type.startsWith('image/')) {
      return file
    }
  }
  return null
}

function _pickClipboardImage(event: ClipboardEvent): File | null {
  const items = event.clipboardData?.items
  if (!items) {
    return null
  }
  for (const item of Array.from(items)) {
    if (item.kind !== 'file' || !item.type.startsWith('image/')) {
      continue
    }
    return item.getAsFile()
  }
  return null
}

function _insertMarkdownImage(url: string, fileName: string): void {
  const altText = _escapeMarkdownText(fileName.replace(/\.[^.]+$/, '').trim()) || 'image'
  const snippet = `\n![${altText}](${url})\n`

  const input = markdownInputRef.value
  if (!input) {
    editorForm.content = `${editorForm.content}${snippet}`
    return
  }

  const start = input.selectionStart ?? editorForm.content.length
  const end = input.selectionEnd ?? start
  editorForm.content = `${editorForm.content.slice(0, start)}${snippet}${editorForm.content.slice(end)}`

  requestAnimationFrame(() => {
    const cursor = start + snippet.length
    input.focus()
    input.setSelectionRange(cursor, cursor)
  })
}

async function _uploadImageAndInsert(file: File): Promise<void> {
  if (uploadingImage.value) {
    return
  }

  uploadingImage.value = true
  errorMessage.value = ''
  try {
    showGlobalProgress('图片上传中 0%', 0)
    const imageUrl = await uploadMarkdownImage(file, {
      onProgress: ({ percent }) => {
        updateGlobalProgress(`图片上传中 ${percent}%`, percent)
      },
    })
    _insertMarkdownImage(imageUrl, file.name)
    showGlobalSuccess('图片上传成功')
  } catch (error) {
    hideGlobalSnackbar()
    const message = error instanceof Error ? error.message : '图片上传失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    uploadingImage.value = false
  }
}

async function handleUploadCardFiles(files: File[]): Promise<void> {
  const imageFile = files[0] || null
  if (!imageFile) {
    return
  }
  await _uploadImageAndInsert(imageFile)
}

function openMediaLibraryPicker(): void {
  if (uploadingImage.value) {
    return
  }
  mediaPickerVisible.value = true
}

function handleMediaLibrarySelect(payload: SelectedMediaImage): void {
  const imageUrl = payload.url.trim()
  if (!imageUrl) {
    return
  }
  const fileName = payload.fileName.trim() || 'image'
  _insertMarkdownImage(imageUrl, fileName)
}

function onDragEnter(): void {
  dragDepth.value += 1
  dragOver.value = true
}

function onDragOver(event: DragEvent): void {
  const imageFile = _pickFirstImage(event.dataTransfer?.files || null)
  if (imageFile) {
    event.dataTransfer!.dropEffect = 'copy'
    dragOver.value = true
  }
}

function onDragLeave(): void {
  dragDepth.value = Math.max(0, dragDepth.value - 1)
  if (dragDepth.value === 0) {
    dragOver.value = false
  }
}

async function onDropImage(event: DragEvent): Promise<void> {
  dragDepth.value = 0
  dragOver.value = false
  const imageFile = _pickFirstImage(event.dataTransfer?.files || null)
  if (!imageFile) {
    return
  }
  await _uploadImageAndInsert(imageFile)
}

async function onPasteImage(event: ClipboardEvent): Promise<void> {
  const imageFile = _pickClipboardImage(event)
  if (!imageFile) {
    return
  }
  event.preventDefault()
  await _uploadImageAndInsert(imageFile)
}

function fillEditorForm(daily: {
  title?: string | null
  daily_type?: string | null
  kuma_movie_id?: number | null
  weather?: string | null
  content?: string | null
}): void {
  const weather = daily.weather?.trim().toLowerCase() || ''
  const fallbackType = dailyTypeOptions.value[0]?.value || 'note'
  const dailyType = (daily.daily_type || '').trim() || fallbackType
  if (!dailyTypeOptions.value.some((item) => item.value === dailyType)) {
    dailyTypeOptions.value.push({ value: dailyType, label: dailyType })
  }
  editorForm.title = daily.title?.trim() || ''
  editorForm.dailyType = dailyType
  editorForm.kumaMovieId = dailyType === 'review' && typeof daily.kuma_movie_id === 'number'
    ? daily.kuma_movie_id
    : null
  editorForm.weather = weatherOptionSet.has(weather) ? weather : ''
  editorForm.content = daily.content || ''
}

async function loadDailyDetail(): Promise<void> {
  if (!isEditing.value || !props.dailyId) {
    return
  }

  loading.value = true
  errorMessage.value = ''
  try {
    const daily = await fetchDailyById(props.dailyId)
    fillEditorForm(daily)
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载日常详情失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    loading.value = false
  }
}

async function loadKumaMovies(): Promise<void> {
  kumaMoviesLoading.value = true
  try {
    kumaMovieItems.value = await fetchAdminKumaMovies()
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载 Kuma 电影列表失败'
    showGlobalError(message)
  } finally {
    kumaMoviesLoading.value = false
  }
}

async function loadDailyTypeOptions(): Promise<void> {
  try {
    const options = await fetchDailyClassOptions()
    dailyTypeOptions.value = options.length > 0
      ? options
      : DEFAULT_DAILY_TYPE_OPTIONS.map((item) => ({ ...item }))
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载日常分类失败'
    showGlobalError(message)
    dailyTypeOptions.value = DEFAULT_DAILY_TYPE_OPTIONS.map((item) => ({ ...item }))
  } finally {
    if (!dailyTypeOptions.value.some((item) => item.value === editorForm.dailyType)) {
      editorForm.dailyType = dailyTypeOptions.value[0]?.value || 'note'
    }
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
    if (isEditing.value && props.dailyId) {
      await updateDaily(props.dailyId, payload)
    } else {
      await createDaily(payload)
    }
    syncSavedSnapshot()
    showGlobalSuccess('日常发布成功')
    await router.push('/dailies')
  } catch (error) {
    const message = error instanceof Error ? error.message : '保存日常失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    submitting.value = false
  }
}

async function goManage(): Promise<void> {
  await router.push('/dailies')
}

onMounted(async () => {
  await loadDailyTypeOptions()
  await loadKumaMovies()
  await loadDailyDetail()
  syncSavedSnapshot()
})
</script>

<style scoped>
.daily-editor-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-height: calc(100vh - 64px);
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
  color: var(--admin-text-heading);
}

.header-text p {
  margin: 6px 0 0;
  color: var(--admin-text-muted);
}

.header-actions {
  display: flex;
  gap: 10px;
  flex-shrink: 0;
}

.meta-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.selected-movie-card {
  border: 1px solid var(--admin-accent-border);
  background: var(--admin-card-bg-strong);
}

.selected-movie-inner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
}

.selected-movie-cover {
  width: 72px;
  border-radius: 8px;
  overflow: hidden;
  flex-shrink: 0;
}

.selected-movie-cover--empty {
  height: 96px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--admin-accent-bg);
  color: var(--admin-accent-text);
}

.selected-movie-meta {
  min-width: 0;
}

.selected-movie-title {
  color: var(--admin-text-heading);
  font-size: 16px;
  font-weight: 700;
  line-height: 1.35;
}

.selected-movie-sub {
  margin-top: 6px;
  color: var(--admin-accent-muted);
  font-size: 13px;
}

.split-panel {
  min-height: 560px;
  height: calc(100vh - 280px);
  display: flex;
  border: 1px solid var(--admin-border);
  border-radius: 16px;
  overflow: hidden;
  background: var(--admin-surface);
}

.panel {
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.panel-head {
  padding: 10px 12px;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.4px;
  color: var(--admin-text-secondary);
  background: var(--admin-card-bg-soft);
  border-bottom: 1px solid var(--admin-border-soft);
}

.panel-head-main {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.panel-tools {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  gap: 8px;
}

.daily-upload-card {
  width: 220px;
}

.daily-upload-card--picker {
  width: 220px;
}

.panel-left-markdown {
  position: relative;
}

.markdown-input {
  flex: 1;
  width: 100%;
  border: 0;
  background: var(--admin-surface-2);
  color: var(--admin-text-primary);
  font-size: 14px;
  line-height: 1.7;
  padding: 14px;
  resize: none;
  outline: none;
  font-family: 'Cascadia Code', 'Consolas', 'Monaco', monospace;
}

.drop-overlay {
  position: absolute;
  inset: 46px 10px 10px 10px;
  display: grid;
  place-items: center;
  border: 2px dashed var(--admin-accent-border-strong);
  border-radius: 12px;
  background: var(--admin-overlay-panel);
  color: var(--admin-accent-text);
  font-size: 14px;
  font-weight: 600;
  pointer-events: none;
}

.markdown-preview {
  flex: 1;
  overflow: auto;
  padding: 16px;
  color: var(--admin-text-secondary);
  line-height: 1.75;
}

.splitter {
  width: 12px;
  cursor: col-resize;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--admin-card-bg-strong);
  border-left: 1px solid var(--admin-border-soft);
  border-right: 1px solid var(--admin-border-soft);
  touch-action: none;
}

.splitter-handle {
  width: 4px;
  height: 48px;
  border-radius: 999px;
  background: var(--admin-accent-bg-strong);
}

:deep(.markdown-preview p) {
  margin: 0 0 10px;
}

:deep(.markdown-preview h1),
:deep(.markdown-preview h2),
:deep(.markdown-preview h3),
:deep(.markdown-preview h4) {
  color: var(--admin-text-heading);
  margin: 18px 0 10px;
  line-height: 1.35;
}

:deep(.markdown-preview a) {
  color: var(--admin-link);
}

:deep(.markdown-preview code) {
  padding: 1px 5px;
  border-radius: 6px;
  background: var(--admin-border-soft);
  font-size: 13px;
}

:deep(.markdown-preview pre) {
  overflow: auto;
  padding: 10px;
  border-radius: 10px;
  background: var(--admin-overlay-mask);
}

:deep(.markdown-preview pre code) {
  padding: 0;
  background: transparent;
}

:deep(.markdown-preview blockquote) {
  margin: 12px 0;
  padding: 8px 12px;
  border-left: 3px solid var(--admin-accent-border-strong);
  background: var(--admin-accent-bg-soft);
}

:deep(.markdown-preview ul),
:deep(.markdown-preview ol) {
  padding-left: 20px;
}

:deep(.preview-empty) {
  color: var(--admin-text-faint);
}

@media (max-width: 900px) {
  .daily-editor-page {
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

  .meta-grid {
    grid-template-columns: 1fr;
  }

  .split-panel {
    flex-direction: column;
    height: auto;
    min-height: 0;
  }

  .panel-left,
  .panel-right {
    width: 100% !important;
  }

  .splitter {
    width: 100%;
    height: 12px;
    cursor: row-resize;
  }

  .splitter-handle {
    width: 48px;
    height: 4px;
  }

  .markdown-input {
    min-height: 300px;
  }

  .markdown-preview {
    min-height: 300px;
  }
}
</style>
