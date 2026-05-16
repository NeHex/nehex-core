<template>
  <section class="project-editor-page">
    <header class="editor-header">
      <div class="header-text">
        <h1>{{ isEditing ? '编辑项目' : '新增项目' }}</h1>
        <p>维护项目信息和 Markdown 详情内容，右侧实时预览最终展示效果。</p>
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
          {{ isEditing ? '保存修改' : '创建项目' }}
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
        label="项目标题"
        variant="outlined"
      />

      <v-text-field
        v-model="editorForm.category"
        label="项目分类（可选）"
        variant="outlined"
      />

      <v-text-field
        v-model="editorForm.cover"
        label="封面链接（可选）"
        variant="outlined"
      />

      <v-text-field
        v-model="editorForm.techStack"
        label="技术栈（可选）"
        placeholder="Vue3, FastAPI, PostgreSQL"
        variant="outlined"
      />

      <v-text-field
        v-model="editorForm.projectUrl"
        label="项目链接（可选）"
        variant="outlined"
      />

      <v-text-field
        v-model="editorForm.githubUrl"
        label="GitHub 链接（可选）"
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

    <v-textarea
      v-model="editorForm.description"
      auto-grow
      label="项目简介（可选）"
      min-rows="2"
      variant="outlined"
    />

    <div class="split-panel" ref="splitPanelRef">
      <section class="panel panel-left" :style="{ width: `${leftPaneWidth}%` }">
        <header class="panel-head">Markdown</header>
        <textarea
          v-model="editorForm.content"
          class="markdown-input"
          placeholder="在这里输入项目详情 Markdown..."
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
    <UnsavedChangesLeaveDialog
      v-model="unsavedLeaveDialogVisible"
      @cancel="cancelUnsavedLeave"
      @confirm="confirmUnsavedLeave"
    />
  </section>
</template>

<script lang="ts" setup>
import MarkdownIt from 'markdown-it'
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import UnsavedChangesLeaveDialog from '@/components/common/UnsavedChangesLeaveDialog.vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import { useUnsavedChangesGuard } from '@/composables/useUnsavedChangesGuard'
import {
  createProject,
  fetchProjectById,
  updateProject,
  type ProjectUpsertPayload,
} from '@/services/projects'

const props = defineProps<{
  projectId?: number | null
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
  cover: string
  category: string
  description: string
  content: string
  techStack: string
  projectUrl: string
  githubUrl: string
  sort: number
  status: number
}

type EditorSnapshot = {
  title: string
  cover: string
  category: string
  description: string
  content: string
  techStack: string
  projectUrl: string
  githubUrl: string
  sort: number
  status: number
}

const statusOptions = [
  { label: '启用', value: 1 },
  { label: '禁用', value: 0 },
]
const { showGlobalSuccess, showGlobalError } = useGlobalSnackbar()

const loading = ref(false)
const submitting = ref(false)
const errorMessage = ref('')
const successMessage = ref('')
const leftPaneWidth = ref(50)
const resizing = ref(false)
const splitPanelRef = ref<HTMLElement | null>(null)
const savedSnapshot = ref('')

const editorForm = reactive<EditorForm>({
  title: '',
  cover: '',
  category: '',
  description: '',
  content: '',
  techStack: '',
  projectUrl: '',
  githubUrl: '',
  sort: 0,
  status: 1,
})

const isEditing = computed(() => Number.isFinite(props.projectId))

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
    cover: editorForm.cover.trim(),
    category: editorForm.category.trim(),
    description: editorForm.description.trim(),
    content: editorForm.content,
    techStack: editorForm.techStack.trim(),
    projectUrl: editorForm.projectUrl.trim(),
    githubUrl: editorForm.githubUrl.trim(),
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

function normalizeSort(value: number): number {
  if (!Number.isFinite(value)) {
    return 0
  }
  return Math.floor(value)
}

function normalizeStatus(value: number): number {
  return value > 0 ? 1 : 0
}

function buildPayload(): ProjectUpsertPayload | null {
  const title = editorForm.title.trim()
  if (!title) {
    errorMessage.value = '项目标题不能为空'
    showGlobalError('项目标题不能为空')
    return null
  }

  return {
    title,
    cover: editorForm.cover.trim() || null,
    category: editorForm.category.trim() || null,
    description: editorForm.description.trim() || null,
    content: editorForm.content.trim() || null,
    tech_stack: editorForm.techStack.trim() || null,
    project_url: editorForm.projectUrl.trim() || null,
    github_url: editorForm.githubUrl.trim() || null,
    sort: normalizeSort(editorForm.sort),
    status: normalizeStatus(editorForm.status),
  }
}

function fillEditorForm(project: {
  title?: string | null
  cover?: string | null
  category?: string | null
  description?: string | null
  content?: string | null
  tech_stack?: string | null
  project_url?: string | null
  github_url?: string | null
  sort?: number | null
  status?: number | null
}): void {
  editorForm.title = project.title?.trim() || ''
  editorForm.cover = project.cover?.trim() || ''
  editorForm.category = project.category?.trim() || ''
  editorForm.description = project.description?.trim() || ''
  editorForm.content = project.content || ''
  editorForm.techStack = project.tech_stack?.trim() || ''
  editorForm.projectUrl = project.project_url?.trim() || ''
  editorForm.githubUrl = project.github_url?.trim() || ''
  editorForm.sort = Number.isFinite(project.sort) ? Number(project.sort) : 0
  editorForm.status = Number(project.status) > 0 ? 1 : 0
}

async function loadProjectDetail(): Promise<void> {
  if (!isEditing.value || !props.projectId) {
    return
  }

  loading.value = true
  errorMessage.value = ''
  try {
    const project = await fetchProjectById(props.projectId)
    fillEditorForm(project)
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载项目详情失败'
    errorMessage.value = message
    showGlobalError(message)
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
    if (isEditing.value && props.projectId) {
      await updateProject(props.projectId, payload)
      syncSavedSnapshot()
      successMessage.value = '项目已保存'
    } else {
      const created = await createProject(payload)
      syncSavedSnapshot()
      successMessage.value = '项目已创建'
      await router.replace(`/projects/edit/${created.id}`)
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : '保存项目失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    submitting.value = false
  }
}

async function goManage(): Promise<void> {
  await router.push('/projects')
}

onMounted(async () => {
  await loadProjectDetail()
  syncSavedSnapshot()
})

watch(successMessage, (nextMessage) => {
  const text = nextMessage.trim()
  if (!text) {
    return
  }
  showGlobalSuccess(text)
})
</script>

<style scoped>
.project-editor-page {
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
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}

.split-panel {
  min-height: 560px;
  height: calc(100vh - 320px);
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

.markdown-preview {
  flex: 1;
  overflow: auto;
  padding: 16px;
  color: var(--admin-text-secondary);
  line-height: 1.75;
}

.splitter {
  width: 12px;
  flex-shrink: 0;
  cursor: col-resize;
  background: var(--admin-card-bg-soft);
  display: flex;
  align-items: center;
  justify-content: center;
  touch-action: none;
}

.splitter:hover {
  background: var(--admin-accent-bg);
}

.splitter-handle {
  width: 3px;
  height: 72px;
  border-radius: 999px;
  background: var(--admin-accent-bg-strong);
}

.markdown-preview :deep(h1),
.markdown-preview :deep(h2),
.markdown-preview :deep(h3) {
  color: var(--admin-text-heading);
  margin-top: 1.1em;
}

.markdown-preview :deep(p) {
  margin: 0.6em 0;
}

.markdown-preview :deep(code) {
  padding: 2px 6px;
  border-radius: 6px;
  background: var(--admin-border-soft);
  font-family: 'Cascadia Code', 'Consolas', 'Monaco', monospace;
}

.markdown-preview :deep(pre) {
  overflow: auto;
  padding: 12px;
  border-radius: 10px;
  background: var(--admin-surface-3);
  border: 1px solid var(--admin-border-soft);
}

.markdown-preview :deep(blockquote) {
  margin: 1em 0;
  padding: 10px 12px;
  border-left: 3px solid var(--admin-accent-border-strong);
  background: var(--admin-accent-bg-soft);
  color: var(--admin-text-secondary);
}

.markdown-preview :deep(a) {
  color: var(--admin-link);
}

.markdown-preview :deep(img) {
  max-width: 100%;
  border-radius: 10px;
}

@media (max-width: 1200px) {
  .meta-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 960px) {
  .editor-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .header-actions {
    width: 100%;
    justify-content: flex-end;
  }

  .meta-grid {
    grid-template-columns: 1fr;
  }

  .split-panel {
    min-height: 620px;
    height: auto;
    flex-direction: column;
  }

  .panel {
    width: 100% !important;
    min-height: 260px;
  }

  .splitter {
    width: 100%;
    height: 12px;
    cursor: row-resize;
  }

  .splitter-handle {
    width: 72px;
    height: 3px;
  }
}
</style>
