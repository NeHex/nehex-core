<template>
  <AdminLayout>
    <section class="developer-page">
      <header class="page-header">
        <div class="header-text">
          <h1>开发者选项</h1>
          <p>限制级功能，谨慎使用！</p>
        </div>
      </header>

      <v-card class="section-card" rounded="xl">
        <v-card-title class="section-title">CLI</v-card-title>
        <v-card-text class="section-body">
          <v-tabs v-model="cliEngine" color="primary">
            <v-tab value="postgresql">PostgreSQL</v-tab>
            <v-tab value="docker">Docker</v-tab>
          </v-tabs>

          <div class="quick-command-row">
            <v-chip
              v-for="item in quickCommands"
              :key="item"
              class="quick-command"
              color="primary"
              size="small"
              variant="tonal"
              @click="applyQuickCommand(item)"
            >
              {{ item }}
            </v-chip>
          </div>

          <v-textarea
            v-model="cliCommand"
            auto-grow
            class="command-input"
            label="COMMAND"
            max-rows="8"
            rows="3"
            variant="outlined"
          />

          <div class="action-row">
            <v-btn
              color="primary"
              prepend-icon="mdi-play"
              :loading="cliRunning"
              @click="runCliCommand"
            >
              RUN COMMAND
            </v-btn>
            <div v-if="cliResult" class="result-meta">
              退出码: {{ cliResult.exit_code }} | 耗时: {{ cliResult.duration_ms }}ms
              <span v-if="cliResult.truncated"> | 输出已截断</span>
            </div>
          </div>

          <v-card class="terminal-card" rounded="lg" variant="outlined">
            <pre class="terminal-pre">{{ cliOutput }}</pre>
          </v-card>
        </v-card-text>
      </v-card>

      <v-card class="section-card" rounded="xl">
        <v-card-title class="section-title">后端日志</v-card-title>
        <v-card-text class="section-body">
          <div class="log-toolbar">
            <v-text-field
              v-model="logKeyword"
              clearable
              density="comfortable"
              hide-details
              label="日志关键词过滤"
              prepend-inner-icon="mdi-magnify"
              variant="outlined"
              @keydown.enter="loadLogs"
            />
            <v-select
              v-model="logLimit"
              class="log-limit-select"
              density="comfortable"
              hide-details
              :items="logLimitOptions"
              label="展示条数"
              variant="outlined"
            />
            <v-switch
              v-model="autoRefresh"
              class="auto-refresh-switch"
              color="primary"
              hide-details
              inset
              label="自动刷新 (5s)"
            />
            <v-btn
              color="primary"
              prepend-icon="mdi-refresh"
              :loading="logsLoading"
              @click="loadLogs"
            >
              刷新
            </v-btn>
          </div>

          <div class="log-meta">
            共 {{ logTotal }} 条日志，当前展示最近 {{ logs.length }} 条
          </div>

          <v-card class="terminal-card" rounded="lg" variant="outlined">
            <pre class="terminal-pre">{{ logsOutput }}</pre>
          </v-card>
        </v-card-text>
      </v-card>
    </section>

    <v-dialog v-model="warningDialog" max-width="560" persistent>
      <v-card class="warning-card" rounded="xl">
        <v-card-title class="warning-title">开发者警告</v-card-title>
        <v-card-text class="warning-text">
          开发者功能需要在完全了解之下使用！<br>
          错误命令可能导致容器、数据库或服务异常，请谨慎执行。
        </v-card-text>
        <v-card-actions class="warning-actions">
          <v-spacer />
          <v-btn variant="text" @click="leaveDeveloperOptions">
            返回设定
          </v-btn>
          <v-btn color="error" variant="flat" @click="confirmWarning">
            我已知晓并继续
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </AdminLayout>
</template>

<script lang="ts" setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import {
  executeDeveloperCli,
  fetchDeveloperLogs,
  type DeveloperCliEngine,
  type DeveloperCliExecuteResult,
} from '@/services/developer'

const router = useRouter()
const { showGlobalError } = useGlobalSnackbar()

const warningDialog = ref(true)
const warningAccepted = ref(false)

const cliEngine = ref<DeveloperCliEngine>('postgresql')
const cliCommand = ref('status')
const cliRunning = ref(false)
const cliOutput = ref('等待执行命令...')
const cliResult = ref<DeveloperCliExecuteResult | null>(null)

const quickCommandMap: Record<DeveloperCliEngine, string[]> = {
  postgresql: [
    'status',
    'databases',
    'tables',
    'query SELECT table_name FROM information_schema.tables WHERE table_schema = \'public\'',
  ],
  docker: [
    'ps',
    'images',
    'compose ps',
    'compose logs backend --tail 200',
  ],
}
const quickCommands = computed(() => quickCommandMap[cliEngine.value])

const logsLoading = ref(false)
const logKeyword = ref('')
const logLimit = ref(300)
const logs = ref<string[]>([])
const logTotal = ref(0)
const autoRefresh = ref(true)
const logLimitOptions = [100, 300, 500, 1000, 2000]

let autoRefreshTimer: number | null = null

const logsOutput = computed(() => {
  if (logs.value.length === 0) {
    return '暂无后端日志'
  }
  return logs.value.join('\n')
})

watch(cliEngine, (nextEngine) => {
  if (cliCommand.value.trim()) {
    return
  }
  const firstCommand = quickCommandMap[nextEngine][0]
  cliCommand.value = firstCommand ?? ''
})

watch(logLimit, async () => {
  await loadLogs()
})

watch(autoRefresh, () => {
  restartAutoRefresh()
})

onMounted(async () => {
  await loadLogs()
  restartAutoRefresh()
})

onBeforeUnmount(() => {
  clearAutoRefreshTimer()
})

function confirmWarning(): void {
  warningAccepted.value = true
  warningDialog.value = false
}

function leaveDeveloperOptions(): void {
  warningDialog.value = false
  void router.push('/settings')
}

function applyQuickCommand(command: string): void {
  cliCommand.value = command
}

async function runCliCommand(): Promise<void> {
  if (!warningAccepted.value) {
    warningDialog.value = true
    showGlobalError('请先确认开发者警告后再执行命令')
    return
  }

  const command = cliCommand.value.trim()
  if (!command) {
    showGlobalError('请输入命令')
    return
  }

  cliRunning.value = true
  try {
    const result = await executeDeveloperCli(cliEngine.value, command)
    cliResult.value = result
    cliOutput.value = result.output || '(no output)'
  } catch (error) {
    const message = error instanceof Error ? error.message : '命令执行失败'
    cliOutput.value = message
    showGlobalError(message)
  } finally {
    cliRunning.value = false
  }
}

async function loadLogs(): Promise<void> {
  logsLoading.value = true
  try {
    const result = await fetchDeveloperLogs(logLimit.value, logKeyword.value)
    logs.value = result.logs
    logTotal.value = result.total
  } catch (error) {
    const message = error instanceof Error ? error.message : '日志加载失败'
    showGlobalError(message)
  } finally {
    logsLoading.value = false
  }
}

function clearAutoRefreshTimer(): void {
  if (autoRefreshTimer === null) {
    return
  }
  clearInterval(autoRefreshTimer)
  autoRefreshTimer = null
}

function restartAutoRefresh(): void {
  clearAutoRefreshTimer()
  if (!autoRefresh.value) {
    return
  }
  autoRefreshTimer = window.setInterval(() => {
    void loadLogs()
  }, 5000)
}
</script>

<style scoped>
.developer-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
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

.section-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, rgba(24, 30, 41, 0.96), rgba(19, 24, 34, 0.96));
  color: #edf1ff;
}

.section-title {
  font-weight: 700;
  letter-spacing: 0.2px;
}

.section-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.quick-command-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.quick-command {
  cursor: pointer;
}

.command-input :deep(textarea),
.terminal-pre {
  font-family: 'JetBrains Mono', 'SFMono-Regular', Menlo, Consolas, 'Liberation Mono', monospace;
}

.action-row {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.result-meta {
  color: #b7c6e6;
  font-size: 13px;
}

.terminal-card {
  background: #0c1118;
  border-color: rgba(113, 138, 179, 0.42);
}

.terminal-pre {
  margin: 0;
  padding: 12px;
  color: #d5e3ff;
  line-height: 1.5;
  font-size: 12px;
  max-height: 360px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
}

.log-toolbar {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 140px auto auto;
  gap: 10px;
  align-items: center;
}

.log-limit-select {
  min-width: 120px;
}

.auto-refresh-switch {
  margin-top: -2px;
}

.log-meta {
  color: #9eb1d8;
  font-size: 13px;
}

.warning-card {
  border: 1px solid rgba(255, 97, 97, 0.4);
  background: linear-gradient(180deg, rgba(45, 15, 15, 0.96), rgba(26, 11, 11, 0.96));
}

.warning-title {
  color: #ffd3d3;
  font-weight: 700;
}

.warning-text {
  color: #ffd6d6;
  line-height: 1.7;
}

.warning-actions {
  padding: 0 16px 16px;
}

@media (max-width: 980px) {
  .log-toolbar {
    grid-template-columns: 1fr;
  }

  .auto-refresh-switch {
    margin-top: 0;
  }
}
</style>
