<template>
  <AdminLayout>
    <section class="backup-restore-page">
      <header class="page-header">
        <div class="header-text">
          <h1>备份与恢复</h1>
          <p>备份全站数据（数据库与本地存储）为 .tar.gz，可下载并按需恢复覆盖。</p>
        </div>
        <div class="header-actions">
          <v-btn
            variant="text"
            prepend-icon="mdi-upload"
            :disabled="creating || restoring || !!deletingFilename"
            @click="pickRestoreFile"
          >
            上传备份并恢复
          </v-btn>
          <v-btn
            color="primary"
            prepend-icon="mdi-database-export-outline"
            :loading="creating"
            :disabled="restoring || !!deletingFilename"
            @click="createBackup"
          >
            创建备份
          </v-btn>
        </div>
        <input
          ref="uploadInput"
          class="hidden-file-input"
          type="file"
          accept=".tar.gz,application/gzip,application/x-gzip"
          @change="handleRestoreFileSelect"
        >
      </header>

      <v-alert v-if="errorMessage" density="comfortable" type="error" variant="tonal">
        {{ errorMessage }}
      </v-alert>
      <v-alert v-if="successMessage" density="comfortable" type="success" variant="tonal">
        {{ successMessage }}
      </v-alert>

      <v-card class="section-card" rounded="xl">
        <v-card-text>
          <div class="list-head">共 {{ backups.length }} 个备份包</div>

          <v-progress-linear v-if="loading" class="mb-3" color="primary" indeterminate />

          <div class="backup-list">
            <v-card
              v-for="item in backups"
              :key="item.filename"
              class="backup-item"
              rounded="lg"
              variant="outlined"
            >
              <div class="item-top">
                <div class="meta-left">
                  <v-chip color="primary" size="small" variant="tonal">
                    tar.gz
                  </v-chip>
                  <span class="filename">{{ item.filename }}</span>
                </div>
                <div class="meta-right">
                  {{ formatDateTime(item.updated_at) }}
                </div>
              </div>

              <div class="item-meta">
                <span>大小：{{ formatFileSize(item.size_bytes) }}</span>
                <span>创建：{{ formatDateTime(item.created_at) }}</span>
              </div>

              <div class="item-actions">
                <v-btn
                  variant="text"
                  prepend-icon="mdi-download"
                  :loading="downloadingFilename === item.filename"
                  @click="downloadBackup(item.filename)"
                >
                  下载
                </v-btn>
                <v-btn
                  color="warning"
                  prepend-icon="mdi-database-refresh-outline"
                  variant="tonal"
                  :disabled="restoring || !!deletingFilename"
                  @click="openRestoreDialog(item.filename)"
                >
                  恢复
                </v-btn>
                <v-btn
                  color="error"
                  prepend-icon="mdi-delete-outline"
                  variant="tonal"
                  :loading="deletingFilename === item.filename"
                  :disabled="restoring || !!deletingFilename"
                  @click="openDeleteDialog(item.filename)"
                >
                  删除
                </v-btn>
              </div>
            </v-card>

            <v-card
              v-if="!loading && backups.length === 0"
              class="empty-card"
              rounded="lg"
              variant="outlined"
            >
              暂无备份，请先创建一个备份包。
            </v-card>
          </div>
        </v-card-text>
      </v-card>

      <v-dialog v-model="restoreDialog" max-width="560">
        <v-card class="dialog-card" rounded="xl">
          <v-card-title>确认恢复备份</v-card-title>
          <v-card-text>
            <p class="dialog-tip">恢复将覆盖当前数据库和本地存储，且不可撤销。</p>
            <p class="dialog-source">恢复来源：{{ restoreMode === 'upload' ? '上传文件' : '备份列表' }}</p>
            <p class="dialog-file">目标备份：{{ pendingRestoreFilename || '-' }}</p>
            <v-text-field
              v-model="restoreConfirmText"
              label="请输入“覆盖”以确认"
              placeholder="覆盖"
              variant="outlined"
            />
          </v-card-text>
          <v-card-actions>
            <v-spacer />
            <v-btn
              variant="text"
              :disabled="restoring"
              @click="closeRestoreDialog"
            >
              取消
            </v-btn>
            <v-btn
              color="error"
              prepend-icon="mdi-alert-outline"
              :disabled="!canConfirmRestore"
              :loading="restoring"
              @click="confirmRestore"
            >
              确认覆盖恢复
            </v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>

      <v-dialog v-model="deleteDialog" max-width="520">
        <v-card class="dialog-card" rounded="xl">
          <v-card-title>确认删除备份</v-card-title>
          <v-card-text>
            <p class="dialog-tip">删除后不可恢复，请谨慎操作。</p>
            <p class="dialog-file">目标备份：{{ pendingDeleteFilename || '-' }}</p>
            <v-text-field
              v-model="deleteConfirmText"
              label="请输入“删除”以确认"
              placeholder="删除"
              variant="outlined"
            />
          </v-card-text>
          <v-card-actions>
            <v-spacer />
            <v-btn
              variant="text"
              :disabled="!!deletingFilename"
              @click="closeDeleteDialog"
            >
              取消
            </v-btn>
            <v-btn
              color="error"
              prepend-icon="mdi-delete-alert-outline"
              :disabled="!canConfirmDelete"
              :loading="!!deletingFilename"
              @click="confirmDelete"
            >
              确认删除
            </v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>
    </section>
  </AdminLayout>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import {
  createAdminBackup,
  deleteAdminBackup,
  downloadAdminBackup,
  fetchAdminBackups,
  restoreAdminBackup,
  uploadAndRestoreAdminBackup,
  type AdminBackupItem,
} from '@/services/backup'

const loading = ref(false)
const creating = ref(false)
const restoring = ref(false)
const downloadingFilename = ref('')
const deletingFilename = ref('')
const backups = ref<AdminBackupItem[]>([])
const errorMessage = ref('')
const successMessage = ref('')

const restoreDialog = ref(false)
const restoreMode = ref<'existing' | 'upload'>('existing')
const pendingRestoreFilename = ref('')
const pendingRestoreFile = ref<File | null>(null)
const restoreConfirmText = ref('')
const deleteDialog = ref(false)
const pendingDeleteFilename = ref('')
const deleteConfirmText = ref('')
const uploadInput = ref<HTMLInputElement | null>(null)

const canConfirmRestore = computed(() => {
  if (restoring.value || !!deletingFilename.value) {
    return false
  }
  return restoreConfirmText.value.trim() === '覆盖' && !!pendingRestoreFilename.value
})

const canConfirmDelete = computed(() => {
  if (deletingFilename.value || restoring.value) {
    return false
  }
  return deleteConfirmText.value.trim() === '删除' && !!pendingDeleteFilename.value
})

function formatDateTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }
  return date.toLocaleString('zh-CN')
}

function formatFileSize(size: number): string {
  const safeSize = Number.isFinite(size) ? Math.max(0, size) : 0
  if (safeSize < 1024) {
    return `${safeSize} B`
  }
  if (safeSize < 1024 * 1024) {
    return `${(safeSize / 1024).toFixed(1)} KB`
  }
  if (safeSize < 1024 * 1024 * 1024) {
    return `${(safeSize / (1024 * 1024)).toFixed(2)} MB`
  }
  return `${(safeSize / (1024 * 1024 * 1024)).toFixed(2)} GB`
}

async function loadBackups(): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  try {
    backups.value = await fetchAdminBackups()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '加载备份列表失败'
  } finally {
    loading.value = false
  }
}

async function createBackup(): Promise<void> {
  creating.value = true
  errorMessage.value = ''
  successMessage.value = ''
  try {
    const item = await createAdminBackup()
    successMessage.value = `备份创建成功：${item.filename}`
    await loadBackups()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '创建备份失败'
  } finally {
    creating.value = false
  }
}

async function downloadBackup(filename: string): Promise<void> {
  downloadingFilename.value = filename
  errorMessage.value = ''
  successMessage.value = ''
  try {
    await downloadAdminBackup(filename)
    successMessage.value = `开始下载备份：${filename}`
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '下载备份失败'
  } finally {
    downloadingFilename.value = ''
  }
}

function openRestoreDialog(filename: string): void {
  restoreMode.value = 'existing'
  pendingRestoreFilename.value = filename
  pendingRestoreFile.value = null
  restoreConfirmText.value = ''
  restoreDialog.value = true
}

function pickRestoreFile(): void {
  uploadInput.value?.click()
}

function handleRestoreFileSelect(event: Event): void {
  const target = event.target as HTMLInputElement | null
  const file = target?.files?.[0]
  if (!file) {
    return
  }
  restoreMode.value = 'upload'
  pendingRestoreFilename.value = file.name
  pendingRestoreFile.value = file
  restoreConfirmText.value = ''
  restoreDialog.value = true
  if (target) {
    target.value = ''
  }
}

function closeRestoreDialog(): void {
  restoreDialog.value = false
  restoreMode.value = 'existing'
  pendingRestoreFilename.value = ''
  pendingRestoreFile.value = null
  restoreConfirmText.value = ''
}

function openDeleteDialog(filename: string): void {
  pendingDeleteFilename.value = filename
  deleteConfirmText.value = ''
  deleteDialog.value = true
}

function closeDeleteDialog(): void {
  deleteDialog.value = false
  pendingDeleteFilename.value = ''
  deleteConfirmText.value = ''
}

async function confirmRestore(): Promise<void> {
  if (!canConfirmRestore.value) {
    return
  }

  restoring.value = true
  errorMessage.value = ''
  successMessage.value = ''
  try {
    const message = restoreMode.value === 'upload' && pendingRestoreFile.value
      ? await uploadAndRestoreAdminBackup(pendingRestoreFile.value)
      : await restoreAdminBackup(pendingRestoreFilename.value)
    successMessage.value = message
    closeRestoreDialog()
    await loadBackups()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '恢复备份失败'
  } finally {
    restoring.value = false
  }
}

async function confirmDelete(): Promise<void> {
  if (!canConfirmDelete.value) {
    return
  }

  const filename = pendingDeleteFilename.value
  deletingFilename.value = filename
  errorMessage.value = ''
  successMessage.value = ''

  try {
    const message = await deleteAdminBackup(filename)
    successMessage.value = message
    closeDeleteDialog()
    await loadBackups()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '删除备份失败'
  } finally {
    deletingFilename.value = ''
  }
}

onMounted(async () => {
  await loadBackups()
})
</script>

<style scoped>
.backup-restore-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: 14px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.hidden-file-input {
  display: none;
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

.list-head {
  margin: 6px 0 10px;
  color: #aeb8cc;
  font-size: 14px;
}

.backup-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.backup-item {
  border-color: rgba(255, 255, 255, 0.14);
  background: linear-gradient(180deg, rgba(30, 37, 49, 0.82), rgba(22, 28, 39, 0.82));
  padding: 12px;
}

.item-top {
  display: flex;
  justify-content: space-between;
  gap: 10px;
}

.meta-left {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #d7e0f5;
}

.filename {
  word-break: break-all;
}

.meta-right {
  color: #9eb1d8;
  font-size: 13px;
  white-space: nowrap;
}

.item-meta {
  margin-top: 8px;
  display: flex;
  flex-wrap: wrap;
  gap: 14px;
  color: #b8c7e8;
  font-size: 13px;
}

.item-actions {
  margin-top: 10px;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.empty-card {
  border-style: dashed;
  border-color: rgba(255, 255, 255, 0.2);
  padding: 20px;
  text-align: center;
  color: #aeb8cc;
}

.dialog-card {
  border: 1px solid rgba(255, 255, 255, 0.14);
  background: linear-gradient(180deg, rgba(23, 29, 40, 0.98), rgba(18, 24, 34, 0.98));
  color: #edf1ff;
}

.dialog-tip {
  margin: 0 0 6px;
  color: #ffb4a9;
}

.dialog-source {
  margin: 0 0 6px;
  color: #b8c7e8;
}

.dialog-file {
  margin: 0 0 12px;
  color: #d7e0f5;
  word-break: break-all;
}

@media (max-width: 900px) {
  .page-header {
    flex-direction: column;
    align-items: stretch;
  }

  .header-actions {
    justify-content: flex-start;
  }

  .item-top {
    flex-direction: column;
  }

  .meta-right {
    white-space: normal;
  }
}
</style>
