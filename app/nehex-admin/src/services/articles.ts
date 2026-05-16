import { adminFetch } from '@/services/admin-api'

export type ArticleItem = {
  id: number
  title: string
  articleTopImage?: string | null
  aiSummary?: string | null
  class: string
  read: number
  like_count: number
  lastEditTime: string
  tag?: string | null
  top: number
  status: number
  content?: string | null
}

type ArticleListResponse = {
  data: ArticleItem[]
  pagination?: {
    page?: number
    size?: number
    total?: number
    total_pages?: number
  }
}

type ArticleDetailResponse = {
  data: ArticleItem
}

export type ArticleUpsertPayload = {
  title: string
  articleTopImage?: string | null
  aiSummary?: string | null
  class: string
  read: number
  like_count?: number
  tag?: string | null
  top: number
  status: number
  content?: string | null
}

export type ArticleListResult = {
  items: ArticleItem[]
  pagination: {
    page: number
    size: number
    total: number
    total_pages: number
  }
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchArticles(
  page = 1,
  size = 24,
): Promise<ArticleListResult> {
  const params = new URLSearchParams()
  params.set('page', String(Math.max(1, Math.floor(page))))
  params.set('size', String(Math.max(1, Math.floor(size))))

  const response = await adminFetch(`/admin-api/articles?${params.toString()}`, {
    method: 'GET',
  })

  const payload = await parseJson<ArticleListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected article response format')
  }

  const safePage = Number.isFinite(payload.pagination?.page) ? Number(payload.pagination?.page) : Math.max(1, Math.floor(page))
  const safeSize = Number.isFinite(payload.pagination?.size) ? Number(payload.pagination?.size) : Math.max(1, Math.floor(size))
  const safeTotal = Number.isFinite(payload.pagination?.total) ? Number(payload.pagination?.total) : payload.data.length
  const safeTotalPages = Number.isFinite(payload.pagination?.total_pages)
    ? Number(payload.pagination?.total_pages)
    : Math.max(0, Math.ceil(safeTotal / safeSize))

  return {
    items: payload.data,
    pagination: {
      page: safePage,
      size: safeSize,
      total: Math.max(0, safeTotal),
      total_pages: Math.max(0, safeTotalPages),
    },
  }
}

export async function fetchArticleById(articleId: number): Promise<ArticleItem> {
  const response = await adminFetch(`/admin-api/articles/${articleId}`, {
    method: 'GET',
  })

  const payload = await parseJson<ArticleDetailResponse>(response)
  if (!payload?.data) {
    throw new Error('Unexpected article detail response format')
  }

  return payload.data
}

export async function createArticle(payload: ArticleUpsertPayload): Promise<ArticleItem> {
  const response = await adminFetch('/admin-api/articles', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
  const result = await parseJson<ArticleDetailResponse>(response)
  return result.data
}

export async function updateArticle(articleId: number, payload: ArticleUpsertPayload): Promise<ArticleItem> {
  const response = await adminFetch(`/admin-api/articles/${articleId}`, {
    method: 'PUT',
    body: JSON.stringify(payload),
  })
  const result = await parseJson<ArticleDetailResponse>(response)
  return result.data
}

export async function deleteArticle(articleId: number): Promise<void> {
  await adminFetch(`/admin-api/articles/${articleId}`, {
    method: 'DELETE',
  })
}
