type SettingItem = {
  setting_key: string
  setting_content: unknown
}

type SettingsResponse = {
  data: SettingItem[]
}

type ThemeSettingData = {
  active_profile: string
  profiles: Record<string, Record<string, unknown>>
  current: Record<string, unknown>
}

type ThemeSettingsResponse = {
  data: ThemeSettingData
}

type VersionResponse = {
  version?: unknown
}

export type ArticleClassOption = {
  value: string
  label: string
}

const DEFAULT_ADMIN_TITLE = 'NeHex Admin'
const ADMIN_TITLE_SUFFIX = '后台管理'
const THEME_BACKGROUND_KEY = 'theme_background'
const ADMIN_LOGIN_BACKGROUND_KEY = 'admin_login_background'
const DEFAULT_ADMIN_LOGIN_BACKGROUND = '/images/background-2k.png'
const ARTICLE_CLASS_SETTING_KEY = 'nehex_article_class'
const DAILY_CLASS_SETTING_KEY = 'nehex_daily_class'
const SITE_URL_KEY = 'site_url'
const DEFAULT_ARTICLE_CLASS_OPTIONS: ArticleClassOption[] = [
  {
    value: 'default',
    label: '默认分类',
  },
]
const DEFAULT_DAILY_CLASS_OPTIONS: ArticleClassOption[] = [
  {
    value: 'note',
    label: '日常',
  },
  {
    value: 'review',
    label: '影评',
  },
]

let settingsMapPromise: Promise<Map<string, unknown>> | null = null
let themeSettingsPromise: Promise<ThemeSettingData> | null = null

export function resetSettingsCache(): void {
  settingsMapPromise = null
  themeSettingsPromise = null
}

async function requestSettingsMap(): Promise<Map<string, unknown>> {
  const response = await fetch('/setting', {
    method: 'GET',
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new Error(`Failed to request setting: ${response.status}`)
  }

  const payload = await response.json() as SettingsResponse
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected setting response format')
  }

  return new Map(payload.data.map((item) => [item.setting_key, item.setting_content]))
}

export async function fetchSettingsMap(): Promise<Map<string, unknown>> {
  if (!settingsMapPromise) {
    settingsMapPromise = requestSettingsMap()
  }

  try {
    const settings = await settingsMapPromise
    return new Map(settings)
  } catch (error) {
    settingsMapPromise = null
    throw error
  }
}

async function requestThemeSettings(): Promise<ThemeSettingData> {
  const response = await fetch('/setting/theme', {
    method: 'GET',
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new Error(`Failed to request theme setting: ${response.status}`)
  }

  const payload = await response.json() as ThemeSettingsResponse
  if (!payload?.data || typeof payload.data !== 'object') {
    throw new Error('Unexpected theme setting response format')
  }

  return payload.data
}

async function fetchThemeSettings(): Promise<ThemeSettingData> {
  if (!themeSettingsPromise) {
    themeSettingsPromise = requestThemeSettings()
  }

  try {
    return await themeSettingsPromise
  } catch (error) {
    themeSettingsPromise = null
    throw error
  }
}

export async function fetchAdminTitle(): Promise<string> {
  const settingsMap = await fetchSettingsMap()
  const siteTitle = String(settingsMap.get('site_title') ?? '').trim()
  if (!siteTitle) {
    return DEFAULT_ADMIN_TITLE
  }
  return `${siteTitle} ${ADMIN_TITLE_SUFFIX}`
}

export function getDefaultAdminTitle(): string {
  return DEFAULT_ADMIN_TITLE
}

function normalizeSiteUrl(value: unknown): string {
  const normalized = String(value ?? '').trim().replace(/\/+$/, '')
  if (!normalized) {
    return ''
  }

  if (normalized.startsWith('http://') || normalized.startsWith('https://')) {
    return normalized
  }

  if (normalized.startsWith('/')) {
    return normalized
  }

  return `https://${normalized.replace(/^\/+/, '')}`
}

export async function fetchSiteUrl(): Promise<string> {
  const settingsMap = await fetchSettingsMap()
  return normalizeSiteUrl(settingsMap.get(SITE_URL_KEY))
}

function pickThemeBackground(value: unknown): string {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return ''
  }

  const source = value as Record<string, unknown>
  const backgroundImages = String(source.background_images ?? '').trim()
  if (backgroundImages) {
    return backgroundImages
  }

  return String(source.background ?? '').trim()
}

export async function fetchThemeBackgroundUrl(): Promise<string> {
  try {
    const themeSettings = await fetchThemeSettings()
    const currentBackground = pickThemeBackground(themeSettings.current)
    if (currentBackground) {
      return currentBackground
    }

    const activeProfile = themeSettings.profiles?.[themeSettings.active_profile]
    const activeBackground = pickThemeBackground(activeProfile)
    if (activeBackground) {
      return activeBackground
    }
  } catch {
    // Fallback to legacy /setting API.
  }

  const settingsMap = await fetchSettingsMap()
  return String(settingsMap.get(THEME_BACKGROUND_KEY) ?? '').trim()
}

export async function fetchAdminLoginBackgroundUrl(): Promise<string> {
  const settingsMap = await fetchSettingsMap()
  const value = String(settingsMap.get(ADMIN_LOGIN_BACKGROUND_KEY) ?? '').trim()
  if (value) {
    return value
  }
  return DEFAULT_ADMIN_LOGIN_BACKGROUND
}

export async function fetchBackendVersion(): Promise<string> {
  const response = await fetch('/version', {
    method: 'GET',
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new Error(`Failed to request backend version: ${response.status}`)
  }

  const payload = await response.json() as VersionResponse
  return String(payload?.version ?? '').trim()
}

function parseClassOptionsFromMap(value: unknown): ArticleClassOption[] {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return []
  }

  const classMap = (value as Record<string, unknown>).class
  if (!classMap || typeof classMap !== 'object' || Array.isArray(classMap)) {
    return []
  }

  return Object.entries(classMap as Record<string, unknown>)
    .map(([rawValue, rawLabel]) => {
      const optionValue = rawValue.trim()
      const optionLabel = String(rawLabel ?? '').trim() || optionValue
      return {
        value: optionValue,
        label: optionLabel,
      }
    })
    .filter((item) => item.value)
}

function uniqueClassOptions(options: ArticleClassOption[]): ArticleClassOption[] {
  const optionMap = new Map<string, ArticleClassOption>()

  options.forEach((item) => {
    if (!item.value) {
      return
    }
    if (!optionMap.has(item.value)) {
      optionMap.set(item.value, item)
    }
  })

  return Array.from(optionMap.values())
}

function parseClassOptions(raw: unknown): ArticleClassOption[] {
  if (typeof raw === 'string') {
    const text = raw.trim()
    if (!text) {
      return []
    }

    try {
      const parsed = JSON.parse(text) as unknown
      const parsedOptions = parseClassOptionsFromMap(parsed)
      if (parsedOptions.length > 0) {
        return parsedOptions
      }
    } catch {
      // Keep compatibility with old comma-separated formats.
    }

    return text
      .split(',')
      .map((item) => item.trim())
      .filter(Boolean)
      .map((item) => ({
        value: item,
        label: item,
      }))
  }

  return parseClassOptionsFromMap(raw)
}

export async function fetchArticleClassOptions(): Promise<ArticleClassOption[]> {
  return fetchClassOptionsByKey(ARTICLE_CLASS_SETTING_KEY, DEFAULT_ARTICLE_CLASS_OPTIONS)
}

export async function fetchDailyClassOptions(): Promise<ArticleClassOption[]> {
  return fetchClassOptionsByKey(DAILY_CLASS_SETTING_KEY, DEFAULT_DAILY_CLASS_OPTIONS)
}

async function fetchClassOptionsByKey(
  settingKey: string,
  fallbackOptions: ArticleClassOption[],
): Promise<ArticleClassOption[]> {
  const settingsMap = await fetchSettingsMap()
  const raw = settingsMap.get(settingKey)
  const options = uniqueClassOptions(parseClassOptions(raw))

  if (options.length === 0) {
    return fallbackOptions
  }

  return options
}
