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
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchAdminComments(keyword = ''): Promise<AdminCommentItem[]> {
  const normalized = keyword.trim()
  const query = normalized ? `?keyword=${encodeURIComponent(normalized)}` : ''
  const response = await adminFetch(`/admin-api/comments${query}`, {
    method: 'GET',
  })

  const payload = await parseJson<AdminCommentListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected comment list response format')
  }
  return payload.data
}

export async function deleteAdminComment(commentId: number): Promise<void> {
  await adminFetch(`/admin-api/comments/${commentId}`, {
    method: 'DELETE',
  })
}
