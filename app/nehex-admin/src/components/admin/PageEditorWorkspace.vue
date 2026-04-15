<template>
  <section class="page-editor-page">
    <header class="editor-header">
      <div class="header-text">
        <h1>{{ isEditing ? '编辑独立页' : '新增独立页' }}</h1>
        <p>与文章编辑器一致：左侧 Markdown 编辑，右侧独立信息卡片。</p>
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
          {{ isEditing ? '保存修改' : '创建页面' }}
        </v-btn>
      </div>
    </header>

    <v-progress-linear
      v-if="loading"
      class="mb-4"
      color="primary"
      indeterminate
    />

    <div class="workspace-grid">
      <section
        class="editor-card"
        @dragenter.prevent="onDragEnter"
        @dragover.prevent="onDragOver"
        @dragleave.prevent="onDragLeave"
        @drop.prevent="onDropImage"
      >
        <header class="editor-card-head">
          <div class="editor-title-wrap">
            <span class="editor-card-title">Markdown</span>
            <span class="editor-card-subtitle">支持拖拽/粘贴上传图片与快捷格式插入</span>
          </div>

          <div class="editor-toolbar">
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-format-header-1" @click="insertHeading(1)">
              H1
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-format-header-2" @click="insertHeading(2)">
              H2
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-format-header-3" @click="insertHeading(3)">
              H3
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-format-bold" @click="insertBold">
              粗体
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-format-italic" @click="insertItalic">
              斜体
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-format-list-bulleted" @click="insertBulletList">
              列表
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-format-list-numbered" @click="insertOrderedList">
              有序
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-format-quote-open" @click="insertQuote">
              引用
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-format-strikethrough-variant" @click="insertStrikethrough">
              删除线
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-link-variant" @click="insertLink">
              链接
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-code-tags" @click="insertCodeBlock">
              代码块
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-table" @click="insertTable">
              表格
            </v-btn>
            <v-btn density="comfortable" size="small" variant="text" prepend-icon="mdi-minus" @click="insertHorizontalRule">
              分割线
            </v-btn>
            <div class="editor-media-actions">
              <ImageUploadHintCard
                class="editor-upload-card editor-upload-card--picker"
                :disabled="uploadingImage"
                icon="mdi-folder-image"
                mode="action"
                title="从媒体库选择"
                hint="选择媒体库内已上传图片"
                @activate="openMediaLibraryPicker"
              />
              <ImageUploadHintCard
                class="editor-upload-card"
                :loading="uploadingImage"
                title="上传并插入图片"
                hint="拖到卡片或点击选择图片"
                @select-files="handleUploadCardFiles"
              />
            </div>
          </div>
        </header>

        <div class="editor-surface" @paste="onPasteImage">
          <textarea
            v-if="!previewMode"
            ref="markdownInputRef"
            v-model="editorForm.content"
            class="markdown-input"
            placeholder="在这里输入 Markdown 内容..."
            spellcheck="false"
          />
          <article
            v-else
            class="markdown-preview"
            v-html="renderedMarkdown"
          />

          <div v-if="dragOver" class="drop-overlay">松开鼠标上传图片并插入 Markdown</div>

          <v-btn
            class="preview-toggle"
            color="primary"
            size="small"
            variant="elevated"
            :prepend-icon="previewMode ? 'mdi-pencil' : 'mdi-eye-outline'"
            @click="togglePreviewMode"
          >
            {{ previewMode ? '返回编辑' : '预览' }}
          </v-btn>
        </div>
      </section>

      <aside class="settings-card">
        <header class="settings-head">
          <h2>页面设置</h2>
          <p>基础信息与展示状态</p>
        </header>

        <div class="settings-form">
          <v-text-field
            v-model="editorForm.title"
            label="页面标题"
            variant="outlined"
          />

          <v-text-field
            v-model="editorForm.pageKey"
            label="页面路径（page_key）"
            placeholder="about"
            variant="outlined"
          />

          <v-text-field
            v-model="editorForm.coverImage"
            label="封面图片链接（可选）"
            variant="outlined"
          />

          <v-text-field
            v-model.number="editorForm.sort"
            label="排序（sort）"
            type="number"
            variant="outlined"
          />

          <v-select
            v-model.number="editorForm.status"
            :items="statusOptions"
            item-title="label"
            item-value="value"
            label="状态（status）"
            variant="outlined"
          />
        </div>
      </aside>
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
import { computed, nextTick, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import UnsavedChangesLeaveDialog from '@/components/common/UnsavedChangesLeaveDialog.vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import { useUnsavedChangesGuard } from '@/composables/useUnsavedChangesGuard'
import {
  createStandalonePage,
  fetchStandalonePageById,
  updateStandalonePage,
  type StandalonePageUpsertPayload,
} from '@/services/pages'
import { uploadMarkdownImage } from '@/services/storage'
import ImageUploadHintCard from '@/components/admin/ImageUploadHintCard.vue'
import MediaLibraryImagePicker from '@/components/admin/MediaLibraryImagePicker.vue'

const props = defineProps<{
  pageId?: number | null
}>()

const router = useRouter()
const markdown = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
  typographer: true,
})

type EditorForm = {
  pageKey: string
  title: string
  coverImage: string
  content: string
  sort: number
  status: number
}

type SelectionTransformResult = {
  text: string
  selectionStart?: number
  selectionEnd?: number
}

type SelectedMediaImage = {
  url: string
  fileName: string
}

type EditorSnapshot = {
  pageKey: string
  title: string
  coverImage: string
  content: string
  sort: number
  status: number
}

const statusOptions = [
  { label: '启用', value: 1 },
  { label: '禁用', value: 0 },
]

const loading = ref(false)
const submitting = ref(false)
const uploadingImage = ref(false)
const mediaPickerVisible = ref(false)
const errorMessage = ref('')
const previewMode = ref(false)
const dragOver = ref(false)
const dragDepth = ref(0)
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
  pageKey: '',
  title: '',
  coverImage: '',
  content: '',
  sort: 0,
  status: 1,
})

const isEditing = computed(() => Number.isFinite(props.pageId))

const renderedMarkdown = computed(() => {
  const content = editorForm.content.trim()
  if (!content) {
    return '<p class="preview-empty">暂无内容，输入 Markdown 后点击右下角预览。</p>'
  }
  return markdown.render(content)
})

function buildEditorSnapshot(): EditorSnapshot {
  return {
    pageKey: normalizePageKey(editorForm.pageKey),
    title: editorForm.title.trim(),
    coverImage: editorForm.coverImage.trim(),
    content: editorForm.content,
    sort: normalizeSort(editorForm.sort),
    status: normalizeStatus(editorForm.status),
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

function normalizeSort(value: number): number {
  if (!Number.isFinite(value)) {
    return 0
  }
  return Math.floor(value)
}

function normalizeStatus(value: number): number {
  return value > 0 ? 1 : 0
}

function normalizePageKey(value: string): string {
  return value.trim().replace(/^\/+|\/+$/g, '')
}

function ensureEditMode(callback: () => void): void {
  if (!previewMode.value) {
    callback()
    return
  }

  previewMode.value = false
  nextTick(() => {
    callback()
  })
}

function updateSelection(transform: (selectedText: string) => SelectionTransformResult): void {
  const content = editorForm.content
  const input = markdownInputRef.value

  const start = input?.selectionStart ?? content.length
  const end = input?.selectionEnd ?? start
  const selectedText = content.slice(start, end)
  const result = transform(selectedText)

  editorForm.content = `${content.slice(0, start)}${result.text}${content.slice(end)}`

  requestAnimationFrame(() => {
    const latestInput = markdownInputRef.value
    if (!latestInput) {
      return
    }

    latestInput.focus()
    const selectionStart = start + (result.selectionStart ?? result.text.length)
    const selectionEnd = start + (result.selectionEnd ?? selectionStart - start)
    latestInput.setSelectionRange(selectionStart, selectionEnd)
  })
}

function wrapSelection(before: string, after: string, placeholder: string): void {
  ensureEditMode(() => {
    updateSelection((selectedText) => {
      if (selectedText) {
        return { text: `${before}${selectedText}${after}` }
      }
      const text = `${before}${placeholder}${after}`
      return {
        text,
        selectionStart: before.length,
        selectionEnd: before.length + placeholder.length,
      }
    })
  })
}

function prefixSelectionLines(prefix: string, placeholder: string): void {
  ensureEditMode(() => {
    updateSelection((selectedText) => {
      if (!selectedText) {
        const text = `${prefix}${placeholder}`
        return {
          text,
          selectionStart: prefix.length,
          selectionEnd: prefix.length + placeholder.length,
        }
      }

      const text = selectedText
        .split('\n')
        .map((line) => `${prefix}${line}`)
        .join('\n')
      return { text }
    })
  })
}

function insertHeading(level: 1 | 2 | 3): void {
  const prefix = `${'#'.repeat(level)} `
  const placeholder = level === 1
    ? '一级标题'
    : level === 2
      ? '二级标题'
      : '三级标题'
  prefixSelectionLines(prefix, placeholder)
}

function insertBold(): void {
  wrapSelection('**', '**', '粗体文本')
}

function insertItalic(): void {
  wrapSelection('*', '*', '斜体文本')
}

function insertQuote(): void {
  prefixSelectionLines('> ', '引用内容')
}

function insertBulletList(): void {
  prefixSelectionLines('- ', '列表项')
}

function insertOrderedList(): void {
  prefixSelectionLines('1. ', '列表项')
}

function insertLink(): void {
  ensureEditMode(() => {
    updateSelection((selectedText) => {
      const linkText = selectedText || '链接文字'
      const text = `[${linkText}](https://example.com)`
      const urlStart = text.indexOf('https://example.com')
      return {
        text,
        selectionStart: urlStart,
        selectionEnd: urlStart + 'https://example.com'.length,
      }
    })
  })
}

function insertCodeBlock(): void {
  ensureEditMode(() => {
    updateSelection((selectedText) => {
      const codeText = selectedText || 'code'
      const text = `\n\`\`\`\n${codeText}\n\`\`\`\n`
      return {
        text,
        selectionStart: 5,
        selectionEnd: 5 + codeText.length,
      }
    })
  })
}

function insertStrikethrough(): void {
  wrapSelection('~~', '~~', '删除线文本')
}

function insertHorizontalRule(): void {
  ensureEditMode(() => {
    updateSelection(() => ({
      text: '\n---\n',
      selectionStart: 5,
      selectionEnd: 5,
    }))
  })
}

function insertTable(): void {
  ensureEditMode(() => {
    updateSelection(() => {
      const text = '\n| 列1 | 列2 |\n| --- | --- |\n| 内容1 | 内容2 |\n'
      const focusStart = text.indexOf('内容1')
      return {
        text,
        selectionStart: focusStart,
        selectionEnd: focusStart + '内容1'.length,
      }
    })
  })
}

function togglePreviewMode(): void {
  previewMode.value = !previewMode.value
}

function escapeMarkdownText(value: string): string {
  return value.replace(/[\[\]\(\)]/g, '')
}

function pickFirstImage(files: FileList | null): File | null {
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

function pickClipboardImage(event: ClipboardEvent): File | null {
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

function insertMarkdownImage(url: string, fileName: string): void {
  const altText = escapeMarkdownText(fileName.replace(/\.[^.]+$/, '').trim()) || 'image'
  const snippet = `\n![${altText}](${url})\n`

  ensureEditMode(() => {
    updateSelection(() => ({ text: snippet }))
  })
}

async function uploadImageAndInsert(file: File): Promise<void> {
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
    insertMarkdownImage(imageUrl, file.name)
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
  await uploadImageAndInsert(imageFile)
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
  insertMarkdownImage(imageUrl, fileName)
}

function onDragEnter(): void {
  dragDepth.value += 1
  dragOver.value = true
}

function onDragOver(event: DragEvent): void {
  const imageFile = pickFirstImage(event.dataTransfer?.files || null)
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
  const imageFile = pickFirstImage(event.dataTransfer?.files || null)
  if (!imageFile) {
    return
  }
  await uploadImageAndInsert(imageFile)
}

async function onPasteImage(event: ClipboardEvent): Promise<void> {
  const imageFile = pickClipboardImage(event)
  if (!imageFile) {
    return
  }
  event.preventDefault()
  await uploadImageAndInsert(imageFile)
}

function buildPayload(): StandalonePageUpsertPayload | null {
  const title = editorForm.title.trim()
  const pageKey = normalizePageKey(editorForm.pageKey)

  if (!title) {
    errorMessage.value = '页面标题不能为空'
    showGlobalError('页面标题不能为空')
    return null
  }
  if (!pageKey) {
    errorMessage.value = '页面路径不能为空'
    showGlobalError('页面路径不能为空')
    return null
  }

  return {
    page_key: pageKey,
    title,
    cover_image: editorForm.coverImage.trim() || null,
    content: editorForm.content.trim() || null,
    sort: normalizeSort(editorForm.sort),
    status: normalizeStatus(editorForm.status),
  }
}

function fillEditorForm(page: {
  page_key?: string | null
  title?: string | null
  cover_image?: string | null
  content?: string | null
  sort?: number | null
  status?: number | null
}): void {
  editorForm.pageKey = page.page_key?.trim() || ''
  editorForm.title = page.title?.trim() || ''
  editorForm.coverImage = page.cover_image?.trim() || ''
  editorForm.content = page.content || ''
  editorForm.sort = Number.isFinite(page.sort) ? Number(page.sort) : 0
  editorForm.status = Number(page.status) > 0 ? 1 : 0
}

async function loadPageDetail(): Promise<void> {
  if (!isEditing.value || !props.pageId) {
    return
  }

  loading.value = true
  errorMessage.value = ''
  try {
    const page = await fetchStandalonePageById(props.pageId)
    fillEditorForm(page)
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载页面详情失败'
    errorMessage.value = message
    showGlobalError(message)
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
    if (isEditing.value && props.pageId) {
      await updateStandalonePage(props.pageId, payload)
    } else {
      await createStandalonePage(payload)
    }
    syncSavedSnapshot()
    showGlobalSuccess('独立页发布成功')
    await router.push('/pages')
  } catch (error) {
    const message = error instanceof Error ? error.message : '保存页面失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    submitting.value = false
  }
}

async function goManage(): Promise<void> {
  await router.push('/pages')
}

onMounted(async () => {
  await loadPageDetail()
  syncSavedSnapshot()
})
</script>

<style scoped>
.page-editor-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
  height: calc(100vh - 64px);
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

.workspace-grid {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(280px, 340px);
  gap: 14px;
}

.editor-card {
  min-width: 0;
  min-height: 0;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 16px;
  background: #111826;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.editor-card-head {
  padding: 10px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.05);
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 10px;
}

.editor-title-wrap {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.editor-card-title {
  font-size: 13px;
  font-weight: 700;
  color: #dbe7ff;
  letter-spacing: 0.4px;
}

.editor-card-subtitle {
  color: #9fb0d4;
  font-size: 12px;
}

.editor-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}

.editor-media-actions {
  margin-left: auto;
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

.editor-upload-card {
  width: 220px;
}

.editor-upload-card--picker {
  width: 220px;
}

.editor-surface {
  position: relative;
  flex: 1;
  min-height: 0;
}

.markdown-input {
  width: 100%;
  height: 100%;
  border: 0;
  background: #0f1624;
  color: #f4f7ff;
  font-size: 14px;
  line-height: 1.7;
  padding: 14px;
  resize: none;
  outline: none;
  font-family: 'Cascadia Code', 'Consolas', 'Monaco', monospace;
}

.markdown-preview {
  height: 100%;
  overflow: auto;
  padding: 16px;
  color: #dde6fb;
  line-height: 1.75;
}

.drop-overlay {
  position: absolute;
  inset: 10px;
  display: grid;
  place-items: center;
  border: 2px dashed rgba(115, 164, 255, 0.88);
  border-radius: 12px;
  background: rgba(16, 24, 39, 0.82);
  color: #d7e6ff;
  font-size: 14px;
  font-weight: 600;
  pointer-events: none;
}

.preview-toggle {
  position: absolute;
  right: 12px;
  bottom: 12px;
}

.settings-card {
  min-width: 0;
  min-height: 0;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 16px;
  background: #111826;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.settings-head {
  padding: 14px 14px 10px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.settings-head h2 {
  margin: 0;
  font-size: 18px;
  color: #edf3ff;
}

.settings-head p {
  margin: 6px 0 0;
  color: #9fb0d4;
  font-size: 13px;
}

.settings-form {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

:deep(.markdown-preview p) {
  margin: 0 0 10px;
}

:deep(.markdown-preview h1),
:deep(.markdown-preview h2),
:deep(.markdown-preview h3),
:deep(.markdown-preview h4) {
  color: #ffffff;
  margin: 18px 0 10px;
  line-height: 1.35;
}

:deep(.markdown-preview a) {
  color: #8ab5ff;
}

:deep(.markdown-preview code) {
  padding: 1px 5px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.08);
  font-size: 13px;
}

:deep(.markdown-preview pre) {
  overflow: auto;
  padding: 10px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.34);
}

:deep(.markdown-preview pre code) {
  padding: 0;
  background: transparent;
}

:deep(.markdown-preview blockquote) {
  margin: 12px 0;
  padding: 8px 12px;
  border-left: 3px solid rgba(126, 163, 237, 0.85);
  background: rgba(126, 163, 237, 0.12);
}

:deep(.markdown-preview ul),
:deep(.markdown-preview ol) {
  padding-left: 20px;
}

:deep(.preview-empty) {
  color: #97a4bf;
}

@media (max-width: 1100px) {
  .page-editor-page {
    height: auto;
    overflow: visible;
  }

  .workspace-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 900px) {
  .page-editor-page {
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

  .editor-card-head {
    flex-direction: column;
  }
}
</style>
