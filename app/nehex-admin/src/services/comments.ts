import { adminFetch } from '@/services/admin-api'

export type AdminCommentItem = {
  id: number
  parent_id: number
  target_type: string
  target_id: number
  content: string
  nickname: string
  email?: string | null
  website?: string | null
  like_count: number
  status: number
  ip?: string | null
  create_time: string
  update_time: string
  replies: AdminCommentItem[]
}

type AdminCommentListResponse = {
  data: AdminCommentItem[]
  pagination?: {
    page?: number
    size?: number
    total?: number
    total_pages?: number
  }
}

export type AdminCommentListResult = {
  items: AdminCommentItem[]
  pagination: {
    page: number
    size: number
    total: number
    total_pages: number
  }
}

export type AdminCommentUpdatePayload = {
  nickname?: string
  email?: string | null
  website?: string | null
  content?: string
  status?: number
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchAdminComments(
  keyword = '',
  page = 1,
  size = 20,
): Promise<AdminCommentListResult> {
  const normalized = keyword.trim()
  const params = new URLSearchParams()
  if (normalized) {
    params.set('keyword', normalized)
  }
  params.set('page', String(Math.max(1, Math.floor(page))))
  params.set('size', String(Math.max(1, Math.floor(size))))
  const query = params.toString() ? `?${params.toString()}` : ''

  const response = await adminFetch(`/admin-api/comments${query}`, {
    method: 'GET',
  })

  const payload = await parseJson<AdminCommentListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected comment list response format')
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

export async function deleteAdminComment(commentId: number): Promise<void> {
  await adminFetch(`/admin-api/comments/${commentId}`, {
    method: 'DELETE',
  })
}

type AdminCommentDetailResponse = {
  data: AdminCommentItem
}

export async function updateAdminComment(
  commentId: number,
  payload: AdminCommentUpdatePayload,
): Promise<AdminCommentItem> {
  const response = await adminFetch(`/admin-api/comments/${commentId}`, {
    method: 'PUT',
    body: JSON.stringify(payload),
  })
  const result = await parseJson<AdminCommentDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected update comment response format')
  }
  return result.data
}
