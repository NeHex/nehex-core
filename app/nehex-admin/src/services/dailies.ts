import { adminFetch } from '@/services/admin-api'

export type DailyItem = {
  id: number
  title: string
  content?: string | null
  create_time: string
  weather?: string | null
}

type DailyListResponse = {
  data: DailyItem[]
}

type DailyDetailResponse = {
  data: DailyItem
}

export type DailyUpsertPayload = {
  title: string
  content?: string | null
  weather?: string | null
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchDailies(): Promise<DailyItem[]> {
  const response = await fetch('/daily', {
    method: 'GET',
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new Error(`Failed to request dailies: ${response.status}`)
  }

  const payload = await parseJson<DailyListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected dailies response format')
  }

  return payload.data
}

export async function fetchDailyById(dailyId: number): Promise<DailyItem> {
  const response = await adminFetch(`/admin-api/dailies/${dailyId}`, {
    method: 'GET',
  })

  const payload = await parseJson<DailyDetailResponse>(response)
  if (!payload?.data) {
    throw new Error('Unexpected daily detail response format')
  }

  return payload.data
}

export async function createDaily(payload: DailyUpsertPayload): Promise<DailyItem> {
  const response = await adminFetch('/admin-api/dailies', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
  const result = await parseJson<DailyDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected create daily response format')
  }
  return result.data
}

export async function updateDaily(dailyId: number, payload: DailyUpsertPayload): Promise<DailyItem> {
  const response = await adminFetch(`/admin-api/dailies/${dailyId}`, {
    method: 'PUT',
    body: JSON.stringify(payload),
  })
  const result = await parseJson<DailyDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected update daily response format')
  }
  return result.data
}

export async function deleteDaily(dailyId: number): Promise<void> {
  await adminFetch(`/admin-api/dailies/${dailyId}`, {
    method: 'DELETE',
  })
}
