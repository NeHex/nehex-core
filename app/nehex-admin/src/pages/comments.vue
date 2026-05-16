<template>
  <AdminLayout>
    <section class="comments-page">
      <header class="page-header">
        <div class="header-text">
          <h1>评论管理</h1>
          <p>在这里管理评论</p>
        </div>
      </header>

      <div class="search-row">
        <v-text-field
          v-model="searchKeyword"
          class="search-input"
          clearable
          density="comfortable"
          hide-details
          label="搜索评论"
          prepend-inner-icon="mdi-magnify"
          variant="outlined"
          @keydown.enter.prevent="searchComments"
        />
        <v-btn
          color="primary"
          prepend-icon="mdi-magnify"
          :loading="loading"
          @click="searchComments"
        >
          搜索
        </v-btn>
      </div>

      <div class="list-head">
        <span>共 {{ totalComments }} 条评论</span>
      </div>

      <v-progress-linear
        v-if="loading"
        class="mb-2"
        color="primary"
        indeterminate
      />

      <div class="comments-list">
        <v-card
          v-for="comment in comments"
          :key="comment.id"
          class="comment-card"
          rounded="xl"
        >
          <div class="comment-card-shell">
            <div class="comment-head">
              <div class="meta-group">
                <v-chip size="small" variant="tonal">#{{ comment.id }}</v-chip>
                <v-chip size="small" variant="tonal">{{ mapTargetLabel(comment.target_type) }}:{{ comment.target_id }}</v-chip>
                <v-chip size="small" :color="comment.parent_id > 0 ? 'warning' : 'primary'" variant="tonal">
                  {{ comment.parent_id > 0 ? '回复' : '主评论' }}
                </v-chip>
                <v-chip size="small" :color="Number(comment.status) > 0 ? 'success' : 'default'" variant="tonal">
                  {{ Number(comment.status) > 0 ? '启用' : '禁用' }}
                </v-chip>
              </div>
              <span class="time">{{ formatDateTime(comment.create_time) }}</span>
            </div>

            <div class="comment-summary">
              <div class="summary-item">
                <span class="summary-label">昵称</span>
                <span class="summary-value">{{ comment.nickname || '未填写' }}</span>
              </div>
              <div class="summary-item">
                <span class="summary-label">邮箱</span>
                <span class="summary-value">{{ comment.email || '未填写' }}</span>
              </div>
              <div class="summary-item">
                <span class="summary-label">网址</span>
                <span class="summary-value">{{ comment.website || '未填写' }}</span>
              </div>
            </div>

            <p class="comment-preview">
              {{ comment.content || '（空内容）' }}
            </p>

            <div class="card-actions">
              <v-btn
                color="primary"
                prepend-icon="mdi-pencil-outline"
                size="small"
                variant="tonal"
                @click="openEditDialog(comment)"
              >
                编辑
              </v-btn>
              <v-btn
                prepend-icon="mdi-open-in-new"
                size="small"
                variant="text"
                :disabled="!canJumpToTarget(comment)"
                @click="goToTarget(comment)"
              >
                跳转
              </v-btn>
              <v-btn
                color="error"
                prepend-icon="mdi-delete-outline"
                size="small"
                variant="text"
                @click="openDeleteDialog(comment)"
              >
                删除
              </v-btn>
            </div>
          </div>
        </v-card>

        <v-card
          v-if="!loading && comments.length === 0"
          class="empty-card"
          rounded="lg"
        >
          暂无评论
        </v-card>
      </div>

      <div v-if="totalPages > 1" class="pagination-row">
        <v-pagination
          v-model="currentPage"
          :disabled="loading"
          :length="totalPages"
          density="comfortable"
          rounded="circle"
          :total-visible="7"
        />
      </div>
    </section>

    <v-dialog v-model="editDialog" max-width="760">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title class="dialog-title">
          编辑评论 #{{ editingCommentId || '' }}
        </v-card-title>
        <v-card-text>
          <div v-if="editingCommentId !== null" class="edit-panel">
            <div class="edit-grid">
              <v-text-field
                v-model="getEditForm(editingCommentId).nickname"
                density="comfortable"
                hide-details
                label="昵称"
                variant="outlined"
              />
              <v-text-field
                v-model="getEditForm(editingCommentId).email"
                density="comfortable"
                hide-details
                label="邮箱"
                variant="outlined"
              />
              <v-text-field
                v-model="getEditForm(editingCommentId).website"
                density="comfortable"
                hide-details
                label="网址"
                variant="outlined"
              />
              <v-select
                v-model.number="getEditForm(editingCommentId).status"
                :items="statusOptions"
                density="comfortable"
                hide-details
                item-title="label"
                item-value="value"
                label="状态"
                variant="outlined"
              />
            </div>
            <v-textarea
              v-model="getEditForm(editingCommentId).content"
              auto-grow
              class="content-editor"
              hide-details
              label="留言内容"
              min-rows="6"
              variant="outlined"
            />
          </div>
        </v-card-text>
        <v-card-actions class="dialog-actions">
          <v-spacer />
          <v-btn variant="text" @click="closeEditDialog">取消</v-btn>
          <v-btn
            color="primary"
            prepend-icon="mdi-content-save-outline"
            :loading="isEditingSaving()"
            @click="saveEditingComment"
          >
            保存
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="deleteDialog" max-width="420">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title class="dialog-title">确认删除</v-card-title>
        <v-card-text>
          即将删除评论 #{{ pendingDelete?.id || '' }}，删除后不可恢复。
        </v-card-text>
        <v-card-actions class="dialog-actions">
          <v-spacer />
          <v-btn variant="text" @click="closeDeleteDialog">取消</v-btn>
          <v-btn color="error" :loading="deleting" @click="confirmDelete">
            确认删除
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </AdminLayout>
</template>

<script lang="ts" setup>
import { onMounted, reactive, ref, watch } from 'vue'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import {
  deleteAdminComment,
  fetchAdminComments,
  updateAdminComment,
  type AdminCommentItem,
} from '@/services/comments'
import { fetchStandalonePageById } from '@/services/pages'
import { fetchSiteUrl } from '@/services/settings'
import {
  buildCommentTargetUrl,
  canJumpToCommentTarget,
  mapCommentTargetLabel,
} from '@/utils/commentTargets'

type EditCommentForm = {
  nickname: string
  email: string
  website: string
  content: string
  status: number
}

const { showGlobalSuccess, showGlobalError } = useGlobalSnackbar()

const loading = ref(false)
const deleting = ref(false)
const errorMessage = ref('')
const searchKeyword = ref('')
const comments = ref<AdminCommentItem[]>([])
const editForms = reactive<Record<number, EditCommentForm>>({})
const currentPage = ref(1)
const pageSize = 20
const totalComments = ref(0)
const totalPages = ref(0)
const deleteDialog = ref(false)
const pendingDelete = ref<AdminCommentItem | null>(null)
const savingIds = ref<Set<number>>(new Set())
const editDialog = ref(false)
const editingCommentId = ref<number | null>(null)
const siteUrl = ref('')
const singlePagePathCache = reactive<Record<number, string>>({})

const statusOptions = [
  { label: '启用', value: 1 },
  { label: '禁用', value: 0 },
]

function getEditForm(commentId: number): EditCommentForm {
  if (!editForms[commentId]) {
    editForms[commentId] = {
      nickname: '',
      email: '',
      website: '',
      content: '',
      status: 1,
    }
  }
  return editForms[commentId]!
}

function formatDateTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }
  return date.toLocaleString('zh-CN')
}

function mapTargetLabel(targetType: string): string {
  return mapCommentTargetLabel(targetType)
}

function canJumpToTarget(comment: AdminCommentItem): boolean {
  return canJumpToCommentTarget(comment)
}

async function resolveStandalonePagePath(pageId: number): Promise<string | null> {
  const normalizedId = Math.max(1, Math.floor(Number(pageId) || 0))
  const cachedPath = singlePagePathCache[normalizedId]
  if (cachedPath) {
    return cachedPath
  }

  try {
    const page = await fetchStandalonePageById(normalizedId)
    const pageKey = String(page.page_key ?? '').trim().replace(/^\/+|\/+$/g, '')
    if (pageKey) {
      const resolvedPath = `/${pageKey}`
      singlePagePathCache[normalizedId] = resolvedPath
      return resolvedPath
    }
  } catch (error) {
    console.warn('Failed to resolve standalone page path for comment target', error)
  }

  return null
}

async function goToTarget(comment: AdminCommentItem): Promise<void> {
  const targetUrl = await buildCommentTargetUrl(comment, siteUrl.value, resolveStandalonePagePath)
  if (!targetUrl) {
    showGlobalError('无法生成前台页面跳转地址')
    return
  }

  window.open(targetUrl, '_blank', 'noopener')
}

function normalizeOptional(value: string): string | null {
  const text = value.trim()
  return text || null
}

function syncEditForms(items: AdminCommentItem[]): void {
  const usedIds = new Set<number>()

  for (const item of items) {
    usedIds.add(item.id)
    editForms[item.id] = {
      nickname: item.nickname || '',
      email: item.email || '',
      website: item.website || '',
      content: item.content || '',
      status: Number(item.status) > 0 ? 1 : 0,
    }
  }

  for (const key of Object.keys(editForms)) {
    const id = Number(key)
    if (!usedIds.has(id)) {
      delete editForms[id]
    }
  }
}

function isSaving(commentId: number): boolean {
  return savingIds.value.has(commentId)
}

function setSaving(commentId: number, saving: boolean): void {
  const next = new Set(savingIds.value)
  if (saving) {
    next.add(commentId)
  } else {
    next.delete(commentId)
  }
  savingIds.value = next
}

async function loadComments(targetPage = currentPage.value): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  try {
    const result = await fetchAdminComments(searchKeyword.value, targetPage, pageSize)
    comments.value = result.items
    syncEditForms(result.items)
    totalComments.value = result.pagination.total
    totalPages.value = result.pagination.total_pages
    currentPage.value = result.pagination.page

    if (
      result.items.length === 0
      && result.pagination.total_pages > 0
      && targetPage > result.pagination.total_pages
    ) {
      currentPage.value = result.pagination.total_pages
      return
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载评论失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    loading.value = false
  }
}

async function searchComments(): Promise<void> {
  if (currentPage.value !== 1) {
    currentPage.value = 1
    return
  }
  await loadComments(1)
}

async function saveComment(commentId: number): Promise<boolean> {
  const form = getEditForm(commentId)

  const nickname = form.nickname.trim()
  const content = form.content.trim()
  if (!nickname) {
    errorMessage.value = '昵称不能为空'
    showGlobalError('昵称不能为空')
    return false
  }
  if (!content) {
    errorMessage.value = '留言内容不能为空'
    showGlobalError('留言内容不能为空')
    return false
  }

  setSaving(commentId, true)
  errorMessage.value = ''

  try {
    const updated = await updateAdminComment(commentId, {
      nickname,
      email: normalizeOptional(form.email),
      website: normalizeOptional(form.website),
      content,
      status: form.status > 0 ? 1 : 0,
    })

    const index = comments.value.findIndex((item) => item.id === commentId)
    if (index >= 0) {
      comments.value[index] = updated
    }
    syncEditForms(comments.value)
    showGlobalSuccess(`评论 #${commentId} 已保存`)
    return true
  } catch (error) {
    const message = error instanceof Error ? error.message : '保存评论失败'
    errorMessage.value = message
    showGlobalError(message)
    return false
  } finally {
    setSaving(commentId, false)
  }
}

function openEditDialog(comment: AdminCommentItem): void {
  editingCommentId.value = comment.id
  getEditForm(comment.id)
  editDialog.value = true
}

function closeEditDialog(force = false): void {
  if (
    editingCommentId.value !== null
    && isSaving(editingCommentId.value)
    && !force
  ) {
    return
  }
  editDialog.value = false
  editingCommentId.value = null
}

function isEditingSaving(): boolean {
  return editingCommentId.value !== null && isSaving(editingCommentId.value)
}

async function saveEditingComment(): Promise<void> {
  if (editingCommentId.value === null) {
    return
  }
  const ok = await saveComment(editingCommentId.value)
  if (ok) {
    closeEditDialog(true)
  }
}

function openDeleteDialog(comment: AdminCommentItem): void {
  pendingDelete.value = comment
  deleteDialog.value = true
}

function closeDeleteDialog(force = false): void {
  if (deleting.value && !force) {
    return
  }
  deleteDialog.value = false
  pendingDelete.value = null
}

async function confirmDelete(): Promise<void> {
  if (!pendingDelete.value) {
    return
  }

  const deletingId = pendingDelete.value.id
  deleting.value = true
  errorMessage.value = ''
  try {
    await deleteAdminComment(deletingId)

    const remaining = comments.value.filter((item) => item.id !== deletingId)
    const removedCount = comments.value.length - remaining.length
    if (removedCount > 0) {
      comments.value = remaining
      totalComments.value = Math.max(0, totalComments.value - removedCount)
      syncEditForms(remaining)
    }

    closeDeleteDialog(true)
    if (editingCommentId.value === deletingId) {
      closeEditDialog(true)
    }
    await loadComments(currentPage.value)
    showGlobalSuccess(`评论 #${deletingId} 已删除`)
  } catch (error) {
    const message = error instanceof Error ? error.message : '删除评论失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    deleting.value = false
  }
}

onMounted(async () => {
  try {
    siteUrl.value = await fetchSiteUrl()
  } catch (error) {
    console.warn('Failed to load site_url for comment target jump', error)
  }
  await loadComments(1)
})

watch(currentPage, async (page, previous) => {
  if (page === previous || loading.value) {
    return
  }
  await loadComments(page)
})
</script>

<style scoped>
.comments-page {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.page-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 14px;
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

.search-row {
  display: flex;
  gap: 10px;
  align-items: center;
}

.search-input {
  flex: 1;
}

.list-head {
  color: var(--admin-text-muted);
  font-size: 14px;
}

.pagination-row {
  display: flex;
  justify-content: center;
  padding: 6px 0 4px;
}

.comments-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 14px;
}

.comment-card {
  display: block;
  border: 1px solid var(--admin-border);
  background: var(--admin-card-bg-strong);
  transition:
    transform 0.22s ease,
    box-shadow 0.22s ease;
}

.comment-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--admin-shadow-hover);
}

.comment-card-shell {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
}

.comment-head {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
}

.meta-group {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.time {
  color: var(--admin-text-faint);
  font-size: 12px;
  white-space: nowrap;
}

.comment-summary {
  display: grid;
  gap: 6px;
  padding: 8px 9px;
  border-radius: 10px;
  border: 1px solid var(--admin-border-soft);
  background: var(--admin-card-bg-softer);
}

.summary-item {
  display: flex;
  gap: 8px;
  min-width: 0;
}

.summary-label {
  flex-shrink: 0;
  font-size: 12px;
  color: var(--admin-text-faint);
}

.summary-value {
  min-width: 0;
  font-size: 12px;
  color: var(--admin-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.comment-preview {
  margin: 0;
  padding: 8px 9px;
  border-radius: 10px;
  border: 1px solid var(--admin-border-soft);
  background: var(--admin-card-bg-softer);
  color: var(--admin-accent-text);
  font-size: 13px;
  line-height: 1.56;
  display: -webkit-box;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 8;
}

.card-actions {
  margin-top: 0;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.empty-card {
  grid-column: 1 / -1;
  border: 1px dashed var(--admin-empty-border);
  background: var(--admin-dashed-bg);
  color: var(--admin-text-faint);
  text-align: center;
  padding: 22px;
}

.edit-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.edit-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.content-editor {
  margin-top: 0;
}

.dialog-card {
  background: var(--admin-card-bg-strong);
  border: 1px solid var(--admin-border);
}

.dialog-title {
  font-size: 20px;
  font-weight: 700;
}

.dialog-actions {
  padding: 12px 16px 16px;
}

@media (max-width: 1200px) {
  .comments-list {
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
  }
}

@media (max-width: 900px) {
  .search-row {
    flex-direction: column;
    align-items: stretch;
  }

  .comments-list {
    grid-template-columns: 1fr;
  }

  .edit-grid {
    grid-template-columns: 1fr;
  }
}
</style>
