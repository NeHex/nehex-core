<template>
  <AdminLayout>
    <AdminSection
      description="访问趋势、调用趋势与全站总量总览。"
      subtitle="Dashboard"
      title="仪表盘"
    >

      <v-progress-linear
        v-if="loading"
        class="mb-4"
        color="primary"
        indeterminate
      />

      <v-row class="dashboard-grid" dense>
        <v-col cols="12" md="6">
          <v-card class="metric-card metric-card--sparkline overflow-visible" rounded="xl" variant="outlined">
            <div class="metric-head">
              <div>
                <div class="metric-title">独立IP访问数</div>
                <div class="metric-value">{{ formatNumber(visitSeries.total) }}</div>
              </div>
              <v-btn-toggle
                v-model="visitPeriod"
                class="period-toggle"
                density="compact"
                divided
                mandatory
              >
                <v-btn
                  v-for="item in periodOptions"
                  :key="item.value"
                  :value="item.value"
                  size="small"
                >
                  {{ item.label }}
                </v-btn>
              </v-btn-toggle>
            </div>

            <v-sheet
              class="metric-sheet--offset mx-auto"
              color="#0288d1"
              elevation="4"
              max-width="calc(100% - 32px)"
              rounded="lg"
            >
              <v-sparkline
                auto-draw
                :model-value="visitSeries.values"
                color="white"
                :line-width="2"
                :padding="16"
                smooth="6"
              />
            </v-sheet>

          </v-card>
        </v-col>

        <v-col cols="12" md="6">
          <v-card class="metric-card metric-card--sparkline overflow-visible" rounded="xl" variant="outlined">
            <div class="metric-head">
              <div>
                <div class="metric-title">API调用数据</div>
                <div class="metric-value">{{ formatNumber(apiSeries.total) }}</div>
              </div>
              <v-btn-toggle
                v-model="apiPeriod"
                class="period-toggle"
                density="compact"
                divided
                mandatory
              >
                <v-btn
                  v-for="item in periodOptions"
                  :key="item.value"
                  :value="item.value"
                  size="small"
                >
                  {{ item.label }}
                </v-btn>
              </v-btn-toggle>
            </div>

            <v-sheet
              class="metric-sheet--offset mx-auto"
              color="#43a047"
              elevation="4"
              max-width="calc(100% - 32px)"
              rounded="lg"
            >
              <v-sparkline
                auto-draw
                :model-value="apiSeries.values"
                color="white"
                :line-width="2"
                :padding="16"
                smooth="6"
              />
            </v-sheet>

          </v-card>
        </v-col>

        <v-col cols="12">
          <v-card class="summary-card" rounded="xl" variant="outlined">
            <div class="summary-head">全站数据统计</div>
            <div class="summary-grid">
              <div class="summary-item">
                <div class="summary-label">总文字数</div>
                <div class="summary-value">{{ formatNumber(siteTotals.text_count) }}</div>
              </div>
              <div class="summary-item">
                <div class="summary-label">文章数</div>
                <div class="summary-value">{{ formatNumber(siteTotals.article_count) }}</div>
              </div>
              <div class="summary-item">
                <div class="summary-label">评论数</div>
                <div class="summary-value">{{ formatNumber(siteTotals.comment_count) }}</div>
              </div>
              <div class="summary-item">
                <div class="summary-label">相册数</div>
                <div class="summary-value">{{ formatNumber(siteTotals.album_count) }}</div>
              </div>
              <div class="summary-item">
                <div class="summary-label">友链数</div>
                <div class="summary-value">{{ formatNumber(siteTotals.friend_count) }}</div>
              </div>
            </div>
          </v-card>
        </v-col>

        <v-col cols="12">
          <v-card class="summary-card" rounded="xl" variant="outlined">
            <div class="summary-head">最新评论</div>
            <div v-if="recentComments.length > 0" class="recent-comment-list">
              <div
                v-for="item in recentComments"
                :key="item.id"
                class="recent-comment-item"
              >
                <div class="recent-comment-meta">
                  <span class="recent-comment-author">{{ item.nickname || '匿名用户' }}</span>
                  <span class="recent-comment-target">
                    {{ formatCommentTarget(item.target_type, item.target_id) }}
                  </span>
                  <span
                    class="recent-comment-status"
                    :class="item.status > 0 ? 'is-ok' : 'is-pending'"
                  >
                    {{ item.status > 0 ? '已审核' : '待审核' }}
                  </span>
                  <span class="recent-comment-time">{{ formatDateTime(item.create_time) }}</span>
                </div>
                <div class="recent-comment-content">{{ item.content || '（无内容）' }}</div>
                <div class="recent-comment-actions">
                  <v-btn
                    prepend-icon="mdi-open-in-new"
                    size="small"
                    variant="text"
                    :disabled="!canOpenCommentTarget(item)"
                    @click="openCommentTarget(item)"
                  >
                    前台直达
                  </v-btn>
                </div>
              </div>
            </div>
            <div v-else class="recent-comment-empty">暂无评论记录</div>
          </v-card>
        </v-col>
      </v-row>
    </AdminSection>
  </AdminLayout>
</template>

<script lang="ts" setup>
import AdminLayout from '@/components/admin/AdminLayout.vue'
import AdminSection from '@/components/admin/AdminSection.vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import {
  fetchDashboardData,
  type DashboardData,
  type DashboardPeriodKey,
  type DashboardRecentComment,
  type DashboardSeries,
} from '@/services/dashboard'
import { fetchStandalonePageById } from '@/services/pages'
import { fetchSiteUrl } from '@/services/settings'
import {
  buildCommentTargetUrl,
  canJumpToCommentTarget,
  mapCommentTargetLabel,
} from '@/utils/commentTargets'
import { computed, onMounted, ref } from 'vue'

const periodOptions: Array<{ value: DashboardPeriodKey, label: string }> = [
  { value: 'day', label: '日' },
  { value: 'week', label: '周' },
  { value: 'month', label: '月' },
  { value: 'year', label: '年' },
]

const loading = ref(false)
const errorMessage = ref('')
const visitPeriod = ref<DashboardPeriodKey>('day')
const apiPeriod = ref<DashboardPeriodKey>('day')
const dashboardData = ref<DashboardData | null>(null)
const { showGlobalError } = useGlobalSnackbar()
const siteUrl = ref('')
const singlePagePathCache = ref<Record<number, string>>({})

const emptySeries: DashboardSeries = {
  labels: [],
  values: [],
  total: 0,
}

const visitSeries = computed<DashboardSeries>(() => (
  dashboardData.value?.visit_ip[visitPeriod.value] || emptySeries
))

const apiSeries = computed<DashboardSeries>(() => (
  dashboardData.value?.api_calls[apiPeriod.value] || emptySeries
))

const siteTotals = computed(() => dashboardData.value?.site_totals || {
  text_count: 0,
  article_count: 0,
  comment_count: 0,
  album_count: 0,
  friend_count: 0,
})

const recentComments = computed<DashboardRecentComment[]>(() => (
  dashboardData.value?.recent_comments || []
))

function formatNumber(value: number): string {
  return Math.max(0, value).toLocaleString('zh-CN')
}

function formatDateTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value || '-'
  }
  return date.toLocaleString('zh-CN')
}

function formatCommentTarget(type: string, id: number): string {
  return `${mapCommentTargetLabel(type)} #${Math.max(0, id)}`
}

function canOpenCommentTarget(comment: DashboardRecentComment): boolean {
  return canJumpToCommentTarget(comment)
}

async function resolveStandalonePagePath(pageId: number): Promise<string | null> {
  const normalizedId = Math.max(1, Math.floor(Number(pageId) || 0))
  const cachedPath = singlePagePathCache.value[normalizedId]
  if (cachedPath) {
    return cachedPath
  }

  try {
    const page = await fetchStandalonePageById(normalizedId)
    const pageKey = String(page.page_key ?? '').trim().replace(/^\/+|\/+$/g, '')
    if (pageKey) {
      const resolvedPath = `/${pageKey}`
      singlePagePathCache.value = {
        ...singlePagePathCache.value,
        [normalizedId]: resolvedPath,
      }
      return resolvedPath
    }
  } catch (error) {
    console.warn('Failed to resolve dashboard comment target path', error)
  }

  return null
}

async function openCommentTarget(comment: DashboardRecentComment): Promise<void> {
  const targetUrl = await buildCommentTargetUrl(comment, siteUrl.value, resolveStandalonePagePath)
  if (!targetUrl) {
    showGlobalError('无法生成前台页面跳转地址')
    return
  }
  window.open(targetUrl, '_blank', 'noopener')
}

async function loadDashboard(): Promise<void> {
  loading.value = true
  errorMessage.value = ''

  try {
    dashboardData.value = await fetchDashboardData()
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载仪表盘数据失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  try {
    siteUrl.value = await fetchSiteUrl()
  } catch (error) {
    console.warn('Failed to load site_url for dashboard comment jump', error)
  }
  await loadDashboard()
})
</script>

<style scoped>
.dashboard-grid {
  margin-top: 4px;
}

.metric-card,
.summary-card {
  border-color: var(--admin-border-strong);
  background: var(--admin-card-bg-soft);
}

.metric-card--sparkline {
  padding-bottom: 8px;
}

.metric-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 16px 6px;
}

.metric-title {
  font-size: 14px;
  color: var(--admin-text-faint);
}

.metric-value {
  margin-top: 2px;
  font-size: 28px;
  line-height: 1.15;
  font-weight: 700;
}

.period-toggle {
  flex-shrink: 0;
}

.metric-sheet--offset {
  top: -2px;
  position: relative;
}

.summary-card {
  padding: 16px;
}

.summary-head {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 14px;
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 12px;
}

.summary-item {
  border: 1px solid var(--admin-border);
  border-radius: 12px;
  padding: 12px 14px;
  background: var(--admin-card-bg-softer);
}

.summary-label {
  font-size: 13px;
  color: var(--admin-text-faint);
}

.summary-value {
  margin-top: 6px;
  font-size: 24px;
  line-height: 1.2;
  font-weight: 700;
}

.recent-comment-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.recent-comment-item {
  border: 1px solid var(--admin-border);
  border-radius: 12px;
  padding: 10px 12px;
  background: var(--admin-card-bg-softer);
}

.recent-comment-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  color: var(--admin-text-faint);
  font-size: 12px;
}

.recent-comment-author {
  font-weight: 600;
  color: var(--admin-text-heading);
}

.recent-comment-status {
  border-radius: 999px;
  padding: 1px 8px;
}

.recent-comment-status.is-ok {
  color: #22c55e;
  background: rgba(34, 197, 94, 0.14);
}

.recent-comment-status.is-pending {
  color: #f59e0b;
  background: rgba(245, 158, 11, 0.14);
}

.recent-comment-time {
  margin-left: auto;
}

.recent-comment-content {
  margin-top: 6px;
  font-size: 13px;
  line-height: 1.6;
  color: var(--admin-text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
}

.recent-comment-actions {
  margin-top: 8px;
  display: flex;
  justify-content: flex-end;
}

.recent-comment-empty {
  color: var(--admin-text-faint);
  font-size: 14px;
}

@media (max-width: 960px) {
  .summary-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
