import { ADMIN_API_CLIENT_ID, adminFetch } from '@/services/admin-api'

export type MediaFolderItem = {
  id: number
  name: string
  image_count: number
  create_time: string
  update_time: string
}

export type MediaImageItem = {
  id: number
  folder_id: number | null
  media_type: 'image' | 'video' | 'audio' | 'file'
  provider: string
  key: string
  url: string
  file_name: string | null
  content_type: string | null
  size_bytes: number
  create_time: string
}

export type MediaLibraryData = {
  folders: MediaFolderItem[]
  uncategorized: MediaImageItem[]
}

type MediaLibraryResponse = {
  data?: {
    folders?: unknown
    uncategorized?: unknown
  }
}

type MediaFolderDetailResponse = {
  data?: unknown
}

type MediaImageListResponse = {
  data?: unknown
}

type MediaImageDetailResponse = {
  data?: unknown
  detail?: unknown
  message?: unknown
}

type UploadProgressPayload = {
  loaded: number
  total: number
  percent: number
}

type UploadMediaImageOptions = {
  onProgress?: (payload: UploadProgressPayload) => void
}

function normalizeMediaFolderItem(raw: unknown): MediaFolderItem {
  const source = (raw && typeof raw === 'object') ? raw as Record<string, unknown> : {}
  return {
    id: Number(source.id) || 0,
    name: String(source.name || '').trim(),
    image_count: Math.max(0, Number(source.image_count) || 0),
    create_time: String(source.create_time || ''),
    update_time: String(source.update_time || ''),
  }
}

function normalizeMediaImageItem(raw: unknown): MediaImageItem {
  const source = (raw && typeof raw === 'object') ? raw as Record<string, unknown> : {}
  const folderId = Number(source.folder_id)
  const mediaTypeRaw = String(source.media_type || '').trim().toLowerCase()
  const mediaType = mediaTypeRaw === 'image' || mediaTypeRaw === 'video' || mediaTypeRaw === 'audio'
    ? mediaTypeRaw
    : 'file'
  return {
    id: Number(source.id) || 0,
    folder_id: Number.isFinite(folderId) && folderId > 0 ? folderId : null,
    media_type: mediaType,
    provider: String(source.provider || '').trim(),
    key: String(source.key || '').trim(),
    url: String(source.url || '').trim(),
    file_name: source.file_name == null ? null : String(source.file_name).trim(),
    content_type: source.content_type == null ? null : String(source.content_type).trim(),
    size_bytes: Math.max(0, Number(source.size_bytes) || 0),
    create_time: String(source.create_time || ''),
  }
}

function normalizeImageArray(raw: unknown): MediaImageItem[] {
  if (!Array.isArray(raw)) {
    return []
  }

  return raw
    .map((item) => normalizeMediaImageItem(item))
    .filter((item) => item.id > 0 && item.url)
}

function normalizeFolderArray(raw: unknown): MediaFolderItem[] {
  if (!Array.isArray(raw)) {
    return []
  }

  return raw
    .map((item) => normalizeMediaFolderItem(item))
    .filter((item) => item.id > 0 && item.name)
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchMediaLibrary(): Promise<MediaLibraryData> {
  const response = await adminFetch('/admin-api/media/library', {
    method: 'GET',
  })
  const payload = await parseJson<MediaLibraryResponse>(response)
  return {
    folders: normalizeFolderArray(payload?.data?.folders),
    uncategorized: normalizeImageArray(payload?.data?.uncategorized),
  }
}

export async function fetchMediaImagesByFolder(folderId: number): Promise<MediaImageItem[]> {
  const response = await adminFetch(`/admin-api/media/folders/${folderId}/images`, {
    method: 'GET',
  })
  const payload = await parseJson<MediaImageListResponse>(response)
  return normalizeImageArray(payload?.data)
}

export async function createMediaFolder(name: string): Promise<MediaFolderItem> {
  const response = await adminFetch('/admin-api/media/folders', {
    method: 'POST',
    body: JSON.stringify({ name }),
  })
  const payload = await parseJson<MediaFolderDetailResponse>(response)
  const item = normalizeMediaFolderItem(payload?.data)
  if (item.id <= 0 || !item.name) {
    throw new Error('Unexpected create media folder response format')
  }
  return item
}

export async function renameMediaFolder(folderId: number, name: string): Promise<MediaFolderItem> {
  const response = await adminFetch(`/admin-api/media/folders/${folderId}`, {
    method: 'PUT',
    body: JSON.stringify({ name }),
  })
  const payload = await parseJson<MediaFolderDetailResponse>(response)
  const item = normalizeMediaFolderItem(payload?.data)
  if (item.id <= 0 || !item.name) {
    throw new Error('Unexpected rename media folder response format')
  }
  return item
}

export async function deleteMediaFolder(folderId: number): Promise<void> {
  await adminFetch(`/admin-api/media/folders/${folderId}`, {
    method: 'DELETE',
  })
}

export async function moveMediaImages(imageIds: number[], folderId: number | null): Promise<void> {
  const ids = Array.from(new Set(imageIds.map((item) => Number(item)).filter((item) => Number.isFinite(item) && item > 0)))
  if (ids.length <= 0) {
    return
  }

  await adminFetch('/admin-api/media/images/move', {
    method: 'POST',
    body: JSON.stringify({
      ids,
      folder_id: folderId,
    }),
  })
}

export async function deleteMediaImage(imageId: number): Promise<void> {
  await adminFetch(`/admin-api/media/images/${imageId}`, {
    method: 'DELETE',
  })
}

function clampPercent(value: number): number {
  if (!Number.isFinite(value)) {
    return 0
  }
  return Math.max(0, Math.min(100, Math.round(value)))
}

function normalizeUploadError(status: number, payload: MediaImageDetailResponse | null): string {
  const detailArray = Array.isArray(payload?.detail) ? payload.detail : null
  if (detailArray && detailArray.length > 0) {
    const first = detailArray[0] as {
      msg?: unknown
      loc?: unknown
    }
    const message = typeof first?.msg === 'string' ? first.msg : ''
    const location = Array.isArray(first?.loc)
      ? first.loc.map((item) => String(item)).join('.')
      : ''
    if (message && location) {
      return `${location}: ${message}`
    }
    if (message) {
      return message
    }
  }

  const detail = typeof payload?.detail === 'string' ? payload.detail : ''
  const message = typeof payload?.message === 'string' ? payload.message : ''
  const base = detail || message
  return base || `上传失败（${status}）`
}

function parseJsonSafely(text: string): MediaImageDetailResponse | null {
  const normalized = text.trim()
  if (!normalized) {
    return null
  }

  try {
    return JSON.parse(normalized) as MediaImageDetailResponse
  } catch {
    return null
  }
}

export async function uploadMediaImage(
  file: File,
  options: UploadMediaImageOptions = {},
): Promise<MediaImageItem> {
  const formData = new FormData()
  formData.append('file', file)

  return await new Promise<MediaImageItem>((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open('POST', '/admin-api/media/images/upload', true)
    xhr.withCredentials = true
    xhr.setRequestHeader('X-NeHex-Admin-Client', ADMIN_API_CLIENT_ID)

    xhr.upload.onprogress = (event: ProgressEvent<EventTarget>) => {
      if (!options.onProgress || !event.lengthComputable || event.total <= 0) {
        return
      }
      options.onProgress({
        loaded: event.loaded,
        total: event.total,
        percent: clampPercent((event.loaded / event.total) * 100),
      })
    }

    xhr.onerror = () => {
      reject(new Error('上传失败，请检查网络连接'))
    }

    xhr.onload = () => {
      const payload = parseJsonSafely(xhr.responseText || '')
      if (xhr.status < 200 || xhr.status >= 300) {
        reject(new Error(normalizeUploadError(xhr.status, payload)))
        return
      }

      if (options.onProgress) {
        options.onProgress({
          loaded: file.size,
          total: file.size,
          percent: 100,
        })
      }

      const item = normalizeMediaImageItem(payload?.data)
      if (item.id <= 0 || !item.url) {
        reject(new Error('上传成功但响应数据无效'))
        return
      }
      resolve(item)
    }

    xhr.send(formData)
  })
}
