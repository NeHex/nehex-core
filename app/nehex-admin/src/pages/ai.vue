<template>
  <AdminLayout>
    <section class="ai-page">
      <header class="page-header">
        <div class="header-text">
          <h1>AI配置</h1>
          <p>管理 AI 服务与功能开关，保存后对后台功能即时生效。</p>
        </div>
        <div class="header-actions">
          <v-btn prepend-icon="mdi-refresh" variant="text" :loading="loading" @click="loadSettings">
            刷新
          </v-btn>
          <v-btn prepend-icon="mdi-restore" variant="text" :disabled="saving" @click="resetForm">
            重置
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

      <v-progress-linear v-if="loading" class="mb-4" color="primary" indeterminate />

      <v-card class="tabs-card" rounded="xl">
        <v-tabs v-model="activeTab" class="tabs-head" color="primary">
          <v-tab value="ai-config">AI配置</v-tab>
          <v-tab value="feature-config">功能配置</v-tab>
        </v-tabs>
      </v-card>

      <v-window v-model="activeTab" class="tab-window" :touch="false">
        <v-window-item value="ai-config">
          <v-card class="section-card" rounded="xl">
            <v-card-title>AI配置</v-card-title>
            <v-card-text class="section-body">
              <v-switch
                v-model="enabled"
                color="primary"
                hide-details
                inset
                label="启用 AI 功能"
              />

              <v-select
                v-model="provider"
                class="mt-4"
                density="comfortable"
                :items="providerOptions"
                item-title="label"
                item-value="value"
                label="服务商"
                variant="outlined"
              />

              <div class="preset-row">
                <div class="preset-meta">
                  <div class="preset-title">预设填充</div>
                  <div class="preset-desc">
                    {{ currentProviderPreset ? currentProviderPreset.description : '当前服务商暂无内置预设' }}
                  </div>
                </div>
                <v-btn
                  color="primary"
                  prepend-icon="mdi-auto-fix"
                  :disabled="!currentProviderPreset"
                  variant="tonal"
                  @click="applyProviderPreset"
                >
                  应用预设
                </v-btn>
              </div>

              <v-text-field
                v-model="baseUrl"
                class="mt-2"
                density="comfortable"
                label="API BASE URL"
                placeholder="https://api.openai.com/v1"
                variant="outlined"
              />

              <v-text-field
                v-model="model"
                class="mt-2"
                density="comfortable"
                label="模型"
                placeholder="gpt-4.1-mini"
                variant="outlined"
              />

              <v-text-field
                v-model="apiKey"
                class="mt-2"
                density="comfortable"
                :type="showApiKey ? 'text' : 'password'"
                label="API KEY"
                placeholder="sk-..."
                variant="outlined"
              >
                <template #append-inner>
                  <v-btn
                    :icon="showApiKey ? 'mdi-eye-off-outline' : 'mdi-eye-outline'"
                    size="small"
                    variant="text"
                    @click="showApiKey = !showApiKey"
                  />
                </template>
              </v-text-field>
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="feature-config">
          <v-card class="section-card" rounded="xl">
            <v-card-title>功能配置</v-card-title>
            <v-card-text class="section-body">
              <div class="feature-toggle-row">
                <v-btn
                  color="primary"
                  prepend-icon="mdi-text-box-edit-outline"
                  variant="tonal"
                  @click="openPromptDialog('comment')"
                >
                  提示词
                </v-btn>
                <v-switch
                  v-model="commentModerationEnabled"
                  class="feature-switch"
                  color="primary"
                  hide-details
                  inset
                  label="AI评论审核"
                />
              </div>

              <div class="feature-toggle-row">
                <v-btn
                  color="primary"
                  prepend-icon="mdi-text-box-edit-outline"
                  variant="tonal"
                  @click="openPromptDialog('summary')"
                >
                  提示词
                </v-btn>
                <v-switch
                  v-model="articleSummaryEnabled"
                  class="feature-switch"
                  color="primary"
                  hide-details
                  inset
                  label="AI文章总结"
                />
              </div>
            </v-card-text>
          </v-card>
        </v-window-item>
      </v-window>

      <v-dialog v-model="promptDialogVisible" max-width="760">
        <v-card class="dialog-card" rounded="xl">
          <v-card-title>{{ promptDialogTitle }}</v-card-title>
          <v-card-text>
            <v-textarea
              v-model="promptDialogDraft"
              autofocus
              label="提示词内容"
              rows="8"
              variant="outlined"
            />
          </v-card-text>
          <v-card-actions>
            <v-spacer />
            <v-btn variant="text" @click="closePromptDialog">取消</v-btn>
            <v-btn color="primary" :loading="promptDialogSaving" @click="savePromptDialog">
              保存
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
  fetchAdminSettings,
  updateAdminSettings,
  type AdminSettingItem,
  type AdminSettingUpdateItem,
} from '@/services/admin-settings'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'

const KEY_ENABLED = 'ai_enabled'
const KEY_PROVIDER = 'ai_provider'
const KEY_BASE_URL = 'ai_base_url'
const KEY_API_KEY = 'ai_api_key'
const KEY_MODEL = 'ai_model'
const LEGACY_SYSTEM_PROMPT_KEY = 'ai_system_prompt'
const KEY_COMMENT_MODERATION_ENABLED = 'ai_comment_moderation_enabled'
const KEY_ARTICLE_SUMMARY_ENABLED = 'ai_article_summary_enabled'
const KEY_COMMENT_MODERATION_PROMPT = 'ai_comment_moderation_prompt'
const KEY_ARTICLE_SUMMARY_PROMPT = 'ai_article_summary_prompt'
const DEFAULT_COMMENT_MODERATION_PROMPT = '这是一条评论内容，你需要判断是否有违规内容，返回给我的内容只需要回复 yes/no，评论内容{commit_content}'
const DEFAULT_ARTICLE_SUMMARY_PROMPT = '这是我写的一篇文章，希望你可以彻底读懂然后总结出来，总结内容最好控制在200字左右，文章内容: {article_content}'

const loading = ref(false)
const saving = ref(false)
const showApiKey = ref(false)
const activeTab = ref<'ai-config' | 'feature-config'>('ai-config')

const enabled = ref(false)
const provider = ref('openai')
const baseUrl = ref('')
const apiKey = ref('')
const model = ref('')
const commentModerationEnabled = ref(false)
const articleSummaryEnabled = ref(false)
const commentModerationPrompt = ref('')
const articleSummaryPrompt = ref('')
const promptDialogVisible = ref(false)
const promptDialogSaving = ref(false)
const promptDialogMode = ref<'comment' | 'summary'>('comment')
const promptDialogDraft = ref('')

type AiProviderPreset = {
  description: string
  baseUrl: string
  model: string
}

const providerOptions = [
  { label: 'OpenAI', value: 'openai' },
  { label: 'DeepSeek', value: 'deepseek' },
  { label: 'Xiaomi MiMo', value: 'xiaomi-mimo' },
  { label: 'Azure OpenAI', value: 'azure-openai' },
  { label: 'Anthropic', value: 'anthropic' },
  { label: 'Gemini', value: 'gemini' },
  { label: '自定义', value: 'custom' },
]

const providerPresetMap: Record<string, AiProviderPreset> = {
  openai: {
    description: 'OpenAI 官方 API 预设',
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-4.1-mini',
  },
  deepseek: {
    description: '官方 OpenAI 兼容地址与模型（DeepSeek 首次调用 API 文档）',
    baseUrl: 'https://api.deepseek.com',
    model: 'deepseek-v4-flash',
  },
  'xiaomi-mimo': {
    description: '官方 OpenAI 兼容地址与示例模型（Xiaomi MiMo First API Call 文档）',
    baseUrl: 'https://api.xiaomimimo.com/v1',
    model: 'mimo-v2.5-pro',
  },
  'azure-openai': {
    description: 'Azure OpenAI（请替换资源名；model 建议填写部署名）',
    baseUrl: 'https://YOUR-RESOURCE-NAME.openai.azure.com/openai/v1/',
    model: 'your-deployment-name',
  },
  anthropic: {
    description: 'Anthropic 官方 API 预设',
    baseUrl: 'https://api.anthropic.com/v1',
    model: 'claude-sonnet-4-20250514',
  },
  gemini: {
    description: 'Gemini OpenAI 兼容预设（Google 官方 OpenAI compatibility）',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai/',
    model: 'gemini-2.5-flash',
  },
  custom: {
    description: '自定义服务商模板预设（请按你的网关/平台填写）',
    baseUrl: 'https://api.example.com/v1',
    model: 'your-model-id',
  },
}

const currentProviderPreset = computed<AiProviderPreset | null>(() => {
  return providerPresetMap[provider.value] ?? null
})

const promptDialogTitle = computed(() => {
  return promptDialogMode.value === 'comment' ? 'AI评论审核提示词' : 'AI文章总结提示词'
})

const { showGlobalSuccess, showGlobalError } = useGlobalSnackbar()

onMounted(async () => {
  await loadSettings()
})

function normalizeString(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeBoolean(value: unknown): boolean {
  if (typeof value === 'boolean') {
    return value
  }
  const normalized = normalizeString(value).toLowerCase()
  return normalized === '1' || normalized === 'true' || normalized === 'yes' || normalized === 'on'
}

function getSettingMap(items: AdminSettingItem[]): Map<string, unknown> {
  return new Map(items.map((item) => [item.setting_key, item.setting_content]))
}

async function loadSettings(): Promise<void> {
  loading.value = true
  try {
    const settings = await fetchAdminSettings()
    const settingMap = getSettingMap(settings)

    enabled.value = normalizeBoolean(settingMap.get(KEY_ENABLED))
    provider.value = normalizeString(settingMap.get(KEY_PROVIDER)) || 'openai'
    baseUrl.value = normalizeString(settingMap.get(KEY_BASE_URL))
    apiKey.value = normalizeString(settingMap.get(KEY_API_KEY))
    model.value = normalizeString(settingMap.get(KEY_MODEL))
    commentModerationEnabled.value = normalizeBoolean(settingMap.get(KEY_COMMENT_MODERATION_ENABLED))
    articleSummaryEnabled.value = normalizeBoolean(settingMap.get(KEY_ARTICLE_SUMMARY_ENABLED))

    const legacyPrompt = normalizeString(settingMap.get(LEGACY_SYSTEM_PROMPT_KEY))
    commentModerationPrompt.value = normalizeString(settingMap.get(KEY_COMMENT_MODERATION_PROMPT))
      || legacyPrompt
      || DEFAULT_COMMENT_MODERATION_PROMPT
    articleSummaryPrompt.value = normalizeString(settingMap.get(KEY_ARTICLE_SUMMARY_PROMPT))
      || legacyPrompt
      || DEFAULT_ARTICLE_SUMMARY_PROMPT
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载 AI 配置失败'
    showGlobalError(message)
  } finally {
    loading.value = false
  }
}

function resetForm(): void {
  enabled.value = false
  provider.value = 'openai'
  baseUrl.value = ''
  apiKey.value = ''
  model.value = ''
  commentModerationEnabled.value = false
  articleSummaryEnabled.value = false
  commentModerationPrompt.value = DEFAULT_COMMENT_MODERATION_PROMPT
  articleSummaryPrompt.value = DEFAULT_ARTICLE_SUMMARY_PROMPT
}

function applyProviderPreset(): void {
  const preset = currentProviderPreset.value
  if (!preset) {
    return
  }

  baseUrl.value = preset.baseUrl
  model.value = preset.model
}

function openPromptDialog(mode: 'comment' | 'summary'): void {
  promptDialogMode.value = mode
  promptDialogDraft.value = mode === 'comment'
    ? commentModerationPrompt.value
    : articleSummaryPrompt.value
  promptDialogVisible.value = true
}

function closePromptDialog(): void {
  if (promptDialogSaving.value) {
    return
  }
  promptDialogVisible.value = false
}

async function savePromptDialog(): Promise<void> {
  const content = promptDialogDraft.value.trim()
  const key = promptDialogMode.value === 'comment'
    ? KEY_COMMENT_MODERATION_PROMPT
    : KEY_ARTICLE_SUMMARY_PROMPT
  const fallback = promptDialogMode.value === 'comment'
    ? DEFAULT_COMMENT_MODERATION_PROMPT
    : DEFAULT_ARTICLE_SUMMARY_PROMPT

  const finalContent = content || fallback
  promptDialogSaving.value = true
  try {
    await updateAdminSettings([
      {
        setting_key: key,
        setting_content: finalContent,
        setting_type: 'string',
      },
    ])
    if (promptDialogMode.value === 'comment') {
      commentModerationPrompt.value = finalContent
    } else {
      articleSummaryPrompt.value = finalContent
    }
    promptDialogVisible.value = false
    showGlobalSuccess('提示词已保存')
  } catch (error) {
    const message = error instanceof Error ? error.message : '提示词保存失败'
    showGlobalError(message)
  } finally {
    promptDialogSaving.value = false
  }
}

async function saveSettings(): Promise<void> {
  saving.value = true
  try {
    const items: AdminSettingUpdateItem[] = [
      { setting_key: KEY_ENABLED, setting_content: enabled.value, setting_type: 'boolean' },
      { setting_key: KEY_PROVIDER, setting_content: provider.value.trim(), setting_type: 'string' },
      { setting_key: KEY_BASE_URL, setting_content: baseUrl.value.trim(), setting_type: 'string' },
      { setting_key: KEY_API_KEY, setting_content: apiKey.value.trim(), setting_type: 'string' },
      { setting_key: KEY_MODEL, setting_content: model.value.trim(), setting_type: 'string' },
      {
        setting_key: KEY_COMMENT_MODERATION_ENABLED,
        setting_content: commentModerationEnabled.value,
        setting_type: 'boolean',
      },
      {
        setting_key: KEY_COMMENT_MODERATION_PROMPT,
        setting_content: commentModerationPrompt.value.trim(),
        setting_type: 'string',
      },
      {
        setting_key: KEY_ARTICLE_SUMMARY_ENABLED,
        setting_content: articleSummaryEnabled.value,
        setting_type: 'boolean',
      },
      {
        setting_key: KEY_ARTICLE_SUMMARY_PROMPT,
        setting_content: articleSummaryPrompt.value.trim(),
        setting_type: 'string',
      },
    ]

    await updateAdminSettings(items)
    showGlobalSuccess('AI 配置已保存')
  } catch (error) {
    const message = error instanceof Error ? error.message : '保存 AI 配置失败'
    showGlobalError(message)
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.ai-page {
  max-width: 1080px;
  margin: 0 auto;
  padding: 22px 22px 28px;
  color: var(--admin-text-secondary);
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 18px;
}

.header-text h1 {
  margin: 0;
  font-size: 28px;
  line-height: 1.2;
  color: var(--admin-text-heading);
}

.header-text p {
  margin-top: 8px;
  margin-bottom: 0;
  color: var(--admin-text-faint);
}

.header-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.section-card {
  border: 1px solid var(--admin-border-soft);
  background: var(--admin-card-bg-strong);
}

.tabs-card {
  margin-bottom: 12px;
  border: 1px solid var(--admin-border-soft);
  background: var(--admin-card-bg-strong);
}

.tabs-head {
  padding-inline: 8px;
}

.tab-window {
  display: grid;
  gap: 12px;
}

.preset-row {
  margin-top: 4px;
  padding: 10px 12px;
  border: 1px solid var(--admin-border);
  border-radius: 12px;
  background: var(--admin-overlay-panel-soft);
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
}

.preset-meta {
  display: grid;
  gap: 2px;
}

.preset-title {
  font-size: 13px;
  color: var(--admin-text-secondary);
  font-weight: 600;
}

.preset-desc {
  font-size: 12px;
  color: var(--admin-text-faint);
}

.feature-toggle-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border: 1px solid var(--admin-border-soft);
  border-radius: 12px;
  background: var(--admin-overlay-panel-soft);
}

.feature-switch {
  flex: 1;
}

.section-body {
  display: grid;
  gap: 10px;
}

@media (max-width: 960px) {
  .ai-page {
    padding: 16px 14px 22px;
  }

  .page-header {
    flex-direction: column;
    align-items: stretch;
  }

  .header-actions {
    justify-content: flex-start;
  }

  .preset-row {
    flex-direction: column;
    align-items: flex-start;
  }

  .feature-toggle-row {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
