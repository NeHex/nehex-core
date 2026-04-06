import { adminFetch } from '@/services/admin-api'

type StorageUploadData = {
  provider?: unknown
  key?: unknown
  url?: unknown
}

type StorageUploadResponse = {
  data?: StorageUploadData
}

export async function uploadMarkdownImage(file: File): Promise<string> {
  const formData = new FormData()
  formData.append('file', file)

  const response = await adminFetch('/admin-api/storage/upload', {
    method: 'POST',
    body: formData,
  })

  const payload = await response.json() as StorageUploadResponse
  const url = String(payload?.data?.url || '').trim()
  if (!url) {
    throw new Error('上传成功但未返回图片地址')
  }
  return url
}
