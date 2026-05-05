import { adminFetch } from '@/services/admin-api'

export type FriendStatus = 'ok' | 'missing' | 'blocked'
export type FriendApplyStatus = 'pending' | 'approved' | 'rejected' | 'blocked'

export type AdminFriendItem = {
  id: number
  title: string
  description?: string | null
  category: string
  favicon?: string | null
  url: string
  status: FriendStatus
  create_time: string
}

export type AdminFriendApplyItem = {
  id: number
  site_title: string
  site_url: string
  site_description?: string | null
  site_icon?: string | null
  contact?: string | null
  status: FriendApplyStatus
  ip?: string | null
  user_agent?: string | null
  create_time: string
  update_time: string
}

export type AdminFriendUpsertPayload = {
  title: string
  description?: string | null
  category: string
  favicon?: string | null
  url: string
  status: FriendStatus
}

export type AdminFriendApplyStatusUpdatePayload = {
  status: FriendApplyStatus
  create_friend?: boolean
  friend_category?: string | null
}

export type FriendExchangeInfo = {
  site_title: string
  site_url: string
  site_icon: string
  site_description: string
}

type AdminFriendListResponse = {
  data: AdminFriendItem[]
}

type AdminFriendDetailResponse = {
  data: AdminFriendItem
}

type AdminFriendApplyListResponse = {
  data: AdminFriendApplyItem[]
}

type AdminFriendApplyDetailResponse = {
  data: AdminFriendApplyItem
}

type FriendExchangeInfoResponse = {
  data?: Partial<FriendExchangeInfo>
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

function normalizeAdminFriendItem(raw: Partial<AdminFriendItem>): AdminFriendItem | null {
  const id = Number(raw.id)
  const title = String(raw.title || '').trim()
  const category = String(raw.category || 'default').trim() || 'default'
  const url = String(raw.url || '').trim()
  const statusRaw = String(raw.status || '').trim().toLowerCase()
  const status: FriendStatus = statusRaw === 'missing' || statusRaw === 'blocked' ? statusRaw : 'ok'
  const createTime = String(raw.create_time || '').trim()

  if (!Number.isFinite(id) || id <= 0 || !title || !url) {
    return null
  }

  return {
    id,
    title,
    description: raw.description || null,
    category,
    favicon: raw.favicon || null,
    url,
    status,
    create_time: createTime || new Date().toISOString(),
  }
}

export async function fetchAdminFriends(keyword = ''): Promise<AdminFriendItem[]> {
  const normalized = keyword.trim()
  const query = normalized ? `?keyword=${encodeURIComponent(normalized)}` : ''
  const response = await adminFetch(`/admin-api/friends${query}`, {
    method: 'GET',
  })

  const payload = await parseJson<AdminFriendListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected friend list response format')
  }

  return payload.data
    .map((item) => normalizeAdminFriendItem(item))
    .filter((item): item is AdminFriendItem => item !== null)
}

export async function createAdminFriend(
  payload: AdminFriendUpsertPayload,
  options: { overwriteExisting?: boolean } = {},
): Promise<AdminFriendItem> {
  const response = await adminFetch('/admin-api/friends', {
    method: 'POST',
    body: JSON.stringify({
      ...payload,
      overwrite_existing: Boolean(options.overwriteExisting),
    }),
  })

  const result = await parseJson<AdminFriendDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected create friend response format')
  }
  return result.data
}

export async function updateAdminFriend(
  friendId: number,
  payload: Partial<AdminFriendUpsertPayload>,
): Promise<AdminFriendItem> {
  const response = await adminFetch(`/admin-api/friends/${friendId}`, {
    method: 'PUT',
    body: JSON.stringify(payload),
  })

  const result = await parseJson<AdminFriendDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected update friend response format')
  }
  return result.data
}

export async function deleteAdminFriend(friendId: number): Promise<void> {
  await adminFetch(`/admin-api/friends/${friendId}`, {
    method: 'DELETE',
  })
}

export async function fetchAdminFriendApplies(
  options: {
    status?: FriendApplyStatus | ''
    keyword?: string
  } = {},
): Promise<AdminFriendApplyItem[]> {
  const queryParts: string[] = []
  const normalizedStatus = (options.status || '').trim()
  const normalizedKeyword = (options.keyword || '').trim()

  if (normalizedStatus) {
    queryParts.push(`status=${encodeURIComponent(normalizedStatus)}`)
  }
  if (normalizedKeyword) {
    queryParts.push(`keyword=${encodeURIComponent(normalizedKeyword)}`)
  }
  const query = queryParts.length > 0 ? `?${queryParts.join('&')}` : ''

  const response = await adminFetch(`/admin-api/friend-applies${query}`, {
    method: 'GET',
  })

  const payload = await parseJson<AdminFriendApplyListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected friend apply list response format')
  }
  return payload.data
}

export async function updateAdminFriendApplyStatus(
  applyId: number,
  payload: AdminFriendApplyStatusUpdatePayload,
): Promise<AdminFriendApplyItem> {
  const response = await adminFetch(`/admin-api/friend-applies/${applyId}/status`, {
    method: 'PUT',
    body: JSON.stringify(payload),
  })

  const result = await parseJson<AdminFriendApplyDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected friend apply update response format')
  }
  return result.data
}

function normalizeFriendExchangeInfo(raw: Partial<FriendExchangeInfo> | undefined): FriendExchangeInfo {
  return {
    site_title: String(raw?.site_title || '').trim(),
    site_url: String(raw?.site_url || '').trim(),
    site_icon: String(raw?.site_icon || '').trim(),
    site_description: String(raw?.site_description || ''),
  }
}

export async function fetchAdminFriendExchangeInfo(): Promise<FriendExchangeInfo> {
  const response = await adminFetch('/admin-api/friend-exchange-info', {
    method: 'GET',
  })

  const result = await parseJson<FriendExchangeInfoResponse>(response)
  return normalizeFriendExchangeInfo(result?.data)
}

export async function updateAdminFriendExchangeInfo(
  payload: FriendExchangeInfo,
): Promise<FriendExchangeInfo> {
  const response = await adminFetch('/admin-api/friend-exchange-info', {
    method: 'PUT',
    body: JSON.stringify(payload),
  })

  const result = await parseJson<FriendExchangeInfoResponse>(response)
  return normalizeFriendExchangeInfo(result?.data)
}
