<template>
  <AdminLayout>
    <section class="friends-page">
      <header class="page-header">
        <div class="header-text">
          <h1>友链管理</h1>
          <p>维护友链列表并处理站点提交的友链申请。</p>
        </div>
      </header>

      <v-card class="exchange-card" rounded="lg" variant="outlined">
        <div class="exchange-head">
          <div>
            <div class="exchange-title">友链交换信息</div>
            <div class="exchange-subtitle">对方添加你站点友链时可直接使用。留空时默认继承网站设置。</div>
          </div>
          <div class="exchange-actions">
            <v-btn
              variant="text"
              prepend-icon="mdi-restore"
              :disabled="exchangeInfoSaving || exchangeInfoLoading"
              @click="resetFriendExchangeInfoToDefault"
            >
              恢复默认
            </v-btn>
            <v-btn
              color="primary"
              prepend-icon="mdi-content-save-outline"
              :loading="exchangeInfoSaving"
              @click="saveFriendExchangeInfo"
            >
              保存
            </v-btn>
          </div>
        </div>

        <v-progress-linear
          v-if="exchangeInfoLoading"
          class="mt-2"
          color="primary"
          indeterminate
        />

        <div class="exchange-grid">
          <v-text-field
            v-model="friendExchangeForm.site_title"
            label="站点标题"
            variant="outlined"
          />
          <v-text-field
            v-model="friendExchangeForm.site_url"
            label="站点链接"
            variant="outlined"
          />
          <v-text-field
            v-model="friendExchangeForm.site_icon"
            label="图标地址"
            variant="outlined"
          />
          <v-textarea
            v-model="friendExchangeForm.site_description"
            auto-grow
            label="站点描述"
            min-rows="2"
            variant="outlined"
          />
        </div>
      </v-card>

      <v-tabs v-model="activeTab" color="primary" grow>
        <v-tab value="friends">友链列表</v-tab>
        <v-tab value="applications">申请处理</v-tab>
      </v-tabs>

      <v-window v-model="activeTab" :touch="false" class="tab-window">
        <v-window-item value="friends">
          <div class="toolbar-row">
            <v-text-field
              v-model="friendKeyword"
              class="search-input"
              clearable
              density="comfortable"
              hide-details
              label="搜索友链"
              prepend-inner-icon="mdi-magnify"
              variant="outlined"
              @keydown.enter.prevent="loadFriends"
            />
            <v-btn
              color="primary"
              prepend-icon="mdi-magnify"
              :loading="friendsLoading"
              @click="loadFriends"
            >
              搜索
            </v-btn>
            <v-btn
              color="primary"
              prepend-icon="mdi-plus"
              variant="flat"
              @click="openCreateFriendDialog"
            >
              新增友链
            </v-btn>
          </div>

          <div class="list-head">共 {{ friends.length }} 条友链</div>

          <v-progress-linear
            v-if="friendsLoading"
            class="mb-2"
            color="primary"
            indeterminate
          />

          <div class="friends-grid">
            <v-card
              v-for="item in friends"
              :key="item.id"
              class="friend-card"
              rounded="lg"
            >
              <div class="friend-head">
                <div class="friend-title">
                  <div class="name">{{ item.title }}</div>
                  <div class="meta">
                    <span>#{{ item.id }}</span>
                    <span>{{ item.category }}</span>
                    <span>{{ statusLabelMap[item.status] }}</span>
                  </div>
                </div>
                <div class="friend-actions">
                  <v-btn
                    class="icon-btn"
                    icon="mdi-pencil-outline"
                    size="small"
                    variant="text"
                    @click="openEditFriendDialog(item)"
                  />
                  <v-btn
                    class="icon-btn"
                    color="error"
                    icon="mdi-delete-outline"
                    size="small"
                    variant="text"
                    @click="openDeleteDialog(item)"
                  />
                </div>
              </div>

              <div class="info-row">
                <span class="label">地址：</span>
                <a :href="item.url" class="link" target="_blank" rel="noopener noreferrer">{{ item.url }}</a>
              </div>
              <div v-if="item.favicon" class="info-row">
                <span class="label">图标：</span>
                <a :href="item.favicon" class="link" target="_blank" rel="noopener noreferrer">{{ item.favicon }}</a>
              </div>
              <div v-if="item.description" class="description">{{ item.description }}</div>
              <div class="time">创建时间：{{ formatDateTime(item.create_time) }}</div>
            </v-card>

            <v-card
              v-if="!friendsLoading && friends.length === 0"
              class="empty-card"
              rounded="lg"
            >
              暂无友链
            </v-card>
          </div>
        </v-window-item>

        <v-window-item value="applications">
          <div class="toolbar-row app-toolbar">
            <v-select
              v-model="applyStatusFilter"
              class="status-filter"
              :items="applyStatusFilters"
              item-title="label"
              item-value="value"
              density="comfortable"
              hide-details
              label="状态筛选"
              variant="outlined"
            />
            <v-text-field
              v-model="applyKeyword"
              class="search-input"
              clearable
              density="comfortable"
              hide-details
              label="搜索申请（站点名、URL、联系方式）"
              prepend-inner-icon="mdi-magnify"
              variant="outlined"
              @keydown.enter.prevent="loadApplications"
            />
            <v-text-field
              v-model="approveCategory"
              class="category-input"
              density="comfortable"
              hide-details
              label="通过后友链分类"
              variant="outlined"
            />
            <v-btn
              color="primary"
              prepend-icon="mdi-refresh"
              :loading="applicationsLoading"
              @click="loadApplications"
            >
              刷新
            </v-btn>
          </div>

          <div class="list-head">共 {{ applications.length }} 条申请</div>

          <v-progress-linear
            v-if="applicationsLoading"
            class="mb-2"
            color="primary"
            indeterminate
          />

          <div class="apply-list">
            <v-card
              v-for="item in applications"
              :key="item.id"
              class="apply-card"
              rounded="lg"
            >
              <div class="apply-head">
                <div class="apply-title-wrap">
                  <div class="name">{{ item.site_title }}</div>
                  <div class="meta">
                    <span>#{{ item.id }}</span>
                    <span>{{ applyStatusLabelMap[item.status] }}</span>
                    <span>{{ formatDateTime(item.create_time) }}</span>
                  </div>
                </div>

                <div class="apply-actions">
                  <v-btn
                    color="success"
                    size="small"
                    variant="tonal"
                    :loading="Boolean(applyActionLoading[item.id])"
                    @click="changeApplyStatus(item.id, 'approved', true)"
                  >
                    通过并入库
                  </v-btn>
                  <v-btn
                    color="warning"
                    size="small"
                    variant="tonal"
                    :loading="Boolean(applyActionLoading[item.id])"
                    @click="changeApplyStatus(item.id, 'rejected')"
                  >
                    驳回
                  </v-btn>
                  <v-btn
                    color="error"
                    size="small"
                    variant="tonal"
                    :loading="Boolean(applyActionLoading[item.id])"
                    @click="changeApplyStatus(item.id, 'blocked')"
                  >
                    拉黑
                  </v-btn>
                </div>
              </div>

              <div class="info-row">
                <span class="label">地址：</span>
                <a :href="item.site_url" class="link" target="_blank" rel="noopener noreferrer">{{ item.site_url }}</a>
              </div>
              <div v-if="item.site_icon" class="info-row">
                <span class="label">图标：</span>
                <a :href="item.site_icon" class="link" target="_blank" rel="noopener noreferrer">{{ item.site_icon }}</a>
              </div>
              <div v-if="item.contact" class="info-row">
                <span class="label">联系：</span>
                <span>{{ item.contact }}</span>
              </div>
              <div v-if="item.site_description" class="description">{{ item.site_description }}</div>
              <div class="meta-tail">
                <span>IP: {{ item.ip || '-' }}</span>
                <span>UA: {{ item.user_agent || '-' }}</span>
              </div>
            </v-card>

            <v-card
              v-if="!applicationsLoading && applications.length === 0"
              class="empty-card"
              rounded="lg"
            >
              暂无申请记录
            </v-card>
          </div>
        </v-window-item>
      </v-window>
    </section>

    <v-dialog v-model="friendDialog" max-width="760">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title class="dialog-title">{{ editingFriendId ? '编辑友链' : '新增友链' }}</v-card-title>
        <v-card-text>
          <div class="form-grid">
            <v-text-field
              v-model="friendForm.title"
              label="名称"
              variant="outlined"
            />
            <v-text-field
              v-model="friendForm.category"
              label="分类"
              variant="outlined"
            />
            <v-text-field
              v-model="friendForm.url"
              label="站点地址"
              variant="outlined"
            />
            <v-text-field
              v-model="friendForm.favicon"
              label="图标地址（可选）"
              variant="outlined"
            />
            <v-select
              v-model="friendForm.status"
              :items="statusOptions"
              item-title="label"
              item-value="value"
              label="状态"
              variant="outlined"
            />
          </div>
          <v-textarea
            v-model="friendForm.description"
            auto-grow
            label="描述（可选）"
            min-rows="3"
            variant="outlined"
          />
        </v-card-text>
        <v-card-actions class="dialog-actions">
          <v-spacer />
          <v-btn variant="text" @click="closeFriendDialog">取消</v-btn>
          <v-btn color="primary" :loading="friendSubmitting" @click="submitFriendForm">
            保存
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="deleteDialog" max-width="420">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title class="dialog-title">确认删除</v-card-title>
        <v-card-text>
          即将删除友链《{{ pendingDelete?.title || '' }}》，删除后不可恢复。
        </v-card-text>
        <v-card-actions class="dialog-actions">
          <v-spacer />
          <v-btn variant="text" @click="closeDeleteDialog">取消</v-btn>
          <v-btn color="error" :loading="deleting" @click="confirmDelete">确认删除</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="overwriteDialog" max-width="480">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title class="dialog-title">检测到重复 URL</v-card-title>
        <v-card-text>
          当前 URL 已存在友链记录，是否用当前表单内容覆盖原记录？
          <div class="mt-2">URL: {{ pendingOverwritePayload?.url || '-' }}</div>
        </v-card-text>
        <v-card-actions class="dialog-actions">
          <v-spacer />
          <v-btn variant="text" @click="closeOverwriteDialog">取消</v-btn>
          <v-btn color="warning" :loading="friendSubmitting" @click="confirmOverwriteCreate">覆盖保存</v-btn>
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
  createAdminFriend,
  deleteAdminFriend,
  fetchAdminFriendExchangeInfo,
  fetchAdminFriendApplies,
  fetchAdminFriends,
  updateAdminFriendExchangeInfo,
  updateAdminFriend,
  updateAdminFriendApplyStatus,
  type AdminFriendApplyItem,
  type AdminFriendItem,
  type AdminFriendUpsertPayload,
  type FriendExchangeInfo,
  type FriendApplyStatus,
  type FriendStatus,
} from '@/services/friends'

const activeTab = ref<'friends' | 'applications'>('friends')
const errorMessage = ref('')
const successMessage = ref('')
const { showGlobalSuccess, showGlobalError } = useGlobalSnackbar()

const friendsLoading = ref(false)
const applicationsLoading = ref(false)
const deleting = ref(false)
const friendSubmitting = ref(false)
const exchangeInfoLoading = ref(false)
const exchangeInfoSaving = ref(false)

const friendKeyword = ref('')
const applyKeyword = ref('')
const applyStatusFilter = ref<FriendApplyStatus | ''>('')
const approveCategory = ref('friend_apply')

const friends = ref<AdminFriendItem[]>([])
const applications = ref<AdminFriendApplyItem[]>([])
const applyActionLoading = ref<Record<number, boolean>>({})

const friendDialog = ref(false)
const deleteDialog = ref(false)
const overwriteDialog = ref(false)
const editingFriendId = ref<number | null>(null)
const pendingDelete = ref<AdminFriendItem | null>(null)
const pendingOverwritePayload = ref<AdminFriendUpsertPayload | null>(null)
const friendExchangeForm = reactive<FriendExchangeInfo>({
  site_title: '',
  site_url: '',
  site_icon: '',
  site_description: '',
})

const friendForm = reactive<{
  title: string
  description: string
  category: string
  favicon: string
  url: string
  status: FriendStatus
}>({
  title: '',
  description: '',
  category: 'default',
  favicon: '',
  url: '',
  status: 'ok',
})

const statusOptions: Array<{ label: string; value: FriendStatus }> = [
  { label: '正常', value: 'ok' },
  { label: '失联', value: 'missing' },
  { label: '屏蔽', value: 'blocked' },
]

const applyStatusFilters: Array<{ label: string; value: FriendApplyStatus | '' }> = [
  { label: '全部', value: '' },
  { label: '待处理', value: 'pending' },
  { label: '已通过', value: 'approved' },
  { label: '已驳回', value: 'rejected' },
  { label: '已拉黑', value: 'blocked' },
]

const statusLabelMap: Record<FriendStatus, string> = {
  ok: '正常',
  missing: '失联',
  blocked: '屏蔽',
}

const applyStatusLabelMap: Record<FriendApplyStatus, string> = {
  pending: '待处理',
  approved: '已通过',
  rejected: '已驳回',
  blocked: '已拉黑',
}

function formatDateTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }
  return date.toLocaleString('zh-CN')
}

function resetFriendForm(): void {
  friendForm.title = ''
  friendForm.description = ''
  friendForm.category = 'default'
  friendForm.favicon = ''
  friendForm.url = ''
  friendForm.status = 'ok'
}

function buildFriendPayload(): AdminFriendUpsertPayload | null {
  const title = friendForm.title.trim()
  const urlInput = friendForm.url.trim()
  const faviconInput = friendForm.favicon.trim()
  const category = friendForm.category.trim() || 'default'
  if (!title) {
    errorMessage.value = '友链名称不能为空'
    showGlobalError('友链名称不能为空')
    return null
  }
  if (!urlInput) {
    errorMessage.value = '友链地址不能为空'
    showGlobalError('友链地址不能为空')
    return null
  }

  const url = normalizeHttpUrl(urlInput)
  if (!url) {
    errorMessage.value = '友链地址格式错误，请输入 http:// 或 https:// 地址'
    showGlobalError('友链地址格式错误，请输入 http:// 或 https:// 地址')
    return null
  }

  const favicon = faviconInput ? normalizeHttpUrl(faviconInput) : null
  if (faviconInput && !favicon) {
    errorMessage.value = '图标地址格式错误，请输入 http:// 或 https:// 地址'
    showGlobalError('图标地址格式错误，请输入 http:// 或 https:// 地址')
    return null
  }

  return {
    title,
    description: friendForm.description.trim() || null,
    category,
    favicon,
    url,
    status: friendForm.status,
  }
}

function normalizeHttpUrl(raw: string): string {
  const text = raw.trim()
  if (!text) {
    return ''
  }

  const withScheme = /^[a-z][a-z0-9+.-]*:\/\//i.test(text) ? text : `https://${text}`
  try {
    const parsed = new URL(withScheme)
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
      return ''
    }
    if (!parsed.hostname) {
      return ''
    }
    return parsed.toString()
  } catch {
    return ''
  }
}

function applyFriendExchangeForm(data: FriendExchangeInfo): void {
  friendExchangeForm.site_title = data.site_title || ''
  friendExchangeForm.site_url = data.site_url || ''
  friendExchangeForm.site_icon = data.site_icon || ''
  friendExchangeForm.site_description = data.site_description || ''
}

function resetFriendExchangeInfoToDefault(): void {
  friendExchangeForm.site_title = ''
  friendExchangeForm.site_url = ''
  friendExchangeForm.site_icon = ''
  friendExchangeForm.site_description = ''
}

function buildFriendExchangePayload(): FriendExchangeInfo | null {
  const siteTitle = friendExchangeForm.site_title.trim()
  const siteUrlInput = friendExchangeForm.site_url.trim()
  const siteIconInput = friendExchangeForm.site_icon.trim()
  const siteDescription = friendExchangeForm.site_description.trim()

  const siteUrl = siteUrlInput ? normalizeHttpUrl(siteUrlInput) : ''
  if (siteUrlInput && !siteUrl) {
    errorMessage.value = '站点链接格式错误，请输入 http:// 或 https:// 地址'
    showGlobalError(errorMessage.value)
    return null
  }

  const siteIcon = siteIconInput ? normalizeHttpUrl(siteIconInput) : ''
  if (siteIconInput && !siteIcon) {
    errorMessage.value = '图标地址格式错误，请输入 http:// 或 https:// 地址'
    showGlobalError(errorMessage.value)
    return null
  }

  return {
    site_title: siteTitle,
    site_url: siteUrl,
    site_icon: siteIcon,
    site_description: siteDescription,
  }
}

async function loadFriendExchangeInfo(): Promise<void> {
  exchangeInfoLoading.value = true
  errorMessage.value = ''
  try {
    const data = await fetchAdminFriendExchangeInfo()
    applyFriendExchangeForm(data)
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载友链交换信息失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    exchangeInfoLoading.value = false
  }
}

async function saveFriendExchangeInfo(): Promise<void> {
  const payload = buildFriendExchangePayload()
  if (!payload) {
    return
  }

  exchangeInfoSaving.value = true
  errorMessage.value = ''
  successMessage.value = ''
  try {
    const data = await updateAdminFriendExchangeInfo(payload)
    applyFriendExchangeForm(data)
    successMessage.value = '友链交换信息已保存'
  } catch (error) {
    const message = error instanceof Error ? error.message : '保存友链交换信息失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    exchangeInfoSaving.value = false
  }
}

async function loadFriends(): Promise<void> {
  friendsLoading.value = true
  errorMessage.value = ''
  try {
    friends.value = await fetchAdminFriends(friendKeyword.value)
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载友链失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    friendsLoading.value = false
  }
}

async function loadApplications(): Promise<void> {
  applicationsLoading.value = true
  errorMessage.value = ''
  try {
    applications.value = await fetchAdminFriendApplies({
      status: applyStatusFilter.value,
      keyword: applyKeyword.value,
    })
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载申请失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    applicationsLoading.value = false
  }
}

function openCreateFriendDialog(): void {
  errorMessage.value = ''
  successMessage.value = ''
  editingFriendId.value = null
  pendingOverwritePayload.value = null
  overwriteDialog.value = false
  resetFriendForm()
  friendDialog.value = true
}

function openEditFriendDialog(item: AdminFriendItem): void {
  errorMessage.value = ''
  successMessage.value = ''
  pendingOverwritePayload.value = null
  overwriteDialog.value = false
  editingFriendId.value = item.id
  friendForm.title = item.title
  friendForm.description = item.description || ''
  friendForm.category = item.category || 'default'
  friendForm.favicon = item.favicon || ''
  friendForm.url = item.url
  friendForm.status = item.status
  friendDialog.value = true
}

function closeFriendDialog(force = false): void {
  if (friendSubmitting.value && !force) {
    return
  }
  friendDialog.value = false
  pendingOverwritePayload.value = null
  overwriteDialog.value = false
}

function isDuplicateFriendUrlError(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error || '')
  return message.includes('Friend URL already exists')
}

function closeOverwriteDialog(force = false): void {
  if (friendSubmitting.value && !force) {
    return
  }
  overwriteDialog.value = false
  pendingOverwritePayload.value = null
}

async function confirmOverwriteCreate(): Promise<void> {
  const payload = pendingOverwritePayload.value
  if (!payload) {
    return
  }

  friendSubmitting.value = true
  errorMessage.value = ''
  successMessage.value = ''
  try {
    await createAdminFriend(payload, { overwriteExisting: true })
    closeOverwriteDialog(true)
    closeFriendDialog(true)
    successMessage.value = '检测到重复 URL，已覆盖原友链'
    await loadFriends()
  } catch (error) {
    const message = error instanceof Error ? error.message : '覆盖友链失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    friendSubmitting.value = false
  }
}

async function submitFriendForm(): Promise<void> {
  errorMessage.value = ''
  successMessage.value = ''
  const payload = buildFriendPayload()
  if (!payload) {
    return
  }

  friendSubmitting.value = true
  try {
    if (editingFriendId.value) {
      await updateAdminFriend(editingFriendId.value, payload)
      successMessage.value = '友链已更新'
    } else {
      await createAdminFriend(payload)
      successMessage.value = '友链已创建'
    }
    closeFriendDialog(true)
    await loadFriends()
  } catch (error) {
    if (!editingFriendId.value && isDuplicateFriendUrlError(error)) {
      pendingOverwritePayload.value = payload
      overwriteDialog.value = true
      return
    }
    const message = error instanceof Error ? error.message : '保存友链失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    friendSubmitting.value = false
  }
}

function openDeleteDialog(item: AdminFriendItem): void {
  pendingDelete.value = item
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
  successMessage.value = ''
  try {
    await deleteAdminFriend(pendingDelete.value.id)
    closeDeleteDialog(true)
    successMessage.value = '友链已删除'
    await loadFriends()
  } catch (error) {
    const message = error instanceof Error ? error.message : '删除友链失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    deleting.value = false
  }
}

async function changeApplyStatus(
  applyId: number,
  status: FriendApplyStatus,
  createFriend = false,
): Promise<void> {
  errorMessage.value = ''
  successMessage.value = ''
  applyActionLoading.value = {
    ...applyActionLoading.value,
    [applyId]: true,
  }

  try {
    await updateAdminFriendApplyStatus(applyId, {
      status,
      create_friend: createFriend,
      friend_category: createFriend ? (approveCategory.value.trim() || 'friend_apply') : undefined,
    })
    successMessage.value = '申请状态已更新'
    await loadApplications()
    if (createFriend) {
      await loadFriends()
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : '更新申请状态失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    const nextMap = { ...applyActionLoading.value }
    delete nextMap[applyId]
    applyActionLoading.value = nextMap
  }
}

onMounted(async () => {
  await Promise.all([
    loadFriendExchangeInfo(),
    loadFriends(),
    loadApplications(),
  ])
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
.friends-page {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.exchange-card {
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: linear-gradient(180deg, #151c2a, #121826);
  padding: 12px 14px;
}

.exchange-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 10px;
}

.exchange-actions {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}

.exchange-title {
  font-size: 16px;
  font-weight: 700;
  color: #f2f6ff;
}

.exchange-subtitle {
  margin-top: 4px;
  color: #aeb8cc;
  font-size: 13px;
}

.exchange-grid {
  margin-top: 10px;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
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

.tab-window {
  margin-top: 6px;
}

.toolbar-row {
  display: flex;
  gap: 10px;
  align-items: center;
  margin: 6px 0 10px;
}

.search-input {
  flex: 1;
}

.status-filter {
  width: 160px;
  flex-shrink: 0;
}

.category-input {
  width: 200px;
  flex-shrink: 0;
}

.list-head {
  color: #aeb8cc;
  font-size: 14px;
  margin-bottom: 8px;
}

.friends-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 10px;
}

.friend-card,
.apply-card {
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: linear-gradient(180deg, #151c2a, #121826);
  padding: 12px 14px;
}

.friend-head,
.apply-head {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 8px;
}

.friend-title,
.apply-title-wrap {
  min-width: 0;
}

.name {
  font-size: 17px;
  font-weight: 700;
  color: #f2f6ff;
  word-break: break-word;
}

.meta {
  margin-top: 4px;
  color: #9fb0d4;
  font-size: 13px;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.friend-actions,
.apply-actions {
  display: flex;
  gap: 4px;
  align-items: flex-start;
  flex-wrap: wrap;
}

.icon-btn {
  border-radius: 10px;
}

.info-row {
  color: #c4d2f0;
  font-size: 14px;
  margin: 4px 0;
  word-break: break-all;
}

.label {
  color: #8fa3cc;
}

.link {
  color: #8db3ff;
  text-decoration: none;
}

.link:hover {
  text-decoration: underline;
}

.description {
  margin-top: 8px;
  color: #dbe4f8;
  line-height: 1.7;
  white-space: pre-wrap;
}

.time {
  margin-top: 8px;
  color: #90a2ca;
  font-size: 12px;
}

.apply-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.meta-tail {
  margin-top: 8px;
  color: #90a2ca;
  font-size: 12px;
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
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

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.dialog-actions {
  padding: 12px 16px 16px;
}

@media (max-width: 1100px) {
  .app-toolbar {
    flex-wrap: wrap;
  }

  .status-filter,
  .category-input {
    width: 100%;
  }
}

@media (max-width: 900px) {
  .toolbar-row {
    flex-direction: column;
    align-items: stretch;
  }

  .form-grid {
    grid-template-columns: 1fr;
  }

  .exchange-grid {
    grid-template-columns: 1fr;
  }

  .friend-head,
  .apply-head {
    flex-direction: column;
  }
}
</style>
