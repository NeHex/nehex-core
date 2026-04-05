import { computed, onMounted, reactive, ref, watch } from 'vue'
import {
  fetchAdminSettings,
  updateAdminAccountSettings,
  updateAdminSettings,
  type AdminSettingItem,
  type AdminSettingUpdateItem,
} from '@/services/admin-settings'
import {
  getSettingsMap,
  normalizeThemeFileName,
  parseArticleClassPayload,
  parseThemeProfileMap,
  readSetting,
  valueToText,
  type ArticleClassItem,
  type ThemeLegacyDefaults,
  type ThemeProfileEntry,
} from '@/pages/settings/helpers'
import { normalizeBasePath } from '@/utils/path'
import { getAuthenticatedAccount } from '@/utils/auth'

type SectionKey = 'nehex' | 'site' | 'theme'

type NehexForm = {
  adminManagerWeb: string
}

type SiteForm = {
  siteTitle: string
  siteSubtitle: string
  siteUrl: string
  siteKeywords: string
  siteDescription: string
  siteFavicon: string
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
  account: string
}

type ThemeSnapshot = {
  profiles: ThemeProfileEntry[]
  selectedFile: string
}

type LatestRelease = {
  tagName: string
  name: string
  htmlUrl: string
  publishedAt: string
}

const sections: SectionMeta[] = [
  {
    key: 'nehex',
    label: 'NeHex配置',
    icon: 'mdi-hexagon-multiple-outline',
    description: '后台地址、分类、更新检测与管理员账号设置。',
  },
  {
    key: 'site',
    label: '网站配置',
    icon: 'mdi-web',
    description: '站点标题、副标题、地址、关键词、描述与 favicon。',
  },
  {
    key: 'theme',
    label: '主题配置',
    icon: 'mdi-code-json',
    description: '选择主题模板并直接编辑 JSON 内容。',
  },
]

const defaultSection: SectionMeta = sections[0]!

const githubLatestReleaseApi = 'https://api.github.com/repos/nehex/nehex-core/releases/latest'
const REI_THEME_FILE = 'rei.json'
const CREATE_THEME_OPTION_VALUE = '__create_theme_template__'
const REI_THEME_DEFAULT_CONTENT: Record<string, unknown> = {
  background_images: '/images/background-2k.png',
  headmsg: 'hi',
  social_link: {
    github: 'https://github.com/nehex',
    bilibili: 'https://space.bilibili.com',
    steam: 'https://steampowered.com',
    music: 'https://music.163.com',
    mail: 'mailto:i@uegee.com',
    feed: true,
  },
}

function toComparableVersionParts(tag: string): number[] | null {
  const text = tag.trim().replace(/^v/i, '')
  if (!text) {
    return null
  }

  const parts = text.split('.')
  if (parts.length === 0) {
    return null
  }

  const numbers: number[] = []
  for (const part of parts) {
    const matched = part.match(/^\d+/)
    if (!matched) {
      break
    }
    numbers.push(Number.parseInt(matched[0], 10))
  }

  if (numbers.length === 0) {
    return null
  }

  while (numbers.length < 3) {
    numbers.push(0)
  }

  return numbers.slice(0, 3)
}

function compareVersionTag(current: string, latest: string): number | null {
  const currentParts = toComparableVersionParts(current)
  const latestParts = toComparableVersionParts(latest)
  if (!currentParts || !latestParts) {
    return null
  }

  for (let i = 0; i < 3; i += 1) {
    if (latestParts[i]! > currentParts[i]!) {
      return 1
    }
    if (latestParts[i]! < currentParts[i]!) {
      return -1
    }
  }

  return 0
}

function cloneProfileEntries(entries: ThemeProfileEntry[]): ThemeProfileEntry[] {
  return entries.map((item) => {
    try {
      return {
        file: item.file,
        content: JSON.parse(JSON.stringify(item.content)) as Record<string, unknown>,
      }
    } catch {
      return {
        file: item.file,
        content: { ...item.content },
      }
    }
  })
}

function createReiThemeContent(): Record<string, unknown> {
  return JSON.parse(JSON.stringify(REI_THEME_DEFAULT_CONTENT)) as Record<string, unknown>
}

function mergeReiThemeContent(content: Record<string, unknown>): Record<string, unknown> {
  const defaults = createReiThemeContent()
  const source = { ...content }
  const defaultSocial = defaults.social_link
  const sourceSocial = source.social_link

  if (
    defaultSocial
    && typeof defaultSocial === 'object'
    && !Array.isArray(defaultSocial)
    && sourceSocial
    && typeof sourceSocial === 'object'
    && !Array.isArray(sourceSocial)
  ) {
    source.social_link = {
      ...(defaultSocial as Record<string, unknown>),
      ...(sourceSocial as Record<string, unknown>),
    }
  }

  return {
    ...defaults,
    ...source,
  }
}

function mergeWithReiTemplate(profiles: ThemeProfileEntry[]): ThemeProfileEntry[] {
  const next = cloneProfileEntries(profiles)
  const rei = next.find((item) => item.file === REI_THEME_FILE)
  if (!rei) {
    next.unshift({
      file: REI_THEME_FILE,
      content: createReiThemeContent(),
    })
    return next
  }

  rei.content = mergeReiThemeContent(rei.content || {})
  const reiIndex = next.findIndex((item) => item.file === REI_THEME_FILE)
  if (reiIndex > 0) {
    const [reiItem] = next.splice(reiIndex, 1)
    next.unshift(reiItem!)
  }

  return next
}

function normalizeAdminManagerWebPath(raw: string): string {
  const normalized = normalizeBasePath(raw || '/nehex-admin')
  if (normalized === '/') {
    return '/nehex-admin'
  }
  return normalized
}

function validateAdminManagerWebPath(raw: string): string {
  const text = (raw || '').trim()
  if (!text) {
    return ''
  }
  if (/\s/.test(text)) {
    return '后台地址不能包含空白字符'
  }
  if (text.includes('?') || text.includes('#')) {
    return '后台地址不能包含 ? 或 #'
  }
  return ''
}

export function useSettingsPage() {
  const activeSectionKey = ref<SectionKey>('nehex')
  const loading = ref(false)
  const saving = ref(false)
  const errorMessage = ref('')
  const successMessage = ref('')

  const nehexForm = reactive<NehexForm>({
    adminManagerWeb: '/nehex-admin',
  })

  const nehexClasses = ref<ArticleClassItem[]>([])
  const nehexExtraConfig = ref<Record<string, unknown>>({})
  const newClassValue = ref('')
  const newClassLabel = ref('')

  const siteForm = reactive<SiteForm>({
    siteTitle: '',
    siteSubtitle: '',
    siteUrl: '',
    siteKeywords: '',
    siteDescription: '',
    siteFavicon: '',
  })

  const accountForm = reactive<AccountForm>({
    account: getAuthenticatedAccount(),
    newPassword: '',
    confirmPassword: '',
  })

  const themeProfiles = ref<ThemeProfileEntry[]>([])
  const selectedThemeFile = ref('')
  const themeCreateDialog = ref(false)
  const themeCreateName = ref('')
  const themeCreateError = ref('')
  const themeEditorJson = ref('{}')
  const themeEditorError = ref('')

  const updateChecking = ref(false)
  const updateCheckError = ref('')
  const latestRelease = ref<LatestRelease | null>(null)
  const currentVersion = ref(
    (
      (import.meta as ImportMeta & { env?: Record<string, string | undefined> }).env
        ?.VITE_NEHEX_CORE_VERSION || ''
    ).trim(),
  )

  const nehexSnapshot = ref<NehexSnapshot>(getNehexSnapshotData())
  const siteSnapshot = ref<SiteForm>(getSiteFormData())
  const themeSnapshot = ref<ThemeSnapshot>(getThemeSnapshotData())

  const activeSection = computed<SectionMeta>(() => {
    return sections.find((item) => item.key === activeSectionKey.value) || defaultSection
  })

  const themeFileOptions = computed(() => {
    const base = themeProfiles.value.map((item) => ({
      label: item.file,
      value: item.file,
    }))
    base.push({
      label: '其他模板（新建）',
      value: CREATE_THEME_OPTION_VALUE,
    })
    return base
  })

  const adminManagerWebValidationMessage = computed(() => {
    return validateAdminManagerWebPath(nehexForm.adminManagerWeb)
  })

  const adminManagerWebNormalized = computed(() => {
    return normalizeAdminManagerWebPath(nehexForm.adminManagerWeb)
  })

  const adminManagerWebHint = computed(() => {
    if (adminManagerWebValidationMessage.value) {
      return ''
    }

    const input = nehexForm.adminManagerWeb.trim() || '/nehex-admin'
    if (input !== adminManagerWebNormalized.value) {
      return `保存时将自动规范为 ${adminManagerWebNormalized.value}`
    }
    return `当前路径 ${adminManagerWebNormalized.value}`
  })

  const hasNewRelease = computed(() => {
    if (!latestRelease.value || !currentVersion.value) {
      return false
    }
    const result = compareVersionTag(currentVersion.value, latestRelease.value.tagName)
    return result === 1
  })

  const releaseStatusText = computed(() => {
    if (!latestRelease.value) {
      return ''
    }

    if (!currentVersion.value) {
      return `最新版本 ${latestRelease.value.tagName}，当前版本未知（可设置 VITE_NEHEX_CORE_VERSION）`
    }

    const result = compareVersionTag(currentVersion.value, latestRelease.value.tagName)
    if (result === 1) {
      return `发现新版本 ${latestRelease.value.tagName}（当前 ${currentVersion.value}）`
    }
    if (result === 0) {
      return `当前已是最新版本 ${currentVersion.value}`
    }
    if (result === -1) {
      return `当前版本 ${currentVersion.value} 高于发布版本 ${latestRelease.value.tagName}`
    }
    return `检测成功：最新版本 ${latestRelease.value.tagName}（当前 ${currentVersion.value}）`
  })

  const canSaveCurrentSection = computed(() => {
    if (activeSectionKey.value === 'nehex' && !!adminManagerWebValidationMessage.value) {
      return false
    }
    return true
  })

  watch(selectedThemeFile, (next, previous) => {
    if (previous) {
      syncThemeEditorToProfile(previous, false)
    }

    if (next) {
      loadThemeEditorFromProfile(next)
    }
  })

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

  function findThemeProfile(file: string): ThemeProfileEntry | undefined {
    return themeProfiles.value.find((item) => item.file === file)
  }

  function loadThemeEditorFromProfile(file: string): void {
    const profile = findThemeProfile(file)
    if (!profile) {
      themeEditorJson.value = '{}'
      return
    }

    themeEditorError.value = ''
    themeEditorJson.value = JSON.stringify(profile.content, null, 2)
  }

  function syncThemeEditorToProfile(file?: string, strict = true): boolean {
    const target = findThemeProfile(file || selectedThemeFile.value)
    if (!target) {
      return true
    }

    const text = themeEditorJson.value.trim()
    if (!text) {
      target.content = {}
      themeEditorError.value = ''
      return true
    }

    try {
      const parsed = JSON.parse(text)
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        throw new Error('主题 JSON 必须是对象')
      }
      target.content = parsed as Record<string, unknown>
      themeEditorError.value = ''
      return true
    } catch (error) {
      themeEditorError.value = error instanceof Error ? error.message : '主题 JSON 格式错误'
      if (strict) {
        throw new Error(`主题 JSON 格式错误：${themeEditorError.value}`)
      }
      return false
    }
  }

  function formatThemeEditorJson(): void {
    syncThemeEditorToProfile(undefined, true)
    loadThemeEditorFromProfile(selectedThemeFile.value)
    successMessage.value = '主题 JSON 已格式化'
  }

  function addThemeProfile(): void {
    errorMessage.value = ''
    successMessage.value = ''
    themeCreateError.value = ''
    themeCreateName.value = ''
    themeCreateDialog.value = true
  }

  function cancelCreateThemeProfile(): void {
    themeCreateDialog.value = false
    themeCreateName.value = ''
    themeCreateError.value = ''
  }

  function confirmCreateThemeProfile(): void {
    themeCreateError.value = ''
    const normalizedFile = normalizeThemeFileName(themeCreateName.value)
    if (!normalizedFile) {
      themeCreateError.value = '请输入合法的主题模板文件名'
      return
    }

    if (themeProfiles.value.some((item) => item.file === normalizedFile)) {
      themeCreateError.value = '主题模板已存在'
      return
    }

    syncThemeEditorToProfile(undefined, false)
    themeProfiles.value.push({
      file: normalizedFile,
      content: createReiThemeContent(),
    })
    selectedThemeFile.value = normalizedFile
    themeCreateDialog.value = false
    themeCreateName.value = ''
    successMessage.value = `已新增主题模板 ${normalizedFile}`
  }

  function handleThemeTemplateSelect(nextValue: string): void {
    if (nextValue === CREATE_THEME_OPTION_VALUE) {
      addThemeProfile()
      return
    }
    selectedThemeFile.value = nextValue
  }

  function removeCurrentThemeProfile(): void {
    errorMessage.value = ''
    successMessage.value = ''

    if (themeProfiles.value.length <= 1) {
      errorMessage.value = '至少保留一个主题模板'
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
    loadThemeEditorFromProfile(selectedThemeFile.value)
    successMessage.value = `已删除主题模板 ${current}`
  }

  function addArticleClass(): void {
    errorMessage.value = ''
    successMessage.value = ''

    const value = newClassValue.value.trim()
    const label = newClassLabel.value.trim()

    if (!value) {
      errorMessage.value = '英文值不能为空'
      return
    }

    if (nehexClasses.value.some((item) => item.value.trim() === value)) {
      errorMessage.value = '英文值已存在'
      return
    }

    nehexClasses.value.push({
      value,
      label: label || value,
    })

    newClassValue.value = ''
    newClassLabel.value = ''
  }

  function removeArticleClass(index: number): void {
    nehexClasses.value.splice(index, 1)
  }

  async function checkLatestRelease(): Promise<void> {
    updateChecking.value = true
    updateCheckError.value = ''

    try {
      const response = await fetch(githubLatestReleaseApi, {
        method: 'GET',
        headers: {
          Accept: 'application/vnd.github+json',
          'X-GitHub-Api-Version': '2022-11-28',
        },
      })

      if (!response.ok) {
        throw new Error(`GitHub API 请求失败 (${response.status})`)
      }

      const data = await response.json() as {
        tag_name?: unknown
        name?: unknown
        html_url?: unknown
        published_at?: unknown
      }

      latestRelease.value = {
        tagName: String(data.tag_name || '').trim() || 'unknown',
        name: String(data.name || '').trim() || 'Untitled Release',
        htmlUrl: String(data.html_url || '').trim(),
        publishedAt: String(data.published_at || '').trim(),
      }
    } catch (error) {
      updateCheckError.value = error instanceof Error ? error.message : '检查更新失败'
    } finally {
      updateChecking.value = false
    }
  }

  function getNehexSnapshotData(): NehexSnapshot {
    return {
      form: {
        adminManagerWeb: nehexForm.adminManagerWeb,
      },
      classes: nehexClasses.value.map((item) => ({ ...item })),
      extraConfig: { ...nehexExtraConfig.value },
      account: accountForm.account,
    }
  }

  function getSiteFormData(): SiteForm {
    return {
      siteTitle: siteForm.siteTitle,
      siteSubtitle: siteForm.siteSubtitle,
      siteUrl: siteForm.siteUrl,
      siteKeywords: siteForm.siteKeywords,
      siteDescription: siteForm.siteDescription,
      siteFavicon: siteForm.siteFavicon,
    }
  }

  function getThemeSnapshotData(): ThemeSnapshot {
    syncThemeEditorToProfile(undefined, false)
    return {
      profiles: cloneProfileEntries(themeProfiles.value),
      selectedFile: selectedThemeFile.value,
    }
  }

  function applyNehexSnapshot(snapshot: NehexSnapshot): void {
    nehexForm.adminManagerWeb = normalizeAdminManagerWebPath(snapshot.form.adminManagerWeb)
    nehexClasses.value = snapshot.classes.map((item) => ({ ...item }))
    nehexExtraConfig.value = { ...snapshot.extraConfig }
    accountForm.account = snapshot.account
    accountForm.newPassword = ''
    accountForm.confirmPassword = ''
  }

  function applySiteFormData(data: SiteForm): void {
    Object.assign(siteForm, data)
  }

  function applyThemeSnapshot(snapshot: ThemeSnapshot): void {
    themeProfiles.value = mergeWithReiTemplate(snapshot.profiles)
    selectedThemeFile.value = snapshot.selectedFile || themeProfiles.value[0]?.file || REI_THEME_FILE
    if (!themeProfiles.value.some((item) => item.file === selectedThemeFile.value)) {
      selectedThemeFile.value = themeProfiles.value[0]?.file || REI_THEME_FILE
    }
    loadThemeEditorFromProfile(selectedThemeFile.value)
  }

  function updateSnapshots(): void {
    nehexSnapshot.value = getNehexSnapshotData()
    siteSnapshot.value = getSiteFormData()
    themeSnapshot.value = getThemeSnapshotData()
  }

  function applyFormsFromSettings(items: AdminSettingItem[]): void {
    const settingsMap = getSettingsMap(items)

    nehexForm.adminManagerWeb = normalizeAdminManagerWebPath(readSetting(settingsMap, 'admin_manager_web') || '/nehex-admin')

    const parsedClass = parseArticleClassPayload(settingsMap.get('nehex_article_class'))
    nehexClasses.value = parsedClass.items
    nehexExtraConfig.value = parsedClass.extraConfig

    siteForm.siteTitle = readSetting(settingsMap, 'site_title')
    siteForm.siteSubtitle = readSetting(settingsMap, 'site_sub_title')
    siteForm.siteUrl = readSetting(settingsMap, 'site_url')
    siteForm.siteKeywords = readSetting(settingsMap, 'site_keywords')
    siteForm.siteDescription = readSetting(settingsMap, 'site_description')
    siteForm.siteFavicon = readSetting(settingsMap, 'site_favicon')

    const legacyTheme: ThemeLegacyDefaults = {
      background: readSetting(settingsMap, 'theme_background'),
      primary: readSetting(settingsMap, 'theme_primary'),
      banner: readSetting(settingsMap, 'theme_banner'),
      cardStyle: readSetting(settingsMap, 'theme_card_style'),
    }

    themeProfiles.value = mergeWithReiTemplate(parseThemeProfileMap(settingsMap.get('theme_profiles'), legacyTheme))
    const activeThemeFile = normalizeThemeFileName(readSetting(settingsMap, 'theme_active_profile'))
    const fallbackThemeFile = themeProfiles.value[0]?.file || REI_THEME_FILE
    selectedThemeFile.value = themeProfiles.value.some((item) => item.file === activeThemeFile)
      ? activeThemeFile
      : fallbackThemeFile
    loadThemeEditorFromProfile(selectedThemeFile.value)

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

  function getThemeLegacyFields(content: Record<string, unknown>): {
    background: string
    primary: string
    banner: string
    cardStyle: string
  } {
    return {
      background: valueToText(content.background).trim(),
      primary: valueToText(content.primary).trim(),
      banner: valueToText(content.banner).trim(),
      cardStyle: valueToText(content.card_style ?? content.cardStyle).trim(),
    }
  }

  function buildThemeProfilesPayload(): Record<string, unknown> {
    const payload: Record<string, unknown> = {}

    themeProfiles.value.forEach((item) => {
      payload[item.file] = item.content
    })

    return payload
  }

  function buildSectionItems(section: SectionKey): AdminSettingUpdateItem[] {
    if (section === 'nehex') {
      return [
        { setting_key: 'admin_manager_web', setting_content: adminManagerWebNormalized.value, setting_type: 'string' },
        { setting_key: 'nehex_article_class', setting_content: buildArticleClassSettingContent(), setting_type: 'json' },
      ]
    }

    if (section === 'site') {
      return [
        { setting_key: 'site_title', setting_content: siteForm.siteTitle.trim(), setting_type: 'string' },
        { setting_key: 'site_sub_title', setting_content: siteForm.siteSubtitle.trim(), setting_type: 'string' },
        { setting_key: 'site_url', setting_content: siteForm.siteUrl.trim(), setting_type: 'string' },
        { setting_key: 'site_keywords', setting_content: siteForm.siteKeywords.trim(), setting_type: 'string' },
        { setting_key: 'site_description', setting_content: siteForm.siteDescription, setting_type: 'string' },
        { setting_key: 'site_favicon', setting_content: siteForm.siteFavicon.trim(), setting_type: 'string' },
      ]
    }

    syncThemeEditorToProfile(undefined, true)
    const current = findThemeProfile(selectedThemeFile.value)
    const legacy = getThemeLegacyFields(current?.content || {})

    return [
      { setting_key: 'theme_active_profile', setting_content: selectedThemeFile.value, setting_type: 'string' },
      { setting_key: 'theme_profiles', setting_content: buildThemeProfilesPayload(), setting_type: 'json' },
      { setting_key: 'theme_background', setting_content: legacy.background, setting_type: 'string' },
      { setting_key: 'theme_primary', setting_content: legacy.primary, setting_type: 'string' },
      { setting_key: 'theme_banner', setting_content: legacy.banner, setting_type: 'string' },
      { setting_key: 'theme_card_style', setting_content: legacy.cardStyle, setting_type: 'string' },
    ]
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
    }

    successMessage.value = `已重置${activeSection.value.label}`
  }

  async function saveCurrentSection(): Promise<void> {
    errorMessage.value = ''
    successMessage.value = ''
    saving.value = true

    try {
      const section = activeSectionKey.value

      if (section === 'nehex') {
        if (adminManagerWebValidationMessage.value) {
          throw new Error(adminManagerWebValidationMessage.value)
        }
        nehexForm.adminManagerWeb = adminManagerWebNormalized.value
        const updatedSettings = await updateAdminSettings(buildSectionItems('nehex'))

        const accountPayload: Record<string, string> = {}
        const account = accountForm.account.trim()
        const oldAccount = nehexSnapshot.value.account.trim()
        const newPassword = accountForm.newPassword.trim()
        const confirmPassword = accountForm.confirmPassword.trim()

        if (account && account !== oldAccount) {
          accountPayload.account = account
        }

        if (newPassword || confirmPassword) {
          if (!newPassword || !confirmPassword) {
            throw new Error('新密码和确认密码必须同时填写')
          }
          if (newPassword !== confirmPassword) {
            throw new Error('两次输入的新密码不一致')
          }
          accountPayload.new_password = newPassword
          accountPayload.confirm_password = confirmPassword
        }

        if (Object.keys(accountPayload).length > 0) {
          await updateAdminAccountSettings(accountPayload)
        }

        applyFormsFromSettings(updatedSettings)
      } else {
        const updated = await updateAdminSettings(buildSectionItems(section))
        applyFormsFromSettings(updated)
      }

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

  return {
    sections,
    activeSection,
    activeSectionKey,
    loading,
    saving,
    errorMessage,
    successMessage,
    canSaveCurrentSection,

    nehexForm,
    nehexClasses,
    newClassValue,
    newClassLabel,
    accountForm,

    siteForm,

    themeProfiles,
    selectedThemeFile,
    themeCreateDialog,
    themeCreateName,
    themeCreateError,
    themeFileOptions,
    themeEditorJson,
    themeEditorError,

    updateChecking,
    updateCheckError,
    latestRelease,
    currentVersion,
    hasNewRelease,
    releaseStatusText,
    adminManagerWebValidationMessage,
    adminManagerWebHint,

    addThemeProfile,
    cancelCreateThemeProfile,
    confirmCreateThemeProfile,
    handleThemeTemplateSelect,
    removeCurrentThemeProfile,
    formatThemeEditorJson,
    addArticleClass,
    removeArticleClass,
    checkLatestRelease,
    resetCurrentSection,
    saveCurrentSection,
  }
}
