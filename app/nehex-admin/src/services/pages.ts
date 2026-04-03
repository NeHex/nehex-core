import { adminFetch } from '@/services/admin-api'

export type StandalonePageItem = {
  id: number
  page_key: string
  title: string
  cover_image?: string | null
  content?: string | null
  sort: number
  status: number
  create_time: string
  update_time: string
}

type StandalonePageListResponse = {
  data: StandalonePageItem[]
}

type StandalonePageDetailResponse = {
  data: StandalonePageItem
}

export type StandalonePageUpsertPayload = {
  page_key: string
  title: string
  cover_image?: string | null
  content?: string | null
  sort: number
  status: number
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchStandalonePages(): Promise<StandalonePageItem[]> {
  const response = await adminFetch('/admin-api/pages', {
    method: 'GET',
  })

  const payload = await parseJson<StandalonePageListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected standalone page response format')
  }

  return payload.data
}

export async function fetchStandalonePageById(pageId: number): Promise<StandalonePageItem> {
  const pages = await fetchStandalonePages()
  const matched = pages.find((item) => item.id === pageId)
  if (!matched) {
    throw new Error('Standalone page not found')
  }
  return matched
}

export async function createStandalonePage(
  payload: StandalonePageUpsertPayload,
): Promise<StandalonePageItem> {
  const response = await adminFetch('/admin-api/pages', {
    method: 'POST',
    body: JSON.stringify(payload),
  })

  const result = await parseJson<StandalonePageDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected create standalone page response format')
  }
  return result.data
}

export async function updateStandalonePage(
  pageId: number,
  payload: StandalonePageUpsertPayload,
): Promise<StandalonePageItem> {
  const response = await adminFetch(`/admin-api/pages/${pageId}`, {
    method: 'PUT',
    body: JSON.stringify(payload),
  })

  const result = await parseJson<StandalonePageDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected update standalone page response format')
  }
  return result.data
}

export async function deleteStandalonePage(pageId: number): Promise<void> {
  await adminFetch(`/admin-api/pages/${pageId}`, {
    method: 'DELETE',
  })
}
