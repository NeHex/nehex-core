<template>
  <AdminLayout>
    <section class="comments-page">
      <header class="page-header">
        <div class="header-text">
          <h1>评论管理</h1>
          <p>按条展示评论记录，可通过关键词搜索昵称、内容和目标信息。</p>
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
        <span>共 {{ comments.length }} 条评论</span>
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
          <div class="comment-top">
            <div class="comment-main">
              <div class="nickname">{{ comment.nickname }}</div>
              <div class="meta">
                <span>#{{ comment.id }}</span>
                <span>{{ comment.target_type }}:{{ comment.target_id }}</span>
                <span>{{ comment.parent_id > 0 ? '回复' : '主评论' }}</span>
                <span>{{ comment.status > 0 ? '启用' : '禁用' }}</span>
              </div>
            </div>

            <div class="comment-actions">
              <span class="time">{{ formatDateTime(comment.create_time) }}</span>
              <v-btn
                class="delete-btn"
                color="error"
                icon="mdi-delete-outline"
                size="small"
                variant="text"
                @click="openDeleteDialog(comment)"
              />
            </div>
          </div>

          <div class="content">
            {{ comment.content }}
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
import { onMounted, ref } from 'vue'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import {
  deleteAdminComment,
  fetchAdminComments,
  type AdminCommentItem,
} from '@/services/comments'

const loading = ref(false)
const deleting = ref(false)
const errorMessage = ref('')
const searchKeyword = ref('')
const comments = ref<AdminCommentItem[]>([])
const deleteDialog = ref(false)
const pendingDelete = ref<AdminCommentItem | null>(null)

function formatDateTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }
  return date.toLocaleString('zh-CN')
}

async function loadComments(): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  try {
    comments.value = await fetchAdminComments(searchKeyword.value)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '加载评论失败'
  } finally {
    loading.value = false
  }
}

async function searchComments(): Promise<void> {
  await loadComments()
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
    await loadComments()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '删除评论失败'
  } finally {
    deleting.value = false
  }
}

onMounted(async () => {
  await loadComments()
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

.comments-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.comment-card {
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: linear-gradient(180deg, #151c2a, #121826);
  padding: 12px 14px;
}

.comment-top {
  display: flex;
  justify-content: space-between;
  gap: 10px;
}

.comment-main {
  min-width: 0;
}

.nickname {
  font-size: 17px;
  font-weight: 700;
  color: #f2f6ff;
}

.meta {
  margin-top: 4px;
  color: #9fb0d4;
  font-size: 13px;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.comment-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.time {
  color: #9fb0d4;
  font-size: 12px;
  white-space: nowrap;
}

.content {
  margin-top: 10px;
  color: #dbe4f8;
  line-height: 1.75;
  white-space: pre-wrap;
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

@media (max-width: 900px) {
  .search-row {
    flex-direction: column;
    align-items: stretch;
  }

  .comment-top {
    flex-direction: column;
  }

  .comment-actions {
    justify-content: space-between;
  }
}
</style>
