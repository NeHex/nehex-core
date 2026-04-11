import { ADMIN_API_CLIENT_ID } from '@/services/admin-api'

type StorageUploadData = {
  provider?: unknown
  key?: unknown
  url?: unknown
}

type StorageUploadResponse = {
  data?: StorageUploadData
  detail?: unknown
  message?: unknown
}

export type UploadProgressPayload = {
  loaded: number
  total: number
  percent: number
}

type UploadMarkdownImageOptions = {
  onProgress?: (payload: UploadProgressPayload) => void
}

function clampPercent(value: number): number {
  if (!Number.isFinite(value)) {
    return 0
  }
  return Math.max(0, Math.min(100, Math.round(value)))
}

function normalizeUploadError(status: number, payload: StorageUploadResponse | null): string {
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

function parseJsonSafely(text: string): StorageUploadResponse | null {
  const normalized = text.trim()
  if (!normalized) {
    return null
  }

  try {
    return JSON.parse(normalized) as StorageUploadResponse
  } catch {
    return null
  }
}

export async function uploadMarkdownImage(
  file: File,
  options: UploadMarkdownImageOptions = {},
): Promise<string> {
  const formData = new FormData()
  formData.append('file', file)

  return await new Promise<string>((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open('POST', '/admin-api/storage/upload', true)
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

      const url = String(payload?.data?.url || '').trim()
      if (!url) {
        reject(new Error('上传成功但未返回图片地址'))
        return
      }
      resolve(url)
    }

    xhr.send(formData)
  })
}
