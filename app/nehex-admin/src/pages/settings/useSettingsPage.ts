import { computed, onMounted, reactive, ref, watch } from 'vue'
import {
  fetchAdminSettings,
  testAdminKumaApiUrl,
  updateAdminAccountSettings,
  updateAdminSettings,
  type AdminSettingItem,
  type AdminSettingUpdateItem,
} from '@/services/admin-settings'
import {
  getSettingsMap,
  normalizeThemeFileName,
  parseArticleClassPayload,
  parseDailyClassPayload,
  parseThemeProfileMap,
  readSetting,
  valueToText,
  type ArticleClassItem,
  type ThemeLegacyDefaults,
  type ThemeProfileEntry,
} from '@/pages/settings/helpers'
import { fetchBackendVersion } from '@/services/settings'
import { normalizeBasePath } from '@/utils/path'
import { getAuthenticatedAccount } from '@/utils/auth'

type SectionKey = 'nehex' | 'site' | 'owner' | 'storage' | 'theme'

type NehexForm = {
  adminManagerWeb: string
  adminLoginBackground: string
  kumaApiUrl: string
}

type SiteForm = {
  siteTitle: string
  siteSubtitle: string
  siteUrl: string
  siteKeywords: string
  siteIcp: string
  siteDescription: string
  siteFavicon: string
}

type StorageProvider = 'local' | 'r2' | 's3' | 'aliyun_oss' | 'hi168_s3'

type StorageForm = {
  provider: StorageProvider
  enabled: boolean
  publicBaseUrl: string
  localRoot: string
  localPathRule: string
  r2Endpoint: string
  r2Bucket: string
  r2AccessKeyId: string
  r2SecretAccessKey: string
  r2Region: string
  s3Endpoint: string
  s3Bucket: string
  s3AccessKeyId: string
  s3SecretAccessKey: string
  s3Region: string
  hi168S3Endpoint: string
  hi168S3Bucket: string
  hi168S3AccessKeyId: string
  hi168S3SecretAccessKey: string
  hi168S3Region: string
  aliyunOssEndpoint: string
  aliyunOssBucket: string
  aliyunOssAccessKeyId: string
  aliyunOssSecretAccessKey: string
  aliyunOssRegion: string
}

type AccountForm = {
  account: string
  newPassword: string
  confirmPassword: string
}

type OwnerForm = {
  avatar: string
  nickname: string
  homepage: string
  email: string
  bio: string
}

type SectionMeta = {
  key: SectionKey
  label: string
  icon: string
  description: string
}

type NehexSnapshot = {
  form: NehexForm
  articleClasses: ArticleClassItem[]
  articleExtraConfig: Record<string, unknown>
  dailyClasses: ArticleClassItem[]
  dailyExtraConfig: Record<string, unknown>
  account: string
}

type ThemeSnapshot = {
  profiles: ThemeProfileEntry[]
  selectedFile: string
}

type StorageSnapshot = StorageForm
type OwnerSnapshot = OwnerForm

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
    description: '站点标题、副标题、地址、关键词、ICP备案、描述与 favicon。',
  },
  {
    key: 'owner',
    label: '站长资料',
    icon: 'mdi-account-circle-outline',
    description: '用于前端评论识别站长身份展示，可配置头像、昵称、主页、邮箱与简介。',
  },
  {
    key: 'storage',
    label: '存储设置',
    icon: 'mdi-cloud-upload-outline',
    description: '配置图片上传存储平台（S3对象存储、HI168 S3、阿里云OSS、Cloudflare R2、本机存储）。',
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
const DEFAULT_ADMIN_LOGIN_BACKGROUND = '/images/background-2k.png'
const DEFAULT_STORAGE_LOCAL_ROOT = 'storage'
const DEFAULT_STORAGE_LOCAL_PATH_RULE = '/{year}-{month}/{day}/{random_name}.{file_type}'
const storageProviderOptions: Array<{ label: string, value: StorageProvider }> = [
  { label: 'S3 对象存储（COS/OSS/B2）', value: 's3' },
  { label: 'HI168 S3（强制路径样式）', value: 'hi168_s3' },
  { label: '阿里云 OSS', value: 'aliyun_oss' },
  { label: 'Cloudflare R2', value: 'r2' },
  { label: '本机存储', value: 'local' },
]

const STORAGE_SETTING_KEYS = {
  provider: 'object_storage_provider',
  enabled: 'object_storage_enabled',
  publicBaseUrl: 'object_storage_public_base_url',
  localRoot: 'object_storage_local_root',
  localPathRule: 'object_storage_local_path_rule',
  r2Endpoint: 'object_storage_r2_endpoint',
  r2Bucket: 'object_storage_r2_bucket',
  r2AccessKeyId: 'object_storage_r2_access_key_id',
  r2SecretAccessKey: 'object_storage_r2_secret_access_key',
  r2Region: 'object_storage_r2_region',
  s3Endpoint: 'object_storage_s3_endpoint',
  s3Bucket: 'object_storage_s3_bucket',
  s3AccessKeyId: 'object_storage_s3_access_key_id',
  s3SecretAccessKey: 'object_storage_s3_secret_access_key',
  s3Region: 'object_storage_s3_region',
  hi168S3Endpoint: 'object_storage_hi168_s3_endpoint',
  hi168S3Bucket: 'object_storage_hi168_s3_bucket',
  hi168S3AccessKeyId: 'object_storage_hi168_s3_access_key_id',
  hi168S3SecretAccessKey: 'object_storage_hi168_s3_secret_access_key',
  hi168S3Region: 'object_storage_hi168_s3_region',
  aliyunOssEndpoint: 'object_storage_aliyun_oss_endpoint',
  aliyunOssBucket: 'object_storage_aliyun_oss_bucket',
  aliyunOssAccessKeyId: 'object_storage_aliyun_oss_access_key_id',
  aliyunOssSecretAccessKey: 'object_storage_aliyun_oss_secret_access_key',
  aliyunOssRegion: 'object_storage_aliyun_oss_region',
} as const

const LEGACY_STORAGE_SETTING_KEYS = {
  ossEndpoint: 'object_storage_oss_endpoint',
  ossBucket: 'object_storage_oss_bucket',
  ossAccessKeyId: 'object_storage_oss_access_key_id',
  ossSecretAccessKey: 'object_storage_oss_secret_access_key',
} as const
const REI_THEME_DEFAULT_CONTENT: Record<string, unknown> = {
  head_pic: '/images/head.jpg',
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
  nav_border: {
    关于: '/about',
    友链: '/friends',
    游戏室: '/games',
    travelling: true,
  },
  about_page: {
    welcome: {
      text: 'hi👋 我是',
      name: 'UEGEE',
      desc: '是一个无业游民，一个穷孩子生活在有钱人的城市。',
    },
    map: {
      天津: '117.200983, 39.084158',
      山东: '118.000923, 36.675807',
    },
    slogan: {
      text: '希望',
      main: '我的人生可以早点',
      more: [
        '顺利',
        '暴富',
        '退休',
      ],
    },
    skills: {
      title: '创造,源于热爱',
      programlanguage: [
        'python',
        'vue',
        'nuxt',
        'docker',
        'ubuntu',
        'linux mint',
        'postgresql',
        'redis',
      ],
    },
    education: {
      text: '好好学习,天天向上！————毛泽东',
      university: '山东曲阜师范大学',
      time: '2020/2023',
    },
    visitor_data: {
      title: '访问数据',
      tips: '本站自主统计',
    },
    hobby: [
      'jk',
      'computer',
      'hardware',
      'linux',
    ],
    life_target: {
      text: '人生目标',
      target: {
        not_yet: [
          '拥有一辆自己的汽车',
          '有一份稳定的工作',
          '拥有9950x3d',
          '月均收入达8000',
          '与爱人结婚',
          '有一套属于自己的房子',
          'MacBookPro',
          '活到100岁',
        ],
        finish: [
          '建造属于自己的HomeLab',
          '每年回一次老家2026',
        ],
      },
    },
    wifes_card: {
      'Aihara Enju': {
        cn_name: '蓝原延珠',
        other_name: '藍原（あいはら） 延珠（えんじゅ）',
        image: 'https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Aihara_Enju-half.png',
      },
      'Alisa Mikhailovna Kujō': {
        cn_name: '艾莉莎·米哈伊羅芙娜·九條',
        other_name: 'Алиса Михайловна Кудзё',
        image: 'https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Alisa_Mikhaylovna_Kujō.png',
      },
      'Ijichi Nijika': {
        cn_name: '伊地知虹夏',
        other_name: '伊地知（いじち） 虹夏（にじか）',
        image: 'https://s3.hi168.com/hi168-31358-3621l8yj/wifes/IjichiNijika-half.png',
      },
      Perlica: {
        cn_name: '佩丽卡',
        other_name: 'Perlica',
        image: 'https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Perlica-half.png',
      },
      'Sento Isuzu': {
        cn_name: '千斗五十鈴',
        other_name: 'Isuzuruha Centollusia',
        image: 'https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Sento_Isuzu-half.png',
      },
      'Togawa Sakiko': {
        cn_name: '丰川祥子',
        other_name: '豊川（とがわ） 祥子（さきこ）',
        image: 'https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Togawa Sakiko-top.png',
      },
      'Nao Tomori': {
        cn_name: '友利奈绪',
        other_name: '友利（ともり）  奈緒（なお）',
        image: 'https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Tomori_Nao-half.png',
      },
      'Suō Yuki': {
        cn_name: '周防有希',
        other_name: '周防(すおう) 有希(ゆき)',
        image: 'https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Yuki_Suou_1.png',
      },
      Takagi: {
        cn_name: '高木同学',
        other_name: '高木（たかぎ）',
        image: 'https://s3.hi168.com/hi168-31358-3621l8yj/wifes/takagi3-half.png',
      },
      Zhuangfangyi: {
        cn_name: '庄方宜',
        other_name: 'ZhuangFangYi',
        image: 'https://s3.hi168.com/hi168-31358-3621l8yj/wifes/zhuangfangyi.png',
      },
    },
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
  const defaultNavBorder = defaults.nav_border
  const sourceNavBorder = source.nav_border

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

  if (
    defaultNavBorder
    && typeof defaultNavBorder === 'object'
    && !Array.isArray(defaultNavBorder)
    && sourceNavBorder
    && typeof sourceNavBorder === 'object'
    && !Array.isArray(sourceNavBorder)
  ) {
    source.nav_border = {
      ...(defaultNavBorder as Record<string, unknown>),
      ...(sourceNavBorder as Record<string, unknown>),
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

function normalizeKumaApiUrl(raw: string): string {
  const text = raw.trim()
  if (!text) {
    return ''
  }

  if (text.startsWith('http://') || text.startsWith('https://')) {
    return text
  }

  if (text.startsWith('//')) {
    return `https:${text}`
  }

  return `https://${text.replace(/^\/+/, '')}`
}

function normalizeStorageProvider(raw: string): StorageProvider {
  const normalized = raw.trim().toLowerCase()
  if (normalized === 'r2' || normalized === 's3' || normalized === 'aliyun_oss' || normalized === 'hi168_s3' || normalized === 'local') {
    return normalized
  }
  if (normalized === 'oss') {
    return 's3'
  }
  return 'local'
}

function readSettingWithFallback(settingsMap: Map<string, unknown>, primary: string, ...fallbackKeys: string[]): string {
  const value = readSetting(settingsMap, primary)
  if (value) {
    return value
  }
  for (const key of fallbackKeys) {
    const fallback = readSetting(settingsMap, key)
    if (fallback) {
      return fallback
    }
  }
  return ''
}

function parseBooleanSetting(raw: string, fallback = true): boolean {
  const text = raw.trim().toLowerCase()
  if (!text) {
    return fallback
  }
  return ['1', 'true', 'yes', 'on'].includes(text)
}

export function useSettingsPage() {
  const activeSectionKey = ref<SectionKey>('nehex')
  const loading = ref(false)
  const saving = ref(false)
  const errorMessage = ref('')
  const successMessage = ref('')

  const nehexForm = reactive<NehexForm>({
    adminManagerWeb: '/nehex-admin',
    adminLoginBackground: DEFAULT_ADMIN_LOGIN_BACKGROUND,
    kumaApiUrl: '',
  })

  const nehexClasses = ref<ArticleClassItem[]>([])
  const nehexExtraConfig = ref<Record<string, unknown>>({})
  const newClassValue = ref('')
  const newClassLabel = ref('')
  const nehexDailyClasses = ref<ArticleClassItem[]>([])
  const nehexDailyExtraConfig = ref<Record<string, unknown>>({})
  const newDailyClassValue = ref('')
  const newDailyClassLabel = ref('')

  const siteForm = reactive<SiteForm>({
    siteTitle: '',
    siteSubtitle: '',
    siteUrl: '',
    siteKeywords: '',
    siteIcp: '',
    siteDescription: '',
    siteFavicon: '',
  })

  const ownerForm = reactive<OwnerForm>({
    avatar: '/images/head.jpg',
    nickname: '站长',
    homepage: '',
    email: '',
    bio: '',
  })

  const storageForm = reactive<StorageForm>({
    provider: 'local',
    enabled: true,
    publicBaseUrl: '',
    localRoot: DEFAULT_STORAGE_LOCAL_ROOT,
    localPathRule: DEFAULT_STORAGE_LOCAL_PATH_RULE,
    r2Endpoint: '',
    r2Bucket: '',
    r2AccessKeyId: '',
    r2SecretAccessKey: '',
    r2Region: 'auto',
    s3Endpoint: '',
    s3Bucket: '',
    s3AccessKeyId: '',
    s3SecretAccessKey: '',
    s3Region: '',
    hi168S3Endpoint: '',
    hi168S3Bucket: '',
    hi168S3AccessKeyId: '',
    hi168S3SecretAccessKey: '',
    hi168S3Region: '',
    aliyunOssEndpoint: '',
    aliyunOssBucket: '',
    aliyunOssAccessKeyId: '',
    aliyunOssSecretAccessKey: '',
    aliyunOssRegion: '',
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
  const currentVersion = ref('')
  const kumaApiTesting = ref(false)
  const kumaApiTestResult = ref('')
  const kumaApiTestError = ref('')

  const buildVersion = __NEHEX_ADMIN_VERSION__.trim() || '1.3.1'

  const nehexSnapshot = ref<NehexSnapshot>(getNehexSnapshotData())
  const siteSnapshot = ref<SiteForm>(getSiteFormData())
  const ownerSnapshot = ref<OwnerSnapshot>(getOwnerFormData())
  const storageSnapshot = ref<StorageSnapshot>(getStorageFormData())
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
      return `最新版本 ${latestRelease.value.tagName}，当前版本未知（未从后端 /version 读取到）`
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

  const showLocalStorageFields = computed(() => storageForm.provider === 'local')
  const showR2StorageFields = computed(() => storageForm.provider === 'r2')
  const showS3StorageFields = computed(() => storageForm.provider === 's3')
  const showHi168S3StorageFields = computed(() => storageForm.provider === 'hi168_s3')
  const showAliyunOssStorageFields = computed(() => storageForm.provider === 'aliyun_oss')

  watch(selectedThemeFile, (next, previous) => {
    if (previous) {
      syncThemeEditorToProfile(previous, false)
    }

    if (next) {
      loadThemeEditorFromProfile(next)
    }
  })

  function buildClassSettingContent(
    classes: ArticleClassItem[],
    extraConfig: Record<string, unknown>,
  ): Record<string, unknown> {
    const classMap: Record<string, string> = {}

    classes.forEach((item) => {
      const value = item.value.trim()
      if (!value) {
        return
      }

      classMap[value] = item.label.trim() || value
    })

    return {
      ...extraConfig,
      class: classMap,
    }
  }

  function buildArticleClassSettingContent(): Record<string, unknown> {
    return buildClassSettingContent(nehexClasses.value, nehexExtraConfig.value)
  }

  function buildDailyClassSettingContent(): Record<string, unknown> {
    return buildClassSettingContent(nehexDailyClasses.value, nehexDailyExtraConfig.value)
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

  function addClassOption(
    classItems: ArticleClassItem[],
    valueRef: { value: string },
    labelRef: { value: string },
  ): void {
    errorMessage.value = ''
    successMessage.value = ''

    const value = valueRef.value.trim()
    const label = labelRef.value.trim()

    if (!value) {
      errorMessage.value = '英文值不能为空'
      return
    }

    if (classItems.some((item) => item.value.trim() === value)) {
      errorMessage.value = '英文值已存在'
      return
    }

    classItems.push({
      value,
      label: label || value,
    })

    valueRef.value = ''
    labelRef.value = ''
  }

  function addArticleClass(): void {
    addClassOption(nehexClasses.value, newClassValue, newClassLabel)
  }

  function addDailyClass(): void {
    addClassOption(nehexDailyClasses.value, newDailyClassValue, newDailyClassLabel)
  }

  function removeArticleClass(index: number): void {
    nehexClasses.value.splice(index, 1)
  }

  function removeDailyClass(index: number): void {
    nehexDailyClasses.value.splice(index, 1)
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
        adminLoginBackground: nehexForm.adminLoginBackground,
        kumaApiUrl: nehexForm.kumaApiUrl,
      },
      articleClasses: nehexClasses.value.map((item) => ({ ...item })),
      articleExtraConfig: { ...nehexExtraConfig.value },
      dailyClasses: nehexDailyClasses.value.map((item) => ({ ...item })),
      dailyExtraConfig: { ...nehexDailyExtraConfig.value },
      account: accountForm.account,
    }
  }

  function getSiteFormData(): SiteForm {
    return {
      siteTitle: siteForm.siteTitle,
      siteSubtitle: siteForm.siteSubtitle,
      siteUrl: siteForm.siteUrl,
      siteKeywords: siteForm.siteKeywords,
      siteIcp: siteForm.siteIcp,
      siteDescription: siteForm.siteDescription,
      siteFavicon: siteForm.siteFavicon,
    }
  }

  function getOwnerFormData(): OwnerSnapshot {
    return {
      avatar: ownerForm.avatar,
      nickname: ownerForm.nickname,
      homepage: ownerForm.homepage,
      email: ownerForm.email,
      bio: ownerForm.bio,
    }
  }

  function getStorageFormData(): StorageSnapshot {
    return {
      provider: storageForm.provider,
      enabled: storageForm.enabled,
      publicBaseUrl: storageForm.publicBaseUrl,
      localRoot: storageForm.localRoot,
      localPathRule: storageForm.localPathRule,
      r2Endpoint: storageForm.r2Endpoint,
      r2Bucket: storageForm.r2Bucket,
      r2AccessKeyId: storageForm.r2AccessKeyId,
      r2SecretAccessKey: storageForm.r2SecretAccessKey,
      r2Region: storageForm.r2Region,
      s3Endpoint: storageForm.s3Endpoint,
      s3Bucket: storageForm.s3Bucket,
      s3AccessKeyId: storageForm.s3AccessKeyId,
      s3SecretAccessKey: storageForm.s3SecretAccessKey,
      s3Region: storageForm.s3Region,
      hi168S3Endpoint: storageForm.hi168S3Endpoint,
      hi168S3Bucket: storageForm.hi168S3Bucket,
      hi168S3AccessKeyId: storageForm.hi168S3AccessKeyId,
      hi168S3SecretAccessKey: storageForm.hi168S3SecretAccessKey,
      hi168S3Region: storageForm.hi168S3Region,
      aliyunOssEndpoint: storageForm.aliyunOssEndpoint,
      aliyunOssBucket: storageForm.aliyunOssBucket,
      aliyunOssAccessKeyId: storageForm.aliyunOssAccessKeyId,
      aliyunOssSecretAccessKey: storageForm.aliyunOssSecretAccessKey,
      aliyunOssRegion: storageForm.aliyunOssRegion,
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
    nehexForm.adminLoginBackground = snapshot.form.adminLoginBackground || DEFAULT_ADMIN_LOGIN_BACKGROUND
    nehexForm.kumaApiUrl = snapshot.form.kumaApiUrl || ''
    nehexClasses.value = snapshot.articleClasses.map((item) => ({ ...item }))
    nehexExtraConfig.value = { ...snapshot.articleExtraConfig }
    nehexDailyClasses.value = snapshot.dailyClasses.map((item) => ({ ...item }))
    nehexDailyExtraConfig.value = { ...snapshot.dailyExtraConfig }
    accountForm.account = snapshot.account
    accountForm.newPassword = ''
    accountForm.confirmPassword = ''
  }

  function applySiteFormData(data: SiteForm): void {
    Object.assign(siteForm, data)
  }

  function applyOwnerFormData(data: OwnerSnapshot): void {
    Object.assign(ownerForm, data)
  }

  function applyStorageFormData(data: StorageSnapshot): void {
    Object.assign(storageForm, {
      provider: normalizeStorageProvider(data.provider),
      enabled: !!data.enabled,
      publicBaseUrl: data.publicBaseUrl || '',
      localRoot: data.localRoot || DEFAULT_STORAGE_LOCAL_ROOT,
      localPathRule: data.localPathRule || DEFAULT_STORAGE_LOCAL_PATH_RULE,
      r2Endpoint: data.r2Endpoint || '',
      r2Bucket: data.r2Bucket || '',
      r2AccessKeyId: data.r2AccessKeyId || '',
      r2SecretAccessKey: data.r2SecretAccessKey || '',
      r2Region: data.r2Region || 'auto',
      s3Endpoint: data.s3Endpoint || '',
      s3Bucket: data.s3Bucket || '',
      s3AccessKeyId: data.s3AccessKeyId || '',
      s3SecretAccessKey: data.s3SecretAccessKey || '',
      s3Region: data.s3Region || '',
      hi168S3Endpoint: data.hi168S3Endpoint || '',
      hi168S3Bucket: data.hi168S3Bucket || '',
      hi168S3AccessKeyId: data.hi168S3AccessKeyId || '',
      hi168S3SecretAccessKey: data.hi168S3SecretAccessKey || '',
      hi168S3Region: data.hi168S3Region || '',
      aliyunOssEndpoint: data.aliyunOssEndpoint || '',
      aliyunOssBucket: data.aliyunOssBucket || '',
      aliyunOssAccessKeyId: data.aliyunOssAccessKeyId || '',
      aliyunOssSecretAccessKey: data.aliyunOssSecretAccessKey || '',
      aliyunOssRegion: data.aliyunOssRegion || '',
    })
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
    ownerSnapshot.value = getOwnerFormData()
    storageSnapshot.value = getStorageFormData()
    themeSnapshot.value = getThemeSnapshotData()
  }

  function applyFormsFromSettings(items: AdminSettingItem[]): void {
    const settingsMap = getSettingsMap(items)

    nehexForm.adminManagerWeb = normalizeAdminManagerWebPath(readSetting(settingsMap, 'admin_manager_web') || '/nehex-admin')
    nehexForm.adminLoginBackground = readSetting(settingsMap, 'admin_login_background') || DEFAULT_ADMIN_LOGIN_BACKGROUND
    nehexForm.kumaApiUrl = readSetting(settingsMap, 'kuma_api_url')

    const parsedClass = parseArticleClassPayload(settingsMap.get('nehex_article_class'))
    nehexClasses.value = parsedClass.items
    nehexExtraConfig.value = parsedClass.extraConfig
    const parsedDailyClass = parseDailyClassPayload(settingsMap.get('nehex_daily_class'))
    nehexDailyClasses.value = parsedDailyClass.items
    nehexDailyExtraConfig.value = parsedDailyClass.extraConfig

    siteForm.siteTitle = readSetting(settingsMap, 'site_title')
    siteForm.siteSubtitle = readSetting(settingsMap, 'site_sub_title')
    siteForm.siteUrl = readSetting(settingsMap, 'site_url')
    siteForm.siteKeywords = readSetting(settingsMap, 'site_keywords')
    siteForm.siteIcp = readSetting(settingsMap, 'site_icp')
    siteForm.siteDescription = readSetting(settingsMap, 'site_description')
    siteForm.siteFavicon = readSetting(settingsMap, 'site_favicon')

    ownerForm.avatar = readSetting(settingsMap, 'site_owner_avatar') || '/images/head.jpg'
    ownerForm.nickname = readSetting(settingsMap, 'site_owner_nickname') || '站长'
    ownerForm.homepage = readSetting(settingsMap, 'site_owner_homepage')
    ownerForm.email = readSetting(settingsMap, 'site_owner_email')
    ownerForm.bio = readSetting(settingsMap, 'site_owner_bio')

    storageForm.provider = normalizeStorageProvider(readSetting(settingsMap, STORAGE_SETTING_KEYS.provider))
    storageForm.enabled = parseBooleanSetting(readSetting(settingsMap, STORAGE_SETTING_KEYS.enabled), true)
    storageForm.publicBaseUrl = readSetting(settingsMap, STORAGE_SETTING_KEYS.publicBaseUrl)
    storageForm.localRoot = readSetting(settingsMap, STORAGE_SETTING_KEYS.localRoot) || DEFAULT_STORAGE_LOCAL_ROOT
    storageForm.localPathRule = readSetting(settingsMap, STORAGE_SETTING_KEYS.localPathRule) || DEFAULT_STORAGE_LOCAL_PATH_RULE
    storageForm.r2Endpoint = readSetting(settingsMap, STORAGE_SETTING_KEYS.r2Endpoint)
    storageForm.r2Bucket = readSetting(settingsMap, STORAGE_SETTING_KEYS.r2Bucket)
    storageForm.r2AccessKeyId = readSetting(settingsMap, STORAGE_SETTING_KEYS.r2AccessKeyId)
    storageForm.r2SecretAccessKey = readSetting(settingsMap, STORAGE_SETTING_KEYS.r2SecretAccessKey)
    storageForm.r2Region = readSetting(settingsMap, STORAGE_SETTING_KEYS.r2Region) || 'auto'
    storageForm.s3Endpoint = readSettingWithFallback(
      settingsMap,
      STORAGE_SETTING_KEYS.s3Endpoint,
      LEGACY_STORAGE_SETTING_KEYS.ossEndpoint,
    )
    storageForm.s3Bucket = readSettingWithFallback(
      settingsMap,
      STORAGE_SETTING_KEYS.s3Bucket,
      LEGACY_STORAGE_SETTING_KEYS.ossBucket,
    )
    storageForm.s3AccessKeyId = readSettingWithFallback(
      settingsMap,
      STORAGE_SETTING_KEYS.s3AccessKeyId,
      LEGACY_STORAGE_SETTING_KEYS.ossAccessKeyId,
    )
    storageForm.s3SecretAccessKey = readSettingWithFallback(
      settingsMap,
      STORAGE_SETTING_KEYS.s3SecretAccessKey,
      LEGACY_STORAGE_SETTING_KEYS.ossSecretAccessKey,
    )
    storageForm.s3Region = readSetting(settingsMap, STORAGE_SETTING_KEYS.s3Region)
    storageForm.hi168S3Endpoint = readSetting(settingsMap, STORAGE_SETTING_KEYS.hi168S3Endpoint)
    storageForm.hi168S3Bucket = readSetting(settingsMap, STORAGE_SETTING_KEYS.hi168S3Bucket)
    storageForm.hi168S3AccessKeyId = readSetting(settingsMap, STORAGE_SETTING_KEYS.hi168S3AccessKeyId)
    storageForm.hi168S3SecretAccessKey = readSetting(settingsMap, STORAGE_SETTING_KEYS.hi168S3SecretAccessKey)
    storageForm.hi168S3Region = readSetting(settingsMap, STORAGE_SETTING_KEYS.hi168S3Region)
    storageForm.aliyunOssEndpoint = readSetting(settingsMap, STORAGE_SETTING_KEYS.aliyunOssEndpoint)
    storageForm.aliyunOssBucket = readSetting(settingsMap, STORAGE_SETTING_KEYS.aliyunOssBucket)
    storageForm.aliyunOssAccessKeyId = readSetting(settingsMap, STORAGE_SETTING_KEYS.aliyunOssAccessKeyId)
    storageForm.aliyunOssSecretAccessKey = readSetting(settingsMap, STORAGE_SETTING_KEYS.aliyunOssSecretAccessKey)
    storageForm.aliyunOssRegion = readSetting(settingsMap, STORAGE_SETTING_KEYS.aliyunOssRegion)

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

  async function loadCurrentVersion(): Promise<void> {
    currentVersion.value = buildVersion
    try {
      const backendVersion = await fetchBackendVersion()
      if (backendVersion) {
        currentVersion.value = backendVersion
      }
    } catch {
      // Keep build-version fallback when backend version endpoint is unavailable.
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

  function buildStorageSettingsPayload(): AdminSettingUpdateItem[] {
    const provider = storageForm.provider
    const localRoot = storageForm.localRoot.trim() || DEFAULT_STORAGE_LOCAL_ROOT
    const localPathRule = storageForm.localPathRule.trim() || DEFAULT_STORAGE_LOCAL_PATH_RULE
    const publicBaseUrl = storageForm.publicBaseUrl.trim()
    const r2Endpoint = storageForm.r2Endpoint.trim()
    const r2Bucket = storageForm.r2Bucket.trim()
    const r2AccessKeyId = storageForm.r2AccessKeyId.trim()
    const r2SecretAccessKey = storageForm.r2SecretAccessKey.trim()
    const r2Region = storageForm.r2Region.trim() || 'auto'
    const s3Endpoint = storageForm.s3Endpoint.trim()
    const s3Bucket = storageForm.s3Bucket.trim()
    const s3AccessKeyId = storageForm.s3AccessKeyId.trim()
    const s3SecretAccessKey = storageForm.s3SecretAccessKey.trim()
    const s3Region = storageForm.s3Region.trim()
    const hi168S3Endpoint = storageForm.hi168S3Endpoint.trim()
    const hi168S3Bucket = storageForm.hi168S3Bucket.trim()
    const hi168S3AccessKeyId = storageForm.hi168S3AccessKeyId.trim()
    const hi168S3SecretAccessKey = storageForm.hi168S3SecretAccessKey.trim()
    const hi168S3Region = storageForm.hi168S3Region.trim()
    const aliyunOssEndpoint = storageForm.aliyunOssEndpoint.trim()
    const aliyunOssBucket = storageForm.aliyunOssBucket.trim()
    const aliyunOssAccessKeyId = storageForm.aliyunOssAccessKeyId.trim()
    const aliyunOssSecretAccessKey = storageForm.aliyunOssSecretAccessKey.trim()
    const aliyunOssRegion = storageForm.aliyunOssRegion.trim()

    if (provider === 'r2') {
      if (!r2Endpoint || !r2Bucket || !r2AccessKeyId || !r2SecretAccessKey) {
        throw new Error('Cloudflare R2 配置不完整，请填写 Endpoint、Bucket、AccessKey 和 Secret')
      }
    }

    if (provider === 's3') {
      if (!s3Endpoint || !s3Bucket || !s3AccessKeyId || !s3SecretAccessKey) {
        throw new Error('S3对象存储配置不完整，请填写 Endpoint、Bucket、AccessKey 和 Secret')
      }
    }

    if (provider === 'hi168_s3') {
      if (!hi168S3Endpoint || !hi168S3Bucket || !hi168S3AccessKeyId || !hi168S3SecretAccessKey) {
        throw new Error('HI168 S3 配置不完整，请填写 Endpoint、Bucket、AccessKey 和 Secret')
      }
    }

    if (provider === 'aliyun_oss') {
      if (!aliyunOssEndpoint || !aliyunOssBucket || !aliyunOssAccessKeyId || !aliyunOssSecretAccessKey) {
        throw new Error('阿里云 OSS 配置不完整，请填写 Endpoint、Bucket、AccessKey 和 Secret')
      }
    }

    return [
      { setting_key: STORAGE_SETTING_KEYS.provider, setting_content: provider, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.enabled, setting_content: true, setting_type: 'boolean' },
      { setting_key: STORAGE_SETTING_KEYS.publicBaseUrl, setting_content: publicBaseUrl, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.localRoot, setting_content: localRoot, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.localPathRule, setting_content: localPathRule, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.r2Endpoint, setting_content: r2Endpoint, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.r2Bucket, setting_content: r2Bucket, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.r2AccessKeyId, setting_content: r2AccessKeyId, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.r2SecretAccessKey, setting_content: r2SecretAccessKey, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.r2Region, setting_content: r2Region, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.s3Endpoint, setting_content: s3Endpoint, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.s3Bucket, setting_content: s3Bucket, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.s3AccessKeyId, setting_content: s3AccessKeyId, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.s3SecretAccessKey, setting_content: s3SecretAccessKey, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.s3Region, setting_content: s3Region, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.hi168S3Endpoint, setting_content: hi168S3Endpoint, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.hi168S3Bucket, setting_content: hi168S3Bucket, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.hi168S3AccessKeyId, setting_content: hi168S3AccessKeyId, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.hi168S3SecretAccessKey, setting_content: hi168S3SecretAccessKey, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.hi168S3Region, setting_content: hi168S3Region, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.aliyunOssEndpoint, setting_content: aliyunOssEndpoint, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.aliyunOssBucket, setting_content: aliyunOssBucket, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.aliyunOssAccessKeyId, setting_content: aliyunOssAccessKeyId, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.aliyunOssSecretAccessKey, setting_content: aliyunOssSecretAccessKey, setting_type: 'string' },
      { setting_key: STORAGE_SETTING_KEYS.aliyunOssRegion, setting_content: aliyunOssRegion, setting_type: 'string' },
    ]
  }

  function buildSectionItems(section: SectionKey): AdminSettingUpdateItem[] {
    if (section === 'nehex') {
      return [
        { setting_key: 'admin_manager_web', setting_content: adminManagerWebNormalized.value, setting_type: 'string' },
        {
          setting_key: 'admin_login_background',
          setting_content: nehexForm.adminLoginBackground.trim() || DEFAULT_ADMIN_LOGIN_BACKGROUND,
          setting_type: 'string',
        },
        {
          setting_key: 'kuma_api_url',
          setting_content: nehexForm.kumaApiUrl.trim(),
          setting_type: 'string',
        },
        { setting_key: 'nehex_article_class', setting_content: buildArticleClassSettingContent(), setting_type: 'json' },
        { setting_key: 'nehex_daily_class', setting_content: buildDailyClassSettingContent(), setting_type: 'json' },
      ]
    }

    if (section === 'site') {
      return [
        { setting_key: 'site_title', setting_content: siteForm.siteTitle.trim(), setting_type: 'string' },
        { setting_key: 'site_sub_title', setting_content: siteForm.siteSubtitle.trim(), setting_type: 'string' },
        { setting_key: 'site_url', setting_content: siteForm.siteUrl.trim(), setting_type: 'string' },
        { setting_key: 'site_keywords', setting_content: siteForm.siteKeywords.trim(), setting_type: 'string' },
        { setting_key: 'site_icp', setting_content: siteForm.siteIcp.trim(), setting_type: 'string' },
        { setting_key: 'site_description', setting_content: siteForm.siteDescription, setting_type: 'string' },
        { setting_key: 'site_favicon', setting_content: siteForm.siteFavicon.trim(), setting_type: 'string' },
      ]
    }

    if (section === 'owner') {
      return [
        { setting_key: 'site_owner_avatar', setting_content: ownerForm.avatar.trim(), setting_type: 'string' },
        { setting_key: 'site_owner_nickname', setting_content: ownerForm.nickname.trim(), setting_type: 'string' },
        { setting_key: 'site_owner_homepage', setting_content: ownerForm.homepage.trim(), setting_type: 'string' },
        { setting_key: 'site_owner_email', setting_content: ownerForm.email.trim(), setting_type: 'string' },
        { setting_key: 'site_owner_bio', setting_content: ownerForm.bio, setting_type: 'string' },
      ]
    }

    if (section === 'storage') {
      return buildStorageSettingsPayload()
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
    } else if (section === 'owner') {
      applyOwnerFormData(ownerSnapshot.value)
    } else if (section === 'storage') {
      applyStorageFormData(storageSnapshot.value)
    } else if (section === 'theme') {
      applyThemeSnapshot(themeSnapshot.value)
    }

    successMessage.value = `已重置${activeSection.value.label}`
  }

  async function testKumaApiUrl(): Promise<void> {
    kumaApiTestResult.value = ''
    kumaApiTestError.value = ''

    const target = normalizeKumaApiUrl(nehexForm.kumaApiUrl)
    if (!target) {
      kumaApiTestError.value = '请先输入 Kuma-API 地址'
      return
    }

    kumaApiTesting.value = true
    try {
      const result = await testAdminKumaApiUrl(target)
      nehexForm.kumaApiUrl = result.normalized_url || target
      kumaApiTestResult.value = result.message || '连接成功，Kuma-API 可用'
    } catch (error) {
      kumaApiTestError.value = error instanceof Error ? error.message : '测试失败'
    } finally {
      kumaApiTesting.value = false
    }
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
    await Promise.all([
      loadSettings(),
      loadCurrentVersion(),
    ])
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
    nehexDailyClasses,
    newDailyClassValue,
    newDailyClassLabel,
    accountForm,

    siteForm,
    ownerForm,
    storageForm,
    storageProviderOptions,
    showLocalStorageFields,
    showR2StorageFields,
    showS3StorageFields,
    showHi168S3StorageFields,
    showAliyunOssStorageFields,

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
    kumaApiTesting,
    kumaApiTestResult,
    kumaApiTestError,
    adminManagerWebValidationMessage,
    adminManagerWebHint,

    addThemeProfile,
    cancelCreateThemeProfile,
    confirmCreateThemeProfile,
    handleThemeTemplateSelect,
    removeCurrentThemeProfile,
    formatThemeEditorJson,
    addArticleClass,
    addDailyClass,
    removeArticleClass,
    removeDailyClass,
    checkLatestRelease,
    testKumaApiUrl,
    resetCurrentSection,
    saveCurrentSection,
  }
}
