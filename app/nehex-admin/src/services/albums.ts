import { adminFetch } from '@/services/admin-api'

export type AlbumItem = {
  id: number
  title: string
  cover?: string | null
  class: string
  like_count: number
  img_urls?: string | null
  create_time: string
  update_time: string
}

type AlbumListResponse = {
  data: AlbumItem[]
}

type AlbumDetailResponse = {
  data: AlbumItem
}

export type AlbumUpsertPayload = {
  title: string
  cover?: string | null
  class: string
  like_count: number
  img_urls?: string | null
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchAlbums(): Promise<AlbumItem[]> {
  const response = await adminFetch('/admin-api/albums', {
    method: 'GET',
  })

  const payload = await parseJson<AlbumListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected albums response format')
  }

  return payload.data
}

export async function fetchAlbumById(albumId: number): Promise<AlbumItem> {
  const response = await adminFetch(`/admin-api/albums/${albumId}`, {
    method: 'GET',
  })

  const payload = await parseJson<AlbumDetailResponse>(response)
  if (!payload?.data) {
    throw new Error('Unexpected album detail response format')
  }

  return payload.data
}

export async function createAlbum(payload: AlbumUpsertPayload): Promise<AlbumItem> {
  const response = await adminFetch('/admin-api/albums', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
  const result = await parseJson<AlbumDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected create album response format')
  }
  return result.data
}

export async function updateAlbum(albumId: number, payload: AlbumUpsertPayload): Promise<AlbumItem> {
  const response = await adminFetch(`/admin-api/albums/${albumId}`, {
    method: 'PUT',
    body: JSON.stringify(payload),
  })
  const result = await parseJson<AlbumDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected update album response format')
  }
  return result.data
}

export async function deleteAlbum(albumId: number): Promise<void> {
  await adminFetch(`/admin-api/albums/${albumId}`, {
    method: 'DELETE',
  })
}

export function parseAlbumImageUrls(raw: string | null | undefined): string[] {
  const source = (raw || '').trim()
  if (!source) {
    return []
  }

  const normalized = source.replace(/,/g, '\n')
  const unique = new Set<string>()
  normalized
    .split('\n')
    .map((item) => item.trim())
    .filter(Boolean)
    .forEach((item) => {
      if (!unique.has(item)) {
        unique.add(item)
      }
    })

  return Array.from(unique)
}

export function joinAlbumImageUrls(urls: string[]): string | null {
  const normalized = urls
    .map((item) => item.trim())
    .filter(Boolean)
  if (normalized.length === 0) {
    return null
  }
  return normalized.join('\n')
}
