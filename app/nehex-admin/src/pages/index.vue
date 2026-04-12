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
          <v-card class="metric-card" rounded="xl" variant="outlined">
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

            <v-sparkline
              :auto-draw="true"
              color="#64b5f6"
              :gradient="['#4fc3f7', '#0288d1']"
              :line-width="2"
              :model-value="visitSeries.values"
              :padding="14"
              smooth="6"
            />
          </v-card>
        </v-col>

        <v-col cols="12" md="6">
          <v-card class="metric-card" rounded="xl" variant="outlined">
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

            <v-sparkline
              :auto-draw="true"
              color="#81c784"
              :gradient="['#9ccc65', '#43a047']"
              :line-width="2"
              :model-value="apiSeries.values"
              :padding="14"
              smooth="6"
            />
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
  type DashboardSeries,
} from '@/services/dashboard'
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

function formatNumber(value: number): string {
  return Math.max(0, value).toLocaleString('zh-CN')
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
  await loadDashboard()
})
</script>

<style scoped>
.dashboard-grid {
  margin-top: 4px;
}

.metric-card,
.summary-card {
  border-color: rgba(148, 163, 184, 0.28);
  background: rgba(15, 23, 42, 0.35);
}

.metric-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 16px 4px;
}

.metric-title {
  font-size: 14px;
  color: rgba(226, 232, 240, 0.82);
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
  border: 1px solid rgba(148, 163, 184, 0.24);
  border-radius: 12px;
  padding: 12px 14px;
  background: rgba(15, 23, 42, 0.45);
}

.summary-label {
  font-size: 13px;
  color: rgba(203, 213, 225, 0.8);
}

.summary-value {
  margin-top: 6px;
  font-size: 24px;
  line-height: 1.2;
  font-weight: 700;
}

@media (max-width: 960px) {
  .summary-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
