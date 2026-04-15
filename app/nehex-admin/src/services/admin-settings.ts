import { adminFetch } from '@/services/admin-api'

export type SettingType = 'string' | 'int' | 'float' | 'boolean' | 'json'

export type AdminSettingItem = {
  setting_key: string
  setting_type: SettingType
  setting_content: unknown
  description?: string | null
  updated_at: string
  created_at: string
}

export type AdminSettingUpdateItem = {
  setting_key: string
  setting_content: unknown
  setting_type?: SettingType
  description?: string | null
}

export type AdminAccountSettingsUpdatePayload = {
  account?: string | null
  new_password?: string | null
  confirm_password?: string | null
}

export type AdminKumaApiTestResult = {
  success: boolean
  message: string
  normalized_url: string
  response_preview: string
}

type AdminSettingListResponse = {
  data: AdminSettingItem[]
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchAdminSettings(): Promise<AdminSettingItem[]> {
  const response = await adminFetch('/admin-api/settings', {
    method: 'GET',
  })

  const payload = await parseJson<AdminSettingListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected admin settings response format')
  }
  return payload.data
}

export async function updateAdminSettings(items: AdminSettingUpdateItem[]): Promise<AdminSettingItem[]> {
  const response = await adminFetch('/admin-api/settings', {
    method: 'PUT',
    body: JSON.stringify({ items }),
  })

  const payload = await parseJson<AdminSettingListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected admin settings update response format')
  }
  return payload.data
}

export async function updateAdminAccountSettings(
  payload: AdminAccountSettingsUpdatePayload,
): Promise<AdminSettingItem[]> {
  const response = await adminFetch('/admin-api/settings/account', {
    method: 'PUT',
    body: JSON.stringify(payload),
  })

  const result = await parseJson<AdminSettingListResponse>(response)
  if (!Array.isArray(result?.data)) {
    throw new Error('Unexpected admin account settings response format')
  }
  return result.data
}

export async function testAdminKumaApiUrl(url: string): Promise<AdminKumaApiTestResult> {
  const response = await adminFetch('/admin-api/settings/kuma-api/test', {
    method: 'POST',
    body: JSON.stringify({ url }),
  })

  const payload = await parseJson<AdminKumaApiTestResult>(response)
  if (!payload || typeof payload !== 'object') {
    throw new Error('Unexpected kuma api test response format')
  }
  return payload
}
