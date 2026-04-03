type SettingItem = {
  setting_key: string
  setting_content: unknown
}

type SettingsResponse = {
  data: SettingItem[]
}

export type AdminCredentials = {
  account: string
  passwordHash: string
}

export type ArticleClassOption = {
  value: string
  label: string
}

const DEFAULT_ADMIN_TITLE = 'NeHex Admin'
const ADMIN_TITLE_SUFFIX = '后台管理'
const THEME_BACKGROUND_KEY = 'theme_background'
const ARTICLE_CLASS_SETTING_KEY = 'nehex_article_class'
const DEFAULT_ARTICLE_CLASS_OPTIONS: ArticleClassOption[] = [
  {
    value: 'default',
    label: '默认分类',
  },
]

let settingsMapPromise: Promise<Map<string, unknown>> | null = null

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

export async function fetchThemeBackgroundUrl(): Promise<string> {
  const settingsMap = await fetchSettingsMap()
  return String(settingsMap.get(THEME_BACKGROUND_KEY) ?? '').trim()
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
  const settingsMap = await fetchSettingsMap()
  const raw = settingsMap.get(ARTICLE_CLASS_SETTING_KEY)
  const options = uniqueClassOptions(parseClassOptions(raw))

  if (options.length === 0) {
    return DEFAULT_ARTICLE_CLASS_OPTIONS
  }

  return options
}

export async function fetchAdminCredentials(): Promise<AdminCredentials> {
  const settingsMap = await fetchSettingsMap()
  const account = String(settingsMap.get('user_account') ?? '').trim()
  const passwordHash = String(settingsMap.get('user_account_password') ?? '').trim().toLowerCase()

  return {
    account,
    passwordHash,
  }
}
