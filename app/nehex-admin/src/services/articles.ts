import { adminFetch } from '@/services/admin-api'

export type ArticleItem = {
  id: number
  title: string
  articleTopImage?: string | null
  class: string
  read: number
  lastEditTime: string
  tag?: string | null
  top: number
  content?: string | null
}

type ArticleListResponse = {
  data: ArticleItem[]
}

type ArticleDetailResponse = {
  data: ArticleItem
}

export type ArticleUpsertPayload = {
  title: string
  articleTopImage?: string | null
  class: string
  read: number
  tag?: string | null
  top: number
  content?: string | null
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchArticles(): Promise<ArticleItem[]> {
  const response = await fetch('/article', {
    method: 'GET',
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new Error(`Failed to request articles: ${response.status}`)
  }

  const payload = await parseJson<ArticleListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected article response format')
  }

  return payload.data
}

export async function fetchArticleById(articleId: number): Promise<ArticleItem> {
  const response = await fetch(`/article/${articleId}`, {
    method: 'GET',
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new Error(`Failed to request article detail: ${response.status}`)
  }

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
