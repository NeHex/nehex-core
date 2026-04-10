<template>
  <AdminLayout>
    <section class="comments-page">
      <header class="page-header">
        <div class="header-text">
          <h1>评论管理</h1>
          <p>卡片化管理评论，支持跳转目标内容并直接编辑留言信息。</p>
        </div>
      </header>

      <v-alert
        v-if="errorMessage"
        class="mb-2"
        density="comfortable"
        type="error"
        variant="tonal"
      >
        {{ errorMessage }}
      </v-alert>

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
          rounded="lg"
        >
          <div class="comment-head">
            <div class="meta-group">
              <v-chip size="small" variant="tonal">#{{ comment.id }}</v-chip>
              <v-chip size="small" variant="tonal">{{ mapTargetLabel(comment.target_type) }}:{{ comment.target_id }}</v-chip>
              <v-chip size="small" :color="comment.parent_id > 0 ? 'warning' : 'primary'" variant="tonal">
                {{ comment.parent_id > 0 ? '回复' : '主评论' }}
              </v-chip>
            </div>
            <div class="head-actions">
              <v-btn
                variant="text"
                size="small"
                prepend-icon="mdi-open-in-new"
                :disabled="!buildTargetManagePath(comment)"
                @click="goToTarget(comment)"
              >
                跳转目标
              </v-btn>
              <span class="time">{{ formatDateTime(comment.create_time) }}</span>
            </div>
          </div>

          <div class="edit-grid">
            <v-text-field
              v-model="getEditForm(comment.id).nickname"
              density="comfortable"
              hide-details
              label="昵称"
              variant="outlined"
            />
            <v-text-field
              v-model="getEditForm(comment.id).email"
              density="comfortable"
              hide-details
              label="邮箱"
              variant="outlined"
            />
            <v-text-field
              v-model="getEditForm(comment.id).website"
              density="comfortable"
              hide-details
              label="网址"
              variant="outlined"
            />
            <v-select
              v-model.number="getEditForm(comment.id).status"
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
            v-model="getEditForm(comment.id).content"
            auto-grow
            class="content-editor"
            hide-details
            label="留言内容"
            min-rows="4"
            variant="outlined"
          />

          <div class="card-actions">
            <v-btn
              color="primary"
              prepend-icon="mdi-content-save-outline"
              size="small"
              :loading="isSaving(comment.id)"
              @click="saveComment(comment.id)"
            >
              保存
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
import { useRouter } from 'vue-router'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import {
  deleteAdminComment,
  fetchAdminComments,
  updateAdminComment,
  type AdminCommentItem,
} from '@/services/comments'

type EditCommentForm = {
  nickname: string
  email: string
  website: string
  content: string
  status: number
}

const router = useRouter()
const { showGlobalSuccess } = useGlobalSnackbar()

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
  if (targetType === 'article') {
    return '文章'
  }
  if (targetType === 'album') {
    return '相册'
  }
  if (targetType === 'singlepage') {
    return '独立页'
  }
  if (targetType === 'friend_page') {
    return '友链页'
  }
  return targetType || '未知'
}

function buildTargetManagePath(comment: AdminCommentItem): string {
  if (comment.target_type === 'friend_page') {
    return '/friends'
  }

  const targetId = Number(comment.target_id)
  if (!Number.isFinite(targetId) || targetId <= 0) {
    return ''
  }

  if (comment.target_type === 'article') {
    return `/articles/edit/${targetId}`
  }
  if (comment.target_type === 'album') {
    return `/albums/edit/${targetId}`
  }
  if (comment.target_type === 'singlepage') {
    return `/pages/edit/${targetId}`
  }
  return ''
}

function goToTarget(comment: AdminCommentItem): void {
  const path = buildTargetManagePath(comment)
  if (!path) {
    return
  }
  void router.push(path)
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
    errorMessage.value = error instanceof Error ? error.message : '加载评论失败'
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

async function saveComment(commentId: number): Promise<void> {
  const form = getEditForm(commentId)

  const nickname = form.nickname.trim()
  const content = form.content.trim()
  if (!nickname) {
    errorMessage.value = '昵称不能为空'
    return
  }
  if (!content) {
    errorMessage.value = '留言内容不能为空'
    return
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
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '保存评论失败'
  } finally {
    setSaving(commentId, false)
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

  deleting.value = true
  errorMessage.value = ''
  try {
    await deleteAdminComment(pendingDelete.value.id)
    closeDeleteDialog(true)
    await loadComments(currentPage.value)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '删除评论失败'
  } finally {
    deleting.value = false
  }
}

onMounted(async () => {
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
  color: #f1f4ff;
}

.header-text p {
  margin: 6px 0 0;
  color: #aeb8cc;
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
  color: #aeb8cc;
  font-size: 14px;
}

.pagination-row {
  display: flex;
  justify-content: center;
  padding: 6px 0 4px;
}

.comments-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.comment-card {
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: linear-gradient(180deg, #151c2a, #121826);
  padding: 12px;
}

.comment-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.meta-group {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.head-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.time {
  color: #9fb0d4;
  font-size: 12px;
  white-space: nowrap;
}

.edit-grid {
  margin-top: 10px;
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}

.content-editor {
  margin-top: 10px;
}

.card-actions {
  margin-top: 10px;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.empty-card {
  border: 1px dashed rgba(255, 255, 255, 0.18);
  background: rgba(18, 24, 38, 0.65);
  color: #9fb0d4;
  text-align: center;
  padding: 22px;
}

.dialog-card {
  background: linear-gradient(180deg, #151c2a, #121826);
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.dialog-title {
  font-size: 20px;
  font-weight: 700;
}

.dialog-actions {
  padding: 12px 16px 16px;
}

@media (max-width: 1200px) {
  .edit-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 900px) {
  .search-row {
    flex-direction: column;
    align-items: stretch;
  }

  .comment-head {
    flex-direction: column;
    align-items: flex-start;
  }

  .head-actions {
    width: 100%;
    justify-content: space-between;
  }

  .edit-grid {
    grid-template-columns: 1fr;
  }
}
</style>
