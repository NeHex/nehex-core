import { adminFetch } from '@/services/admin-api'

export type DashboardPeriodKey = 'day' | 'week' | 'month' | 'year'

export type DashboardSeries = {
  labels: string[]
  values: number[]
  total: number
}

export type DashboardPeriodMetrics = Record<DashboardPeriodKey, DashboardSeries>

export type DashboardSiteTotals = {
  text_count: number
  article_count: number
  comment_count: number
  album_count: number
  friend_count: number
}

export type DashboardRecentComment = {
  id: number
  nickname: string
  content: string
  target_type: string
  target_id: number
  status: number
  create_time: string
}

export type DashboardData = {
  visit_ip: DashboardPeriodMetrics
  api_calls: DashboardPeriodMetrics
  site_totals: DashboardSiteTotals
  recent_comments: DashboardRecentComment[]
}

type DashboardResponse = {
  data?: unknown
}

function toNumber(value: unknown): number {
  const normalized = Number(value)
  return Number.isFinite(normalized) ? normalized : 0
}

function normalizeSeries(value: unknown): DashboardSeries {
  const source = value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : {}

  const labelsRaw = Array.isArray(source.labels) ? source.labels : []
  const valuesRaw = Array.isArray(source.values) ? source.values : []

  return {
    labels: labelsRaw.map((item) => String(item ?? '')),
    values: valuesRaw.map((item) => Math.max(0, Math.round(toNumber(item)))),
    total: Math.max(0, Math.round(toNumber(source.total))),
  }
}

function normalizePeriodMetrics(value: unknown): DashboardPeriodMetrics {
  const source = value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : {}

  return {
    day: normalizeSeries(source.day),
    week: normalizeSeries(source.week),
    month: normalizeSeries(source.month),
    year: normalizeSeries(source.year),
  }
}

function normalizeSiteTotals(value: unknown): DashboardSiteTotals {
  const source = value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : {}

  return {
    text_count: Math.max(0, Math.round(toNumber(source.text_count))),
    article_count: Math.max(0, Math.round(toNumber(source.article_count))),
    comment_count: Math.max(0, Math.round(toNumber(source.comment_count))),
    album_count: Math.max(0, Math.round(toNumber(source.album_count))),
    friend_count: Math.max(0, Math.round(toNumber(source.friend_count))),
  }
}

function normalizeRecentComments(value: unknown): DashboardRecentComment[] {
  if (!Array.isArray(value)) {
    return []
  }

  return value.map((item) => {
    const source = item && typeof item === 'object' && !Array.isArray(item)
      ? item as Record<string, unknown>
      : {}

    return {
      id: Math.max(0, Math.round(toNumber(source.id))),
      nickname: String(source.nickname ?? '').trim(),
      content: String(source.content ?? ''),
      target_type: String(source.target_type ?? '').trim(),
      target_id: Math.max(0, Math.round(toNumber(source.target_id))),
      status: Math.max(0, Math.round(toNumber(source.status))),
      create_time: String(source.create_time ?? '').trim(),
    }
  })
}

function normalizeDashboardData(value: unknown): DashboardData {
  const source = value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : {}

  return {
    visit_ip: normalizePeriodMetrics(source.visit_ip),
    api_calls: normalizePeriodMetrics(source.api_calls),
    site_totals: normalizeSiteTotals(source.site_totals),
    recent_comments: normalizeRecentComments(source.recent_comments),
  }
}

export async function fetchDashboardData(): Promise<DashboardData> {
  const response = await adminFetch('/admin-api/dashboard', {
    method: 'GET',
  })
  const payload = await response.json() as DashboardResponse
  return normalizeDashboardData(payload.data)
}
