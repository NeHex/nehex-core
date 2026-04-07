import { adminFetch } from '@/services/admin-api'

type AdminBackupListResponse = {
  data: AdminBackupItem[]
}

type AdminBackupDetailResponse = {
  data?: AdminBackupItem
}

type AdminActionResponse = {
  message?: unknown
}

export type AdminBackupItem = {
  filename: string
  size_bytes: number
  created_at: string
  updated_at: string
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchAdminBackups(): Promise<AdminBackupItem[]> {
  const response = await adminFetch('/admin-api/backups', {
    method: 'GET',
  })
  const payload = await parseJson<AdminBackupListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected backup list response format')
  }
  return payload.data
}

export async function createAdminBackup(): Promise<AdminBackupItem> {
  const response = await adminFetch('/admin-api/backups', {
    method: 'POST',
  })
  const payload = await parseJson<AdminBackupDetailResponse>(response)
  if (!payload?.data || typeof payload.data.filename !== 'string') {
    throw new Error('Unexpected backup create response format')
  }
  return payload.data
}

export async function restoreAdminBackup(filename: string): Promise<string> {
  const encoded = encodeURIComponent(filename)
  const response = await adminFetch(`/admin-api/backups/${encoded}/restore`, {
    method: 'POST',
    body: JSON.stringify({
      confirm_overwrite: true,
    }),
  })
  const payload = await parseJson<AdminActionResponse>(response)
  return typeof payload?.message === 'string' ? payload.message : '恢复成功'
}

export async function uploadAndRestoreAdminBackup(file: File): Promise<string> {
  const formData = new FormData()
  formData.append('file', file, file.name)
  formData.append('confirm_overwrite', 'true')

  const response = await adminFetch('/admin-api/backups/upload-restore', {
    method: 'POST',
    body: formData,
  })
  const payload = await parseJson<AdminActionResponse>(response)
  return typeof payload?.message === 'string' ? payload.message : '上传并恢复成功'
}

function resolveDownloadFilename(response: Response, fallback: string): string {
  const contentDisposition = response.headers.get('Content-Disposition') || ''
  const utf8Match = contentDisposition.match(/filename\*=UTF-8''([^;]+)/i)
  if (utf8Match?.[1]) {
    try {
      return decodeURIComponent(utf8Match[1])
    } catch {
      return fallback
    }
  }

  const plainMatch = contentDisposition.match(/filename="?([^";]+)"?/i)
  return plainMatch?.[1] ? plainMatch[1] : fallback
}

export async function downloadAdminBackup(filename: string): Promise<void> {
  const encoded = encodeURIComponent(filename)
  const response = await adminFetch(`/admin-api/backups/${encoded}/download`, {
    method: 'GET',
  })

  const blob = await response.blob()
  const downloadUrl = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = downloadUrl
  link.download = resolveDownloadFilename(response, filename)
  document.body.append(link)
  link.click()
  link.remove()
  URL.revokeObjectURL(downloadUrl)
}
