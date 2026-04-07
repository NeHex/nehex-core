<template>
  <AdminLayout>
    <section class="mail-notify-page">
      <header class="page-header">
        <div class="header-text">
          <h1>邮件通知</h1>
          <p>配置第三方 SMTP、测试通信、编辑通知模板，并控制评论提醒开关。</p>
        </div>
        <div class="header-actions">
          <v-btn
            variant="text"
            prepend-icon="mdi-format-list-bulleted"
            @click="router.push('/settings/mail-management')"
          >
            查看邮件管理
          </v-btn>
          <v-btn
            color="primary"
            prepend-icon="mdi-content-save-outline"
            :loading="saving"
            @click="saveSettings"
          >
            保存配置
          </v-btn>
        </div>
      </header>

      <v-alert v-if="errorMessage" density="comfortable" type="error" variant="tonal">
        {{ errorMessage }}
      </v-alert>
      <v-alert v-if="successMessage" density="comfortable" type="success" variant="tonal">
        {{ successMessage }}
      </v-alert>

      <v-progress-linear v-if="loading" color="primary" indeterminate />

      <v-card class="section-card" rounded="xl">
        <v-card-title>SMTP 连接配置</v-card-title>
        <v-card-text>
          <div class="form-grid">
            <v-text-field
              v-model="form.smtpHost"
              label="SMTP 服务器地址"
              placeholder="smtp.example.com"
              variant="outlined"
            />
            <v-text-field
              v-model.number="form.smtpPort"
              label="SMTP 端口"
              type="number"
              variant="outlined"
            />
            <v-select
              v-model="form.smtpSecurity"
              :items="smtpSecurityOptions"
              item-title="label"
              item-value="value"
              label="加密方式"
              variant="outlined"
            />
            <v-text-field
              v-model.number="form.smtpTimeoutSeconds"
              label="超时秒数"
              type="number"
              variant="outlined"
            />
            <v-text-field v-model="form.smtpUsername" label="SMTP 用户名" variant="outlined" />
            <v-text-field
              v-model="form.smtpPassword"
              autocomplete="new-password"
              label="SMTP 密码"
              type="password"
              variant="outlined"
            />
            <v-text-field v-model="form.smtpFromEmail" label="发件邮箱（From）" variant="outlined" />
            <v-text-field v-model="form.smtpFromName" label="发件人名称（可选）" variant="outlined" />
          </div>

          <div class="test-row">
            <v-text-field
              v-model="testEmail"
              class="test-email-input"
              label="测试邮件接收邮箱"
              placeholder="you@example.com"
              variant="outlined"
            />
            <v-btn
              color="primary"
              prepend-icon="mdi-connection"
              :loading="testing"
              @click="testConnection"
            >
              测试通信
            </v-btn>
          </div>
        </v-card-text>
      </v-card>

      <v-card class="section-card" rounded="xl">
        <v-card-title>通知设置</v-card-title>
        <v-card-text>
          <div class="switch-row">
            <v-switch
              v-model="form.notifyNewCommentEnabled"
              color="primary"
              hide-details
              inset
              label="开启新评论提醒"
            />
            <v-switch
              v-model="form.notifyReplyEnabled"
              color="primary"
              hide-details
              inset
              label="开启回复提醒"
            />
          </div>

          <v-text-field
            v-model="form.notifyAdminEmail"
            label="新评论提醒接收邮箱"
            placeholder="admin@example.com"
            variant="outlined"
          />
          <p class="hint-text">
            回复提醒会发送给被回复评论的邮箱；如果评论者没有填写邮箱，将不会发送。
          </p>
        </v-card-text>
      </v-card>

      <v-card class="section-card" rounded="xl">
        <v-card-title>模板选择与编辑</v-card-title>
        <v-card-text>
          <v-tabs v-model="activeTemplateTab" class="template-tabs" color="primary">
            <v-tab value="reply">回复邮件模板</v-tab>
            <v-tab value="new-comment">新评论提醒模板</v-tab>
          </v-tabs>

          <v-window v-model="activeTemplateTab" :touch="false">
            <v-window-item value="reply">
              <div class="template-panel">
                <v-select
                  v-model="selectedReplyPreset"
                  :items="replyTemplatePresetOptions"
                  item-title="label"
                  item-value="value"
                  label="选择回复模板"
                  variant="outlined"
                  @update:model-value="applyReplyPreset"
                />
                <v-text-field
                  v-model="form.replySubjectTemplate"
                  label="回复邮件主题模板"
                  variant="outlined"
                />
                <v-textarea
                  v-model="form.replyBodyTemplate"
                  auto-grow
                  label="回复邮件内容模板"
                  min-rows="8"
                  variant="outlined"
                />
              </div>
            </v-window-item>

            <v-window-item value="new-comment">
              <div class="template-panel">
                <v-select
                  v-model="selectedNewCommentPreset"
                  :items="newCommentTemplatePresetOptions"
                  item-title="label"
                  item-value="value"
                  label="选择新评论模板"
                  variant="outlined"
                  @update:model-value="applyNewCommentPreset"
                />
                <v-text-field
                  v-model="form.newCommentSubjectTemplate"
                  label="新评论提醒主题模板"
                  variant="outlined"
                />
                <v-textarea
                  v-model="form.newCommentBodyTemplate"
                  auto-grow
                  label="新评论提醒内容模板"
                  min-rows="8"
                  variant="outlined"
                />
              </div>
            </v-window-item>
          </v-window>

          <p class="hint-text">
            可用变量：`site_title`、`target_type`、`target_id`、`comment_nickname`、`comment_email`、`comment_content`、
            `comment_time`、`parent_nickname`、`parent_content`、`reply_nickname`、`reply_content`、`reply_time`
          </p>
        </v-card-text>
      </v-card>
    </section>
  </AdminLayout>
</template>

<script lang="ts" setup>
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import {
  fetchAdminSettings,
  type AdminSettingItem,
  type AdminSettingUpdateItem,
  updateAdminSettings,
} from '@/services/admin-settings'
import {
  testAdminMailSmtpConnection,
  type AdminMailSmtpTestPayload,
  type MailSmtpSecurity,
} from '@/services/mail'

type TemplatePreset = {
  value: string
  label: string
  subject: string
  body: string
}

const router = useRouter()
const loading = ref(false)
const saving = ref(false)
const testing = ref(false)
const errorMessage = ref('')
const successMessage = ref('')
const testEmail = ref('')
const activeTemplateTab = ref<'reply' | 'new-comment'>('reply')
const selectedReplyPreset = ref('default')
const selectedNewCommentPreset = ref('default')

const form = reactive({
  smtpHost: '',
  smtpPort: 465,
  smtpSecurity: 'ssl' as MailSmtpSecurity,
  smtpUsername: '',
  smtpPassword: '',
  smtpFromEmail: '',
  smtpFromName: '',
  smtpTimeoutSeconds: 12,
  notifyAdminEmail: '',
  notifyNewCommentEnabled: false,
  notifyReplyEnabled: false,
  replySubjectTemplate: '',
  replyBodyTemplate: '',
  newCommentSubjectTemplate: '',
  newCommentBodyTemplate: '',
})

const smtpSecurityOptions = [
  { label: 'SSL/TLS', value: 'ssl' },
  { label: 'STARTTLS', value: 'starttls' },
  { label: '无加密', value: 'none' },
] satisfies Array<{ label: string, value: MailSmtpSecurity }>

const replyTemplatePresets: TemplatePreset[] = [
  {
    value: 'default',
    label: '默认模板',
    subject: '[{{site_title}}] 你的评论有新回复',
    body: '你好，{{parent_nickname}}：\n\n你在 {{site_title}} 的评论收到了新回复。\n\n原评论内容：\n{{parent_content}}\n\n回复者：{{reply_nickname}}\n回复内容：\n{{reply_content}}\n\n评论位置：{{target_type}} #{{target_id}}\n回复时间：{{reply_time}}',
  },
  {
    value: 'compact',
    label: '简洁模板',
    subject: '[{{site_title}}] 评论回复提醒',
    body: '{{reply_nickname}} 回复了你的评论：\n{{reply_content}}\n\n位置：{{target_type}} #{{target_id}}\n时间：{{reply_time}}',
  },
]

const newCommentTemplatePresets: TemplatePreset[] = [
  {
    value: 'default',
    label: '默认模板',
    subject: '[{{site_title}}] 收到新评论提醒',
    body: '{{site_title}} 收到了一条新评论。\n\n评论者：{{comment_nickname}}\n评论者邮箱：{{comment_email}}\n评论位置：{{target_type}} #{{target_id}}\n评论时间：{{comment_time}}\n\n评论内容：\n{{comment_content}}',
  },
  {
    value: 'compact',
    label: '简洁模板',
    subject: '[{{site_title}}] 新评论提醒',
    body: '{{comment_nickname}} 在 {{target_type}} #{{target_id}} 发表了新评论：\n{{comment_content}}\n\n时间：{{comment_time}}',
  },
]

const replyTemplatePresetOptions = computed(() => [
  ...replyTemplatePresets.map((item) => ({ label: item.label, value: item.value })),
  { label: '自定义', value: 'custom' },
])

const newCommentTemplatePresetOptions = computed(() => [
  ...newCommentTemplatePresets.map((item) => ({ label: item.label, value: item.value })),
  { label: '自定义', value: 'custom' },
])

function getSettingsMap(items: AdminSettingItem[]): Map<string, unknown> {
  const result = new Map<string, unknown>()
  for (const item of items) {
    result.set(item.setting_key, item.setting_content)
  }
  return result
}

function readText(settingsMap: Map<string, unknown>, key: string, fallback = ''): string {
  const value = settingsMap.get(key)
  if (value === null || value === undefined) {
    return fallback
  }
  return String(value).trim()
}

function readInt(settingsMap: Map<string, unknown>, key: string, fallback: number): number {
  const value = Number(readText(settingsMap, key, String(fallback)))
  if (!Number.isFinite(value)) {
    return fallback
  }
  return Math.max(1, Math.floor(value))
}

function readBool(settingsMap: Map<string, unknown>, key: string, fallback = false): boolean {
  const raw = readText(settingsMap, key)
  if (!raw) {
    return fallback
  }
  return ['1', 'true', 'yes', 'on'].includes(raw.toLowerCase())
}

function detectPreset(subject: string, body: string, presets: TemplatePreset[]): string {
  const matched = presets.find((item) => item.subject === subject && item.body === body)
  return matched?.value || 'custom'
}

function applyReplyPreset(value: string): void {
  const preset = replyTemplatePresets.find((item) => item.value === value)
  if (!preset) {
    return
  }
  form.replySubjectTemplate = preset.subject
  form.replyBodyTemplate = preset.body
}

function applyNewCommentPreset(value: string): void {
  const preset = newCommentTemplatePresets.find((item) => item.value === value)
  if (!preset) {
    return
  }
  form.newCommentSubjectTemplate = preset.subject
  form.newCommentBodyTemplate = preset.body
}

function fillForm(items: AdminSettingItem[]): void {
  const settingsMap = getSettingsMap(items)
  form.smtpHost = readText(settingsMap, 'mail_smtp_host')
  form.smtpPort = readInt(settingsMap, 'mail_smtp_port', 465)
  const security = readText(settingsMap, 'mail_smtp_security', 'ssl').toLowerCase()
  form.smtpSecurity = ['none', 'starttls', 'ssl'].includes(security) ? security as MailSmtpSecurity : 'ssl'
  form.smtpUsername = readText(settingsMap, 'mail_smtp_username')
  form.smtpPassword = readText(settingsMap, 'mail_smtp_password')
  form.smtpFromEmail = readText(settingsMap, 'mail_smtp_from_email')
  form.smtpFromName = readText(settingsMap, 'mail_smtp_from_name')
  form.smtpTimeoutSeconds = readInt(settingsMap, 'mail_smtp_timeout_seconds', 12)
  form.notifyAdminEmail = readText(settingsMap, 'mail_notify_admin_email')
  form.notifyNewCommentEnabled = readBool(settingsMap, 'mail_notify_new_comment_enabled', false)
  form.notifyReplyEnabled = readBool(settingsMap, 'mail_notify_reply_enabled', false)
  form.replySubjectTemplate = readText(settingsMap, 'mail_reply_subject_template', replyTemplatePresets[0]!.subject)
  form.replyBodyTemplate = readText(settingsMap, 'mail_reply_body_template', replyTemplatePresets[0]!.body)
  form.newCommentSubjectTemplate = readText(
    settingsMap,
    'mail_new_comment_subject_template',
    newCommentTemplatePresets[0]!.subject,
  )
  form.newCommentBodyTemplate = readText(
    settingsMap,
    'mail_new_comment_body_template',
    newCommentTemplatePresets[0]!.body,
  )
  selectedReplyPreset.value = detectPreset(form.replySubjectTemplate, form.replyBodyTemplate, replyTemplatePresets)
  selectedNewCommentPreset.value = detectPreset(
    form.newCommentSubjectTemplate,
    form.newCommentBodyTemplate,
    newCommentTemplatePresets,
  )
}

async function loadSettings(): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  try {
    const items = await fetchAdminSettings()
    fillForm(items)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '加载邮件通知设置失败'
  } finally {
    loading.value = false
  }
}

function buildUpdateItems(): AdminSettingUpdateItem[] {
  return [
    { setting_key: 'mail_smtp_host', setting_content: form.smtpHost.trim(), setting_type: 'string' },
    { setting_key: 'mail_smtp_port', setting_content: Math.max(1, Math.floor(form.smtpPort || 465)), setting_type: 'int' },
    { setting_key: 'mail_smtp_security', setting_content: form.smtpSecurity, setting_type: 'string' },
    { setting_key: 'mail_smtp_username', setting_content: form.smtpUsername.trim(), setting_type: 'string' },
    { setting_key: 'mail_smtp_password', setting_content: form.smtpPassword, setting_type: 'string' },
    { setting_key: 'mail_smtp_from_email', setting_content: form.smtpFromEmail.trim(), setting_type: 'string' },
    { setting_key: 'mail_smtp_from_name', setting_content: form.smtpFromName.trim(), setting_type: 'string' },
    {
      setting_key: 'mail_smtp_timeout_seconds',
      setting_content: Math.max(3, Math.floor(form.smtpTimeoutSeconds || 12)),
      setting_type: 'int',
    },
    { setting_key: 'mail_notify_admin_email', setting_content: form.notifyAdminEmail.trim(), setting_type: 'string' },
    {
      setting_key: 'mail_notify_new_comment_enabled',
      setting_content: form.notifyNewCommentEnabled,
      setting_type: 'boolean',
    },
    {
      setting_key: 'mail_notify_reply_enabled',
      setting_content: form.notifyReplyEnabled,
      setting_type: 'boolean',
    },
    {
      setting_key: 'mail_reply_subject_template',
      setting_content: form.replySubjectTemplate.trim(),
      setting_type: 'string',
    },
    { setting_key: 'mail_reply_body_template', setting_content: form.replyBodyTemplate, setting_type: 'string' },
    {
      setting_key: 'mail_new_comment_subject_template',
      setting_content: form.newCommentSubjectTemplate.trim(),
      setting_type: 'string',
    },
    { setting_key: 'mail_new_comment_body_template', setting_content: form.newCommentBodyTemplate, setting_type: 'string' },
  ]
}

async function saveSettings(): Promise<void> {
  saving.value = true
  errorMessage.value = ''
  successMessage.value = ''
  try {
    const updated = await updateAdminSettings(buildUpdateItems())
    fillForm(updated)
    successMessage.value = '邮件通知配置已保存'
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '保存失败'
  } finally {
    saving.value = false
  }
}

function buildSmtpTestPayload(): AdminMailSmtpTestPayload {
  return {
    smtp_host: form.smtpHost.trim(),
    smtp_port: Math.max(1, Math.floor(form.smtpPort || 465)),
    smtp_security: form.smtpSecurity,
    smtp_username: form.smtpUsername.trim() || null,
    smtp_password: form.smtpPassword || null,
    smtp_from_email: form.smtpFromEmail.trim() || null,
    smtp_from_name: form.smtpFromName.trim() || null,
    smtp_timeout_seconds: Math.max(3, Math.floor(form.smtpTimeoutSeconds || 12)),
    test_email: testEmail.value.trim(),
  }
}

async function testConnection(): Promise<void> {
  testing.value = true
  errorMessage.value = ''
  successMessage.value = ''
  try {
    const message = await testAdminMailSmtpConnection(buildSmtpTestPayload())
    successMessage.value = message
  } catch (error) {
    const message = error instanceof Error ? error.message : 'SMTP 通信测试失败'
    if (
      message.includes('body.test_email')
      || message.includes('test_email')
      || message.includes('String should have at least 1 character')
      || message.includes('Field is required')
    ) {
      errorMessage.value = '请填写测试接收邮箱'
    } else {
      errorMessage.value = message
    }
  } finally {
    testing.value = false
  }
}

watch(
  () => [form.replySubjectTemplate, form.replyBodyTemplate],
  ([subject, body]) => {
    selectedReplyPreset.value = detectPreset(subject || '', body || '', replyTemplatePresets)
  },
)

watch(
  () => [form.newCommentSubjectTemplate, form.newCommentBodyTemplate],
  ([subject, body]) => {
    selectedNewCommentPreset.value = detectPreset(subject || '', body || '', newCommentTemplatePresets)
  },
)

onMounted(async () => {
  await loadSettings()
})
</script>

<style scoped>
.mail-notify-page {
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

.header-text h1 {
  margin: 0;
  font-size: 28px;
  color: #f1f4ff;
}

.header-text p {
  margin: 6px 0 0;
  color: #aeb8cc;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.section-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, rgba(24, 30, 41, 0.96), rgba(19, 24, 34, 0.96));
  color: #edf1ff;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.test-row {
  margin-top: 8px;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 10px;
  align-items: center;
}

.test-email-input {
  min-width: 0;
}

.switch-row {
  display: flex;
  flex-wrap: wrap;
  gap: 18px;
}

.template-tabs {
  margin-bottom: 10px;
}

.template-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.hint-text {
  margin: 6px 0 0;
  color: #9eb1d8;
  font-size: 13px;
  line-height: 1.6;
}

@media (max-width: 980px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .header-actions {
    width: 100%;
    justify-content: flex-end;
  }

  .form-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 760px) {
  .test-row {
    grid-template-columns: 1fr;
  }

  .header-actions {
    width: 100%;
    flex-direction: column;
  }
}
</style>
