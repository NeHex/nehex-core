import type { AdminSettingItem } from '@/services/admin-settings'

export type ArticleClassItem = {
  value: string
  label: string
}

export type ThemeLegacyDefaults = {
  background: string
  primary: string
  banner: string
  cardStyle: string
}

export type ThemeProfileEntry = {
  file: string
  content: Record<string, unknown>
}

export function valueToText(value: unknown): string {
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

function cloneRecord(source: Record<string, unknown>): Record<string, unknown> {
  try {
    return JSON.parse(JSON.stringify(source)) as Record<string, unknown>
  } catch {
    return { ...source }
  }
}

export function getSettingsMap(items: AdminSettingItem[]): Map<string, unknown> {
  return new Map(items.map((item) => [item.setting_key, item.setting_content]))
}

export function readSetting(map: Map<string, unknown>, key: string): string {
  return valueToText(map.get(key)).trim()
}

export function normalizeThemeFileName(raw: string): string {
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

export function parseArticleClassPayload(raw: unknown): {
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
          label: valueToText(label).trim() || normalizedValue,
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

export function parseThemeProfileMap(raw: unknown, legacy: ThemeLegacyDefaults): ThemeProfileEntry[] {
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

        const normalizedFile = normalizeThemeFileName(file)
        if (!normalizedFile) {
          return null
        }

        return {
          file: normalizedFile,
          content: cloneRecord(config as Record<string, unknown>),
        } satisfies ThemeProfileEntry
      })
      .filter((item): item is ThemeProfileEntry => item !== null)

    if (profiles.length > 0) {
      return profiles
    }
  }

  return [
    {
      file: 'rei.json',
      content: {
        background_images: legacy.background,
        background: legacy.background,
        primary: legacy.primary,
        banner: legacy.banner,
        card_style: legacy.cardStyle,
      },
    },
  ]
}
