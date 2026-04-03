<template>
  <AdminLayout>
    <template #secondary-nav>
      <div class="settings-subnav">
        <div class="subnav-title">站点设置</div>
        <v-list class="subnav-list" density="comfortable" nav>
          <v-list-item
            v-for="item in sections"
            :key="item.key"
            class="subnav-item"
            :active="activeSectionKey === item.key"
            :prepend-icon="item.icon"
            rounded="lg"
            :title="item.label"
            @click="activeSectionKey = item.key"
          />
        </v-list>
      </div>
    </template>

    <section class="settings-page">
      <header class="page-header">
        <div class="header-text">
          <h1>{{ activeSection.label }}</h1>
          <p>{{ activeSection.description }}</p>
        </div>
        <div class="header-actions">
          <v-btn
            prepend-icon="mdi-restore"
            variant="text"
            @click="resetCurrentSection"
          >
            重置当前分组
          </v-btn>
          <v-btn
            color="primary"
            prepend-icon="mdi-content-save-outline"
            :loading="saving"
            @click="saveCurrentSection"
          >
            保存当前分组
          </v-btn>
        </div>
      </header>

      <v-alert
        v-if="errorMessage"
        class="mb-4"
        density="comfortable"
        type="error"
        variant="tonal"
      >
        {{ errorMessage }}
      </v-alert>

      <v-alert
        v-if="successMessage"
        class="mb-4"
        density="comfortable"
        type="success"
        variant="tonal"
      >
        {{ successMessage }}
      </v-alert>

      <v-progress-linear
        v-if="loading"
        class="mb-4"
        color="primary"
        indeterminate
      />

      <v-window v-model="activeSectionKey" :touch="false" class="section-window">
        <v-window-item value="nehex">
          <v-card class="section-card" rounded="xl">
            <v-card-title>NeHex配置</v-card-title>
            <v-card-text>
              <div class="form-grid">
                <v-text-field
                  v-model="nehexForm.siteTitle"
                  label="站点标题（site_title）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="nehexForm.siteSubtitle"
                  label="副标题（site_sub_title）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="nehexForm.apiBase"
                  label="API 基础路径（site_api_base）"
                  variant="outlined"
                />
              </div>

              <div class="class-toolbar">
                <div class="class-toolbar-title">文章分类配置（nehex_article_class）</div>
                <div class="class-toolbar-actions">
                  <v-text-field
                    v-model="newClassValue"
                    class="class-add-input"
                    density="comfortable"
                    hide-details
                    label="分类值"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="newClassLabel"
                    class="class-add-input"
                    density="comfortable"
                    hide-details
                    label="分类名称（可选）"
                    variant="outlined"
                  />
                  <v-btn color="primary" prepend-icon="mdi-plus" @click="addArticleClass">
                    添加分类
                  </v-btn>
                </div>
              </div>

              <div v-if="nehexClasses.length > 0" class="class-list">
                <div
                  v-for="(item, index) in nehexClasses"
                  :key="`class-${index}`"
                  class="class-row"
                >
                  <v-text-field
                    v-model="item.value"
                    density="comfortable"
                    hide-details
                    label="分类值"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="item.label"
                    density="comfortable"
                    hide-details
                    label="分类名称"
                    variant="outlined"
                  />
                  <v-btn
                    color="error"
                    icon="mdi-delete-outline"
                    variant="text"
                    @click="removeArticleClass(index)"
                  />
                </div>
              </div>

              <v-alert
                v-else
                class="mt-2"
                density="comfortable"
                type="info"
                variant="tonal"
              >
                暂无分类，请添加至少一个分类项。
              </v-alert>
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="site">
          <v-card class="section-card" rounded="xl">
            <v-card-title>站点配置</v-card-title>
            <v-card-text>
              <div class="form-grid">
                <v-text-field
                  v-model="siteForm.siteUrl"
                  label="站点地址（site_url）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="siteForm.siteIcp"
                  label="备案信息（site_icp）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="siteForm.siteKeywords"
                  label="关键词（site_keywords）"
                  variant="outlined"
                />
              </div>

              <v-textarea
                v-model="siteForm.siteDescription"
                auto-grow
                label="站点描述（site_description）"
                min-rows="4"
                variant="outlined"
              />

              <v-textarea
                v-model="siteForm.siteNotice"
                auto-grow
                label="公告（site_notice）"
                min-rows="4"
                variant="outlined"
              />
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="theme">
          <v-card class="section-card" rounded="xl">
            <v-card-title>主题设置</v-card-title>
            <v-card-text>
              <div class="theme-file-row">
                <v-select
                  v-model="selectedThemeFile"
                  class="theme-file-select"
                  :items="themeFileOptions"
                  item-title="label"
                  item-value="value"
                  label="主题配置文件"
                  variant="outlined"
                />

                <v-text-field
                  v-model="newThemeFile"
                  class="theme-file-input"
                  label="新增主题文件名"
                  placeholder="my-theme.json"
                  variant="outlined"
                />

                <v-btn color="primary" prepend-icon="mdi-plus" @click="addThemeProfile">
                  新增
                </v-btn>
                <v-btn
                  color="error"
                  prepend-icon="mdi-delete-outline"
                  :disabled="themeProfiles.length <= 1"
                  @click="removeCurrentThemeProfile"
                >
                  删除
                </v-btn>
              </div>

              <div class="form-grid">
                <v-text-field
                  v-model="themeForm.background"
                  label="背景图（theme_background）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="themeForm.primary"
                  label="主色（theme_primary）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="themeForm.banner"
                  label="横幅图（theme_banner）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="themeForm.cardStyle"
                  label="卡片样式（theme_card_style）"
                  variant="outlined"
                />
              </div>

              <div class="theme-preview" :style="themePreviewStyle">
                <div class="theme-preview-mask">
                  <div class="theme-preview-title">主题预览</div>
                  <div class="theme-preview-meta">
                    配置文件: {{ selectedThemeFile || 'default.json' }}
                  </div>
                </div>
              </div>
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="account">
          <v-card class="section-card" rounded="xl">
            <v-card-title>帐号设置</v-card-title>
            <v-card-text>
              <v-alert class="mb-4" density="comfortable" type="warning" variant="tonal">
                修改密码时无需输入哈希，后端会自动计算并保存。
              </v-alert>

              <div class="form-grid">
                <v-text-field
                  v-model="accountForm.account"
                  label="管理员账号（user_account）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="accountForm.newPassword"
                  autocomplete="new-password"
                  label="新密码（可选）"
                  type="password"
                  variant="outlined"
                />
                <v-text-field
                  v-model="accountForm.confirmPassword"
                  autocomplete="new-password"
                  label="确认新密码"
                  type="password"
                  variant="outlined"
                />
              </div>
            </v-card-text>
          </v-card>
        </v-window-item>
      </v-window>
    </section>
  </AdminLayout>
</template>

<script lang="ts" setup>
import { computed, onMounted, reactive, ref, watch, type CSSProperties } from 'vue'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import {
  fetchAdminSettings,
  updateAdminAccountSettings,
  updateAdminSettings,
  type AdminSettingItem,
  type AdminSettingUpdateItem,
} from '@/services/admin-settings'
import { getAuthenticatedAccount } from '@/utils/auth'

type SectionKey = 'nehex' | 'site' | 'theme' | 'account'

type NehexForm = {
  siteTitle: string
  siteSubtitle: string
  apiBase: string
}

type ArticleClassItem = {
  value: string
  label: string
}

type SiteForm = {
  siteUrl: string
  siteDescription: string
  siteKeywords: string
  siteIcp: string
  siteNotice: string
}

type ThemeForm = {
  background: string
  primary: string
  banner: string
  cardStyle: string
}

type ThemeProfile = {
  file: string
  background: string
  primary: string
  banner: string
  cardStyle: string
}

type AccountForm = {
  account: string
  newPassword: string
  confirmPassword: string
}

type SectionMeta = {
  key: SectionKey
  label: string
  icon: string
  description: string
}

type NehexSnapshot = {
  form: NehexForm
  classes: ArticleClassItem[]
  extraConfig: Record<string, unknown>
}

type ThemeSnapshot = {
  profiles: ThemeProfile[]
  selectedFile: string
}

const sections: SectionMeta[] = [
  {
    key: 'nehex',
    label: 'NeHex配置',
    icon: 'mdi-hexagon-multiple-outline',
    description: '维护站点核心配置与内容分类结构。',
  },
  {
    key: 'site',
    label: '站点配置',
    icon: 'mdi-web',
    description: '维护站点地址、描述、关键词与公告文案。',
  },
  {
    key: 'theme',
    label: '主题设置',
    icon: 'mdi-palette-outline',
    description: '按主题配置文件管理多套主题。',
  },
  {
    key: 'account',
    label: '帐号设置',
    icon: 'mdi-account-cog-outline',
    description: '维护后台管理员账号与密码。',
  },
]

const defaultSection: SectionMeta = sections[0]!

const activeSectionKey = ref<SectionKey>('nehex')
const loading = ref(false)
const saving = ref(false)
const errorMessage = ref('')
const successMessage = ref('')

const nehexForm = reactive<NehexForm>({
  siteTitle: '',
  siteSubtitle: '',
  apiBase: '',
})

const nehexClasses = ref<ArticleClassItem[]>([])
const nehexExtraConfig = ref<Record<string, unknown>>({})
const newClassValue = ref('')
const newClassLabel = ref('')

const siteForm = reactive<SiteForm>({
  siteUrl: '',
  siteDescription: '',
  siteKeywords: '',
  siteIcp: '',
  siteNotice: '',
})

const themeForm = reactive<ThemeForm>({
  background: '',
  primary: '',
  banner: '',
  cardStyle: '',
})
const themeProfiles = ref<ThemeProfile[]>([])
const selectedThemeFile = ref('')
const newThemeFile = ref('')

const accountForm = reactive<AccountForm>({
  account: getAuthenticatedAccount(),
  newPassword: '',
  confirmPassword: '',
})

const nehexSnapshot = ref<NehexSnapshot>(getNehexSnapshotData())
const siteSnapshot = ref<SiteForm>(getSiteFormData())
const themeSnapshot = ref<ThemeSnapshot>(getThemeSnapshotData())
const accountSnapshot = ref<AccountForm>(getAccountFormData())

const activeSection = computed<SectionMeta>(() => {
  return sections.find((item) => item.key === activeSectionKey.value) || defaultSection
})

const themeFileOptions = computed(() => {
  return themeProfiles.value.map((item) => ({
    label: item.file,
    value: item.file,
  }))
})

const themePreviewStyle = computed<CSSProperties>(() => {
  const background = themeForm.background.trim()
  if (!background) {
    return {
      background: 'linear-gradient(140deg, #2a3045 0%, #1a2235 100%)',
      backgroundPosition: 'center',
      backgroundRepeat: 'no-repeat',
      backgroundSize: 'cover',
    }
  }

  const safeUrl = background.replace(/"/g, '\\"')
  return {
    backgroundImage: `linear-gradient(180deg, rgba(7, 11, 20, 0.2), rgba(7, 11, 20, 0.7)), url("${safeUrl}")`,
    backgroundPosition: 'center',
    backgroundRepeat: 'no-repeat',
    backgroundSize: 'cover',
  }
})

watch(selectedThemeFile, (next, previous) => {
  if (previous) {
    syncCurrentThemeFormToProfile(previous)
  }

  if (next) {
    loadThemeFormFromProfile(next)
  }
})

function toText(value: unknown): string {
  if (value === null || value === undefined) {
    return ''
  }
  if (typeof value === 'string') {
    return value
  }
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value)
  }

  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

function parseUnknownJson(value: unknown): unknown {
  if (typeof value === 'string') {
    const text = value.trim()
    if (!text) {
      return null
    }

    try {
      return JSON.parse(text)
    } catch {
      return value
    }
  }
  return value
}

function getSettingsMap(items: AdminSettingItem[]): Map<string, unknown> {
  return new Map(items.map((item) => [item.setting_key, item.setting_content]))
}

function readSetting(map: Map<string, unknown>, key: string): string {
  return toText(map.get(key)).trim()
}

function normalizeThemeFileName(raw: string): string {
  const text = raw.trim()
  if (!text) {
    return ''
  }
  if (text.includes('/')) {
    return ''
  }
  if (text.includes('.')) {
    return text
  }
  return `${text}.json`
}

function parseArticleClassPayload(raw: unknown): {
  items: ArticleClassItem[]
  extraConfig: Record<string, unknown>
} {
  const parsed = parseUnknownJson(raw)
  const items: ArticleClassItem[] = []
  const extraConfig: Record<string, unknown> = {}

  if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
    const source = parsed as Record<string, unknown>
    const classValue = source.class

    Object.keys(source).forEach((key) => {
      if (key === 'class') {
        return
      }
      extraConfig[key] = source[key]
    })

    if (classValue && typeof classValue === 'object' && !Array.isArray(classValue)) {
      Object.entries(classValue as Record<string, unknown>).forEach(([value, label]) => {
        const normalizedValue = value.trim()
        if (!normalizedValue) {
          return
        }

        items.push({
          value: normalizedValue,
          label: toText(label).trim() || normalizedValue,
        })
      })
    }
  } else if (typeof parsed === 'string') {
    parsed
      .split(',')
      .map((item) => item.trim())
      .filter(Boolean)
      .forEach((item) => {
        items.push({ value: item, label: item })
      })
  }

  if (items.length === 0) {
    items.push({ value: 'default', label: '默认分类' })
  }

  return {
    items,
    extraConfig,
  }
}

function buildArticleClassSettingContent(): Record<string, unknown> {
  const classMap: Record<string, string> = {}

  nehexClasses.value.forEach((item) => {
    const value = item.value.trim()
    if (!value) {
      return
    }

    classMap[value] = item.label.trim() || value
  })

  return {
    ...nehexExtraConfig.value,
    class: classMap,
  }
}

function parseThemeProfiles(raw: unknown, legacy: ThemeForm): ThemeProfile[] {
  const parsed = parseUnknownJson(raw)

  if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
    const root = parsed as Record<string, unknown>
    const source = (root.themes && typeof root.themes === 'object' && !Array.isArray(root.themes))
      ? root.themes as Record<string, unknown>
      : root

    const profiles = Object.entries(source)
      .map(([file, config]) => {
        if (!config || typeof config !== 'object' || Array.isArray(config)) {
          return null
        }

        const typed = config as Record<string, unknown>
        const normalizedFile = normalizeThemeFileName(file)
        if (!normalizedFile) {
          return null
        }

        return {
          file: normalizedFile,
          background: toText(typed.background).trim(),
          primary: toText(typed.primary).trim(),
          banner: toText(typed.banner).trim(),
          cardStyle: toText(typed.card_style ?? typed.cardStyle).trim(),
        } satisfies ThemeProfile
      })
      .filter((item): item is ThemeProfile => item !== null)

    if (profiles.length > 0) {
      return profiles
    }
  }

  return [
    {
      file: 'default.json',
      background: legacy.background,
      primary: legacy.primary,
      banner: legacy.banner,
      cardStyle: legacy.cardStyle,
    },
  ]
}

function findThemeProfile(file: string): ThemeProfile | undefined {
  return themeProfiles.value.find((item) => item.file === file)
}

function loadThemeFormFromProfile(file: string): void {
  const profile = findThemeProfile(file)
  if (!profile) {
    return
  }

  themeForm.background = profile.background
  themeForm.primary = profile.primary
  themeForm.banner = profile.banner
  themeForm.cardStyle = profile.cardStyle
}

function syncCurrentThemeFormToProfile(file?: string): void {
  const target = findThemeProfile(file || selectedThemeFile.value)
  if (!target) {
    return
  }

  target.background = themeForm.background.trim()
  target.primary = themeForm.primary.trim()
  target.banner = themeForm.banner.trim()
  target.cardStyle = themeForm.cardStyle.trim()
}

function addThemeProfile(): void {
  errorMessage.value = ''
  successMessage.value = ''

  const normalizedFile = normalizeThemeFileName(newThemeFile.value)
  if (!normalizedFile) {
    errorMessage.value = '请输入合法的主题文件名'
    return
  }

  if (themeProfiles.value.some((item) => item.file === normalizedFile)) {
    errorMessage.value = '主题文件已存在'
    return
  }

  syncCurrentThemeFormToProfile()
  themeProfiles.value.push({
    file: normalizedFile,
    background: '',
    primary: '',
    banner: '',
    cardStyle: '',
  })
  selectedThemeFile.value = normalizedFile
  newThemeFile.value = ''
  successMessage.value = `已新增主题配置文件 ${normalizedFile}`
}

function removeCurrentThemeProfile(): void {
  errorMessage.value = ''
  successMessage.value = ''

  if (themeProfiles.value.length <= 1) {
    errorMessage.value = '至少保留一个主题配置文件'
    return
  }

  const current = selectedThemeFile.value
  const index = themeProfiles.value.findIndex((item) => item.file === current)
  if (index < 0) {
    return
  }

  themeProfiles.value.splice(index, 1)
  const next = themeProfiles.value[Math.max(0, index - 1)]
  selectedThemeFile.value = next?.file || themeProfiles.value[0]!.file
  loadThemeFormFromProfile(selectedThemeFile.value)
  successMessage.value = `已删除主题配置文件 ${current}`
}

function addArticleClass(): void {
  errorMessage.value = ''
  successMessage.value = ''

  const value = newClassValue.value.trim()
  if (!value) {
    errorMessage.value = '分类值不能为空'
    return
  }

  if (nehexClasses.value.some((item) => item.value.trim() === value)) {
    errorMessage.value = '分类值已存在'
    return
  }

  nehexClasses.value.push({
    value,
    label: newClassLabel.value.trim() || value,
  })

  newClassValue.value = ''
  newClassLabel.value = ''
}

function removeArticleClass(index: number): void {
  nehexClasses.value.splice(index, 1)
}

function getNehexSnapshotData(): NehexSnapshot {
  return {
    form: {
      siteTitle: nehexForm.siteTitle,
      siteSubtitle: nehexForm.siteSubtitle,
      apiBase: nehexForm.apiBase,
    },
    classes: nehexClasses.value.map((item) => ({ ...item })),
    extraConfig: { ...nehexExtraConfig.value },
  }
}

function getSiteFormData(): SiteForm {
  return {
    siteUrl: siteForm.siteUrl,
    siteDescription: siteForm.siteDescription,
    siteKeywords: siteForm.siteKeywords,
    siteIcp: siteForm.siteIcp,
    siteNotice: siteForm.siteNotice,
  }
}

function getThemeSnapshotData(): ThemeSnapshot {
  syncCurrentThemeFormToProfile()
  return {
    profiles: themeProfiles.value.map((item) => ({ ...item })),
    selectedFile: selectedThemeFile.value,
  }
}

function getAccountFormData(): AccountForm {
  return {
    account: accountForm.account,
    newPassword: accountForm.newPassword,
    confirmPassword: accountForm.confirmPassword,
  }
}

function applyNehexSnapshot(snapshot: NehexSnapshot): void {
  nehexForm.siteTitle = snapshot.form.siteTitle
  nehexForm.siteSubtitle = snapshot.form.siteSubtitle
  nehexForm.apiBase = snapshot.form.apiBase
  nehexClasses.value = snapshot.classes.map((item) => ({ ...item }))
  nehexExtraConfig.value = { ...snapshot.extraConfig }
}

function applySiteFormData(data: SiteForm): void {
  Object.assign(siteForm, data)
}

function applyThemeSnapshot(snapshot: ThemeSnapshot): void {
  themeProfiles.value = snapshot.profiles.map((item) => ({ ...item }))
  selectedThemeFile.value = snapshot.selectedFile || themeProfiles.value[0]?.file || 'default.json'
  loadThemeFormFromProfile(selectedThemeFile.value)
}

function applyAccountFormData(data: AccountForm): void {
  Object.assign(accountForm, data)
}

function updateSnapshots(): void {
  nehexSnapshot.value = getNehexSnapshotData()
  siteSnapshot.value = getSiteFormData()
  themeSnapshot.value = getThemeSnapshotData()
  accountSnapshot.value = getAccountFormData()
}

function applyFormsFromSettings(items: AdminSettingItem[]): void {
  const settingsMap = getSettingsMap(items)

  nehexForm.siteTitle = readSetting(settingsMap, 'site_title')
  nehexForm.siteSubtitle = readSetting(settingsMap, 'site_sub_title')
  nehexForm.apiBase = readSetting(settingsMap, 'site_api_base')

  const parsedClass = parseArticleClassPayload(settingsMap.get('nehex_article_class'))
  nehexClasses.value = parsedClass.items
  nehexExtraConfig.value = parsedClass.extraConfig

  siteForm.siteUrl = readSetting(settingsMap, 'site_url')
  siteForm.siteDescription = readSetting(settingsMap, 'site_description')
  siteForm.siteKeywords = readSetting(settingsMap, 'site_keywords')
  siteForm.siteIcp = readSetting(settingsMap, 'site_icp')
  siteForm.siteNotice = readSetting(settingsMap, 'site_notice')

  const legacyTheme: ThemeForm = {
    background: readSetting(settingsMap, 'theme_background'),
    primary: readSetting(settingsMap, 'theme_primary'),
    banner: readSetting(settingsMap, 'theme_banner'),
    cardStyle: readSetting(settingsMap, 'theme_card_style'),
  }

  themeProfiles.value = parseThemeProfiles(settingsMap.get('theme_profiles'), legacyTheme)
  const activeThemeFile = normalizeThemeFileName(readSetting(settingsMap, 'theme_active_profile'))
  const fallbackThemeFile = themeProfiles.value[0]?.file || 'default.json'
  selectedThemeFile.value = themeProfiles.value.some((item) => item.file === activeThemeFile)
    ? activeThemeFile
    : fallbackThemeFile
  loadThemeFormFromProfile(selectedThemeFile.value)

  accountForm.account = accountForm.account.trim() || getAuthenticatedAccount()
  accountForm.newPassword = ''
  accountForm.confirmPassword = ''
}

async function loadSettings(): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  successMessage.value = ''

  try {
    const items = await fetchAdminSettings()
    applyFormsFromSettings(items)
    updateSnapshots()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '加载设置失败'
  } finally {
    loading.value = false
  }
}

function buildThemeProfilesPayload(): Record<string, unknown> {
  const payload: Record<string, unknown> = {}

  themeProfiles.value.forEach((item) => {
    payload[item.file] = {
      background: item.background,
      primary: item.primary,
      banner: item.banner,
      card_style: item.cardStyle,
    }
  })

  return payload
}

function buildSectionItems(section: SectionKey): AdminSettingUpdateItem[] {
  if (section === 'nehex') {
    return [
      { setting_key: 'site_title', setting_content: nehexForm.siteTitle.trim(), setting_type: 'string' },
      { setting_key: 'site_sub_title', setting_content: nehexForm.siteSubtitle.trim(), setting_type: 'string' },
      { setting_key: 'site_api_base', setting_content: nehexForm.apiBase.trim(), setting_type: 'string' },
      { setting_key: 'nehex_article_class', setting_content: buildArticleClassSettingContent(), setting_type: 'json' },
    ]
  }

  if (section === 'site') {
    return [
      { setting_key: 'site_url', setting_content: siteForm.siteUrl.trim(), setting_type: 'string' },
      { setting_key: 'site_description', setting_content: siteForm.siteDescription, setting_type: 'string' },
      { setting_key: 'site_keywords', setting_content: siteForm.siteKeywords.trim(), setting_type: 'string' },
      { setting_key: 'site_icp', setting_content: siteForm.siteIcp.trim(), setting_type: 'string' },
      { setting_key: 'site_notice', setting_content: siteForm.siteNotice, setting_type: 'string' },
    ]
  }

  if (section === 'theme') {
    syncCurrentThemeFormToProfile()

    const current = findThemeProfile(selectedThemeFile.value)
    const currentTheme = current || {
      background: '',
      primary: '',
      banner: '',
      cardStyle: '',
    }

    return [
      { setting_key: 'theme_active_profile', setting_content: selectedThemeFile.value, setting_type: 'string' },
      { setting_key: 'theme_profiles', setting_content: buildThemeProfilesPayload(), setting_type: 'json' },
      { setting_key: 'theme_background', setting_content: currentTheme.background, setting_type: 'string' },
      { setting_key: 'theme_primary', setting_content: currentTheme.primary, setting_type: 'string' },
      { setting_key: 'theme_banner', setting_content: currentTheme.banner, setting_type: 'string' },
      { setting_key: 'theme_card_style', setting_content: currentTheme.cardStyle, setting_type: 'string' },
    ]
  }

  return []
}

function resetCurrentSection(): void {
  errorMessage.value = ''
  successMessage.value = ''

  const section = activeSectionKey.value
  if (section === 'nehex') {
    applyNehexSnapshot(nehexSnapshot.value)
  } else if (section === 'site') {
    applySiteFormData(siteSnapshot.value)
  } else if (section === 'theme') {
    applyThemeSnapshot(themeSnapshot.value)
  } else if (section === 'account') {
    applyAccountFormData(accountSnapshot.value)
  }

  successMessage.value = `已重置${activeSection.value.label}`
}

async function saveCurrentSection(): Promise<void> {
  errorMessage.value = ''
  successMessage.value = ''
  saving.value = true

  try {
    let updatedItems: AdminSettingItem[] = []
    const section = activeSectionKey.value

    if (section === 'account') {
      const payload: Record<string, string> = {}
      const account = accountForm.account.trim()
      const newPassword = accountForm.newPassword.trim()
      const confirmPassword = accountForm.confirmPassword.trim()

      if (account) {
        payload.account = account
      }

      if (newPassword || confirmPassword) {
        if (!newPassword || !confirmPassword) {
          throw new Error('新密码和确认密码必须同时填写')
        }

        if (newPassword !== confirmPassword) {
          throw new Error('两次输入的新密码不一致')
        }

        payload.new_password = newPassword
        payload.confirm_password = confirmPassword
      }

      if (Object.keys(payload).length === 0) {
        successMessage.value = '无变化，无需保存'
        return
      }

      updatedItems = await updateAdminAccountSettings(payload)
    } else {
      const items = buildSectionItems(section)
      updatedItems = await updateAdminSettings(items)
    }

    applyFormsFromSettings(updatedItems)
    updateSnapshots()
    successMessage.value = `${activeSection.value.label}已保存`
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '保存设置失败'
  } finally {
    saving.value = false
  }
}

onMounted(async () => {
  await loadSettings()
})
</script>

<style scoped>
.settings-subnav {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.subnav-title {
  padding: 8px 8px 0;
  font-size: 16px;
  font-weight: 700;
  color: #f2f5ff;
  letter-spacing: 0.3px;
}

.subnav-list {
  padding: 4px 0 0;
  background: transparent;
}

:deep(.subnav-item) {
  min-height: 42px;
  margin-bottom: 6px;
  color: #b6c3de;
  border: 1px solid transparent;
  transition:
    background 0.2s ease,
    color 0.2s ease;
}

:deep(.subnav-item:hover) {
  color: #eef3ff;
  background: linear-gradient(90deg, rgba(103, 121, 170, 0.14) 0%, rgba(112, 133, 186, 0.24) 100%);
}

:deep(.subnav-item.v-list-item--active) {
  color: #ffffff;
  background: linear-gradient(90deg, rgba(103, 121, 170, 0.28) 0%, rgba(112, 133, 186, 0.42) 100%);
}

.settings-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: 12px;
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
  flex-shrink: 0;
}

.section-window {
  min-width: 0;
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

.class-toolbar {
  margin-top: 8px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.class-toolbar-title {
  font-size: 14px;
  color: #d4ddf5;
}

.class-toolbar-actions {
  display: grid;
  grid-template-columns: 1fr 1fr auto;
  gap: 10px;
  align-items: center;
}

.class-add-input {
  min-width: 0;
}

.class-list {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.class-row {
  display: grid;
  grid-template-columns: 1fr 1fr auto;
  gap: 10px;
  align-items: center;
}

.theme-file-row {
  margin-bottom: 10px;
  display: grid;
  grid-template-columns: 1.2fr 1fr auto auto;
  gap: 10px;
  align-items: center;
}

.theme-file-select,
.theme-file-input {
  min-width: 0;
}

.theme-preview {
  margin-top: 10px;
  border-radius: 14px;
  overflow: hidden;
  min-height: 170px;
  border: 1px solid rgba(255, 255, 255, 0.12);
}

.theme-preview-mask {
  height: 100%;
  min-height: 170px;
  padding: 14px;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  background: linear-gradient(180deg, rgba(8, 12, 22, 0.1), rgba(8, 12, 22, 0.8));
}

.theme-preview-title {
  font-size: 20px;
  font-weight: 700;
  color: #ffffff;
}

.theme-preview-meta {
  margin-top: 6px;
  color: #d8e2fa;
  font-size: 13px;
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

  .theme-file-row,
  .class-toolbar-actions,
  .class-row {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 760px) {
  .form-grid {
    grid-template-columns: 1fr;
  }

  .header-actions {
    width: 100%;
    flex-direction: column;
  }
}
</style>
