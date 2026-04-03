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

    <div class="meta-grid">
      <v-text-field
        v-model="editorForm.title"
        label="日常标题"
        variant="outlined"
      />

      <v-text-field
        v-model="editorForm.weather"
        label="天气（可选）"
        variant="outlined"
      />
    </div>

    <div class="split-panel" ref="splitPanelRef">
      <section class="panel panel-left" :style="{ width: `${leftPaneWidth}%` }">
        <header class="panel-head">Markdown</header>
        <textarea
          v-model="editorForm.content"
          class="markdown-input"
          placeholder="在这里输入 Markdown 内容..."
          spellcheck="false"
        />
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
  </section>
</template>

<script lang="ts" setup>
import MarkdownIt from 'markdown-it'
import { computed, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  createDaily,
  fetchDailyById,
  updateDaily,
  type DailyUpsertPayload,
} from '@/services/dailies'

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
  weather: string
  content: string
}

const loading = ref(false)
const submitting = ref(false)
const errorMessage = ref('')
const successMessage = ref('')
const leftPaneWidth = ref(50)
const resizing = ref(false)
const splitPanelRef = ref<HTMLElement | null>(null)

const editorForm = reactive<EditorForm>({
  title: '',
  weather: '',
  content: '',
})

const isEditing = computed(() => Number.isFinite(props.dailyId))

const renderedMarkdown = computed(() => {
  const content = editorForm.content.trim()
  if (!content) {
    return '<p class="preview-empty">暂无内容，左侧输入 Markdown 开始编辑。</p>'
  }
  return markdown.render(content)
})

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
  if (!title) {
    errorMessage.value = '日常标题不能为空'
    return null
  }

  return {
    title,
    weather: editorForm.weather.trim() || null,
    content: editorForm.content.trim() || null,
  }
}

function fillEditorForm(daily: {
  title?: string | null
  weather?: string | null
  content?: string | null
}): void {
  editorForm.title = daily.title?.trim() || ''
  editorForm.weather = daily.weather?.trim() || ''
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
    errorMessage.value = error instanceof Error ? error.message : '加载日常详情失败'
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
    if (isEditing.value && props.dailyId) {
      await updateDaily(props.dailyId, payload)
      successMessage.value = '日常已保存'
    } else {
      const created = await createDaily(payload)
      successMessage.value = '日常已创建'
      await router.replace(`/dailies/edit/${created.id}`)
    }
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '保存日常失败'
  } finally {
    submitting.value = false
  }
}

async function goManage(): Promise<void> {
  await router.push('/dailies')
}

onMounted(async () => {
  await loadDailyDetail()
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

.meta-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.split-panel {
  min-height: 560px;
  height: calc(100vh - 280px);
  display: flex;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 16px;
  overflow: hidden;
  background: #111826;
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
  color: #dbe7ff;
  background: rgba(255, 255, 255, 0.05);
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.markdown-input {
  flex: 1;
  width: 100%;
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
  flex: 1;
  overflow: auto;
  padding: 16px;
  color: #dde6fb;
  line-height: 1.75;
}

.splitter {
  width: 12px;
  cursor: col-resize;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(180deg, rgba(38, 48, 71, 0.9), rgba(25, 34, 51, 0.9));
  border-left: 1px solid rgba(255, 255, 255, 0.08);
  border-right: 1px solid rgba(255, 255, 255, 0.08);
  touch-action: none;
}

.splitter-handle {
  width: 4px;
  height: 48px;
  border-radius: 999px;
  background: rgba(205, 218, 255, 0.6);
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
