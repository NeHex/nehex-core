<template>
  <section class="article-editor-page">
    <header class="editor-header">
      <div class="header-text">
        <h1>{{ isEditing ? '编辑文章' : '新增文章' }}</h1>
        <p>编辑器默认全屏 Markdown 输入，右下角可切换预览。</p>
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
          color="secondary"
          prepend-icon="mdi-file-document-edit-outline"
          :loading="submitting"
          variant="tonal"
          @click="submitEditor(0, true)"
        >
          保存草稿
        </v-btn>
        <v-btn
          color="primary"
          prepend-icon="mdi-publish"
          :loading="submitting"
          @click="submitEditor(1)"
        >
          {{ isEditing ? '发布并返回' : '发布文章' }}
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
            <span class="editor-card-subtitle">支持拖拽上传图片与快捷格式插入</span>
          </div>

          <div class="editor-toolbar">
            <v-btn
              density="comfortable"
              size="small"
              variant="text"
              prepend-icon="mdi-format-header-1"
              @click="insertHeading(1)"
            >
              H1
            </v-btn>
            <v-btn
              density="comfortable"
              size="small"
              variant="text"
              prepend-icon="mdi-format-header-2"
              @click="insertHeading(2)"
            >
              H2
            </v-btn>
            <v-btn
              density="comfortable"
              size="small"
              variant="text"
              prepend-icon="mdi-format-bold"
              @click="insertBold"
            >
              粗体
            </v-btn>
            <v-btn
              density="comfortable"
              size="small"
              variant="text"
              prepend-icon="mdi-format-italic"
              @click="insertItalic"
            >
              斜体
            </v-btn>
            <v-btn
              density="comfortable"
              size="small"
              variant="text"
              prepend-icon="mdi-format-list-bulleted"
              @click="insertBulletList"
            >
              列表
            </v-btn>
            <v-btn
              density="comfortable"
              size="small"
              variant="text"
              prepend-icon="mdi-format-quote-open"
              @click="insertQuote"
            >
              引用
            </v-btn>
            <v-btn
              density="comfortable"
              size="small"
              variant="text"
              prepend-icon="mdi-link-variant"
              @click="insertLink"
            >
              链接
            </v-btn>
            <v-btn
              density="comfortable"
              size="small"
              variant="text"
              prepend-icon="mdi-code-tags"
              @click="insertCodeBlock"
            >
              代码块
            </v-btn>
            <div class="editor-media-actions">
              <v-btn
                class="editor-media-picker-btn"
                color="primary"
                density="comfortable"
                size="small"
                variant="tonal"
                prepend-icon="mdi-folder-image"
                :disabled="uploadingImage"
                @click="openMediaLibraryPicker"
              >
                从媒体库选择
              </v-btn>
              <ImageUploadHintCard
                class="editor-upload-card"
                :loading="uploadingImage"
                title="上传并插入图片"
                hint="拖动到卡片或点击选择图片"
                @select-files="handleUploadCardFiles"
              />
            </div>
          </div>
        </header>

        <div class="editor-surface">
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
          <h2>文章设置</h2>
          <p>文章标题、分类与展示选项</p>
        </header>

        <div class="settings-form">
          <v-text-field
            v-model="editorForm.title"
            label="文章标题"
            variant="outlined"
          />

          <v-select
            v-model="editorForm.className"
            :items="classOptions"
            item-title="label"
            item-value="value"
            label="文章分类"
            no-data-text="暂无可用分类"
            variant="outlined"
          />

          <v-text-field
            v-model="editorForm.tag"
            label="标签（可选）"
            variant="outlined"
          />

          <v-text-field
            v-model="editorForm.articleTopImage"
            label="封面图片链接（可选）"
            variant="outlined"
          />

          <v-text-field
            v-model.number="editorForm.top"
            label="置顶权重（top）"
            min="0"
            type="number"
            variant="outlined"
          />

          <v-text-field
            v-model.number="editorForm.read"
            label="阅读数（read）"
            min="0"
            type="number"
            variant="outlined"
          />

          <v-select
            v-model.number="editorForm.status"
            :items="statusOptions"
            item-title="label"
            item-value="value"
            label="发布状态（status）"
            variant="outlined"
          />
        </div>
      </aside>
    </div>
    <MediaLibraryImagePicker
      v-model="mediaPickerVisible"
      @select-image="handleMediaLibrarySelect"
    />
  </section>
</template>

<script lang="ts" setup>
import MarkdownIt from 'markdown-it'
import { computed, nextTick, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  createArticle,
  fetchArticleById,
  updateArticle,
  type ArticleUpsertPayload,
} from '@/services/articles'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import { fetchArticleClassOptions, type ArticleClassOption } from '@/services/settings'
import { uploadMarkdownImage } from '@/services/storage'
import ImageUploadHintCard from '@/components/admin/ImageUploadHintCard.vue'
import MediaLibraryImagePicker from '@/components/admin/MediaLibraryImagePicker.vue'

const props = defineProps<{
  articleId?: number | null
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
  className: string
  tag: string
  articleTopImage: string
  top: number
  read: number
  status: number
  content: string
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

const DEFAULT_CLASS_OPTIONS: ArticleClassOption[] = [
  {
    value: 'default',
    label: '默认分类',
  },
]
const statusOptions = [
  { label: '草稿（0）', value: 0 },
  { label: '已发布（1）', value: 1 },
]

const loading = ref(false)
const submitting = ref(false)
const uploadingImage = ref(false)
const mediaPickerVisible = ref(false)
const errorMessage = ref('')
const successMessage = ref('')
const previewMode = ref(false)
const dragOver = ref(false)
const dragDepth = ref(0)
const markdownInputRef = ref<HTMLTextAreaElement | null>(null)

const classOptions = ref<ArticleClassOption[]>(DEFAULT_CLASS_OPTIONS)
const {
  showGlobalSuccess,
  showGlobalError,
  showGlobalProgress,
  updateGlobalProgress,
  hideGlobalSnackbar,
} = useGlobalSnackbar()

const editorForm = reactive<EditorForm>({
  title: '',
  className: 'default',
  tag: '',
  articleTopImage: '',
  top: 0,
  read: 0,
  status: 1,
  content: '',
})

const isEditing = computed(() => Number.isFinite(props.articleId))

const renderedMarkdown = computed(() => {
  const content = editorForm.content.trim()
  if (!content) {
    return '<p class="preview-empty">暂无内容，输入 Markdown 后点击右下角预览。</p>'
  }
  return markdown.render(content)
})

function ensureClassOption(value: string): void {
  const normalized = value.trim()
  if (!normalized) {
    return
  }

  const exists = classOptions.value.some((item) => item.value === normalized)
  if (!exists) {
    classOptions.value = [
      ...classOptions.value,
      {
        value: normalized,
        label: normalized,
      },
    ]
  }
}

function normalizeNumber(value: number): number {
  if (!Number.isFinite(value)) {
    return 0
  }
  return Math.max(0, Math.floor(value))
}

function normalizeStatus(value: number): 0 | 1 {
  return Number(value) > 0 ? 1 : 0
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
        const text = `${before}${selectedText}${after}`
        return { text }
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

function insertHeading(level: 1 | 2): void {
  const prefix = `${'#'.repeat(level)} `
  const placeholder = level === 1 ? '一级标题' : '二级标题'
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
      const selectionStart = 5
      return {
        text,
        selectionStart,
        selectionEnd: selectionStart + codeText.length,
      }
    })
  })
}

function togglePreviewMode(): void {
  previewMode.value = !previewMode.value
}

function _insertMarkdownImage(url: string, fileName: string): void {
  const altText = _escapeMarkdownText(fileName.replace(/\.[^.]+$/, '').trim()) || 'image'
  const snippet = `\n![${altText}](${url})\n`

  ensureEditMode(() => {
    updateSelection(() => ({ text: snippet }))
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

function buildPayload(statusOverride?: 0 | 1): ArticleUpsertPayload | null {
  const status = normalizeStatus(statusOverride ?? editorForm.status)
  let title = editorForm.title.trim()
  const className = editorForm.className.trim()

  if (!title) {
    if (status === 0) {
      title = '未命名草稿'
    } else {
      errorMessage.value = '文章标题不能为空'
      showGlobalError('文章标题不能为空')
      return null
    }
  }

  if (!className) {
    errorMessage.value = '文章分类不能为空'
    showGlobalError('文章分类不能为空')
    return null
  }

  return {
    title,
    class: className,
    tag: editorForm.tag.trim() || null,
    articleTopImage: editorForm.articleTopImage.trim() || null,
    top: normalizeNumber(editorForm.top),
    read: normalizeNumber(editorForm.read),
    status,
    content: editorForm.content.trim() || null,
  }
}

function fillEditorForm(article: {
  title?: string | null
  class?: string | null
  tag?: string | null
  articleTopImage?: string | null
  top?: number | null
  read?: number | null
  status?: number | null
  content?: string | null
}): void {
  editorForm.title = article.title?.trim() || ''
  const className = article.class?.trim() || classOptions.value[0]?.value || 'default'
  ensureClassOption(className)
  editorForm.className = className
  editorForm.tag = article.tag?.trim() || ''
  editorForm.articleTopImage = article.articleTopImage?.trim() || ''
  editorForm.top = Number.isFinite(article.top) ? Number(article.top) : 0
  editorForm.read = Number.isFinite(article.read) ? Number(article.read) : 0
  const rawStatus = Number(article.status)
  editorForm.status = Number.isFinite(rawStatus) ? normalizeStatus(rawStatus) : 1
  editorForm.content = article.content || ''
}

async function loadClassOptions(): Promise<void> {
  try {
    const options = await fetchArticleClassOptions()
    classOptions.value = options.length > 0 ? options : DEFAULT_CLASS_OPTIONS
  } catch {
    classOptions.value = DEFAULT_CLASS_OPTIONS
  }

  if (!classOptions.value.some((item) => item.value === editorForm.className)) {
    editorForm.className = classOptions.value[0]?.value || 'default'
  }
}

async function loadArticleDetail(): Promise<void> {
  if (!isEditing.value || !props.articleId) {
    return
  }

  loading.value = true
  errorMessage.value = ''
  try {
    const article = await fetchArticleById(props.articleId)
    fillEditorForm(article)
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载文章详情失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    loading.value = false
  }
}

async function submitEditor(nextStatus?: 0 | 1, stayOnPage = false): Promise<void> {
  successMessage.value = ''
  errorMessage.value = ''

  const payload = buildPayload(nextStatus)
  if (!payload) {
    return
  }

  submitting.value = true

  try {
    const saved = isEditing.value && props.articleId
      ? await updateArticle(props.articleId, payload)
      : await createArticle(payload)

    fillEditorForm(saved)

    const savedAsDraft = normalizeStatus(payload.status) === 0
    if (isEditing.value && props.articleId) {
      showGlobalSuccess(savedAsDraft ? '草稿已保存' : '文章已发布')
      if (!stayOnPage) {
        await router.push('/articles')
      }
      return
    }

    if (savedAsDraft && stayOnPage) {
      showGlobalSuccess('草稿已创建')
      await router.replace(`/articles/edit/${saved.id}`)
      return
    }

    showGlobalSuccess(savedAsDraft ? '草稿已保存' : '文章发布成功')
    if (!stayOnPage) {
      await router.push('/articles')
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : '保存文章失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    submitting.value = false
  }
}

async function goManage(): Promise<void> {
  await router.push('/articles')
}

onMounted(async () => {
  await loadClassOptions()
  await loadArticleDetail()
})
</script>

<style scoped>
.article-editor-page {
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
  display: grid;
  grid-template-columns: minmax(0, 1fr) 320px;
  gap: 14px;
  align-items: stretch;
  flex: 1;
  min-height: 0;
}

.editor-card,
.settings-card {
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 16px;
  background: #111826;
  min-height: 0;
}

.editor-card {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.editor-card-head {
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
}

.editor-title-wrap {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
}

.editor-card-title {
  font-size: 14px;
  font-weight: 700;
  color: #dbe7ff;
}

.editor-card-subtitle {
  font-size: 12px;
  color: #9eb0ce;
}

.editor-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.editor-media-actions {
  margin-left: auto;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

.editor-media-picker-btn {
  white-space: nowrap;
}

.editor-upload-card {
  width: min(320px, 100%);
}

.editor-surface {
  position: relative;
  flex: 1;
  min-height: 0;
}

.markdown-input,
.markdown-preview {
  width: 100%;
  height: 100%;
  min-height: 0;
  border: 0;
  outline: none;
}

.markdown-input {
  resize: none;
  background: #0f1624;
  color: #f4f7ff;
  font-size: 14px;
  line-height: 1.7;
  padding: 14px;
  font-family: 'Cascadia Code', 'Consolas', 'Monaco', monospace;
}

.markdown-preview {
  overflow: auto;
  padding: 16px;
  color: #dde6fb;
  line-height: 1.75;
}

.preview-toggle {
  position: absolute;
  right: 12px;
  bottom: 12px;
  z-index: 3;
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
  z-index: 2;
}

.settings-card {
  padding: 14px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.settings-head h2 {
  margin: 0;
  font-size: 18px;
  color: #e8efff;
}

.settings-head p {
  margin: 6px 0 0;
  color: #9eb0ce;
  font-size: 13px;
}

.settings-form {
  margin-top: 12px;
  display: grid;
  grid-template-columns: 1fr;
  gap: 8px;
  min-height: 0;
  overflow: auto;
  padding-right: 2px;
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
  .workspace-grid {
    grid-template-columns: minmax(0, 1fr) 280px;
  }

  .editor-title-wrap {
    flex-direction: column;
    align-items: flex-start;
  }
}

@media (max-width: 900px) {
  .article-editor-page {
    height: auto;
    min-height: auto;
    overflow: visible;
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

  .workspace-grid {
    grid-template-columns: 1fr;
  }

  .editor-card {
    height: auto;
  }

  .markdown-input,
  .markdown-preview {
    min-height: 360px;
  }
}
</style>
