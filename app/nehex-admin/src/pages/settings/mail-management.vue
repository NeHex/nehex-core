<template>
  <AdminLayout>
    <section class="mail-management-page">
      <header class="page-header">
        <div class="header-text">
          <h1>邮件管理</h1>
          <p>查看邮件发送失败、发送成功和全部记录。</p>
        </div>
        <v-btn
          color="primary"
          prepend-icon="mdi-email-fast-outline"
          @click="router.push('/settings/mail-notify')"
        >
          前往邮件通知设置
        </v-btn>
      </header>

      <v-card class="section-card" rounded="xl">
        <v-card-text class="section-card-body">
          <v-tabs v-model="statusTab" color="primary">
            <v-tab value="failed">发送失败</v-tab>
            <v-tab value="success">发送成功</v-tab>
            <v-tab value="all">全部邮件</v-tab>
          </v-tabs>

          <div class="list-head">
            共 {{ totalItems }} 条记录
          </div>

          <v-progress-linear v-if="loading" class="mb-3" color="primary" indeterminate />

          <div class="log-list">
            <v-card
              v-for="item in logs"
              :key="item.id"
              class="log-item"
              rounded="lg"
              variant="outlined"
            >
              <div class="item-top">
                <div class="meta-left">
                  <v-chip
                    class="mr-2"
                    :color="item.status === 'success' ? 'success' : 'error'"
                    size="small"
                    variant="tonal"
                  >
                    {{ item.status === 'success' ? '发送成功' : '发送失败' }}
                  </v-chip>
                  <span>#{{ item.id }}</span>
                  <span>{{ mapCategory(item.category) }}</span>
                  <span>To: {{ item.to_email }}</span>
                </div>
                <div class="meta-right">
                  {{ formatDateTime(item.created_at) }}
                </div>
              </div>

              <div class="subject-line">
                主题：{{ item.subject }}
              </div>

              <div class="body-preview">
                {{ item.body }}
              </div>

              <v-alert
                v-if="item.error_message"
                class="mt-2"
                density="comfortable"
                type="error"
                variant="tonal"
              >
                {{ item.error_message }}
              </v-alert>
            </v-card>

            <v-card
              v-if="!loading && logs.length === 0"
              class="empty-card"
              rounded="lg"
              variant="outlined"
            >
              当前标签暂无记录
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
        </v-card-text>
      </v-card>
    </section>
  </AdminLayout>
</template>

<script lang="ts" setup>
import { onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import {
  fetchAdminMailLogs,
  type AdminMailLogItem,
  type MailLogStatusFilter,
} from '@/services/mail'

const router = useRouter()
const { showGlobalError } = useGlobalSnackbar()
const loading = ref(false)
const errorMessage = ref('')
const logs = ref<AdminMailLogItem[]>([])
const statusTab = ref<MailLogStatusFilter>('failed')
const currentPage = ref(1)
const totalItems = ref(0)
const totalPages = ref(0)
const pageSize = 20

function formatDateTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }
  return date.toLocaleString('zh-CN')
}

function mapCategory(category: string): string {
  if (category === 'reply_notice') {
    return '回复提醒'
  }
  if (category === 'new_comment_notice') {
    return '新评论提醒'
  }
  if (category === 'smtp_test') {
    return 'SMTP 测试'
  }
  return category || '未知类型'
}

async function loadLogs(targetPage = currentPage.value): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  try {
    const result = await fetchAdminMailLogs(statusTab.value, targetPage, pageSize)
    logs.value = result.items
    totalItems.value = result.pagination.total
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
    const message = error instanceof Error ? error.message : '加载邮件记录失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    loading.value = false
  }
}

watch(statusTab, async () => {
  currentPage.value = 1
  await loadLogs(1)
})

watch(currentPage, async (page, previous) => {
  if (page === previous || loading.value) {
    return
  }
  await loadLogs(page)
})

onMounted(async () => {
  await loadLogs(1)
})
</script>

<style scoped>
.mail-management-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
  height: calc(100dvh - 108px);
  min-height: 0;
  overflow: hidden;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
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

.section-card {
  border: 1px solid var(--admin-border-soft);
  background: var(--admin-card-bg);
  color: var(--admin-text-primary);
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.section-card-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.list-head {
  margin: 12px 0 10px;
  color: var(--admin-text-muted);
  font-size: 14px;
}

.log-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding-right: 2px;
}

.log-item {
  border-color: var(--admin-border-strong);
  background: var(--admin-card-bg-strong);
  padding: 12px;
}

.item-top {
  display: flex;
  justify-content: space-between;
  gap: 10px;
}

.meta-left {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  color: var(--admin-accent-text);
  font-size: 13px;
}

.meta-right {
  color: var(--admin-text-faint);
  font-size: 13px;
}

.subject-line {
  margin-top: 8px;
  font-size: 14px;
  color: var(--admin-text-heading);
  font-weight: 600;
}

.body-preview {
  margin-top: 6px;
  color: var(--admin-accent-text);
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
}

.empty-card {
  padding: 20px;
  text-align: center;
  color: var(--admin-text-faint);
}

.pagination-row {
  display: flex;
  justify-content: center;
  margin-top: 10px;
}

@media (max-width: 980px) {
  .mail-management-page {
    height: auto;
    overflow: visible;
  }

  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }
}

@media (max-width: 760px) {
  .item-top {
    flex-direction: column;
  }
}
</style>
