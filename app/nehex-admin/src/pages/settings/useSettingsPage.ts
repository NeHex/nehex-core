import { computed, onMounted, reactive, ref, watch, type CSSProperties } from 'vue'
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
  parseThemeProfiles,
  readSetting,
  type ArticleClassItem,
  type ThemeForm,
  type ThemeProfile,
} from '@/pages/settings/helpers'
import { getAuthenticatedAccount } from '@/utils/auth'

type SectionKey = 'nehex' | 'site' | 'theme' | 'account'

type NehexForm = {
  siteTitle: string
  siteSubtitle: string
  apiBase: string
}

type SiteForm = {
  siteUrl: string
  siteDescription: string
  siteKeywords: string
  siteIcp: string
  siteNotice: string
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

export function useSettingsPage() {
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

  return {
    sections,
    activeSection,
    activeSectionKey,
    loading,
    saving,
    errorMessage,
    successMessage,
    nehexForm,
    nehexClasses,
    newClassValue,
    newClassLabel,
    siteForm,
    themeForm,
    themeProfiles,
    selectedThemeFile,
    newThemeFile,
    accountForm,
    themeFileOptions,
    themePreviewStyle,
    addThemeProfile,
    removeCurrentThemeProfile,
    addArticleClass,
    removeArticleClass,
    resetCurrentSection,
    saveCurrentSection,
  }
}
