type SettingItem = {
  setting_key: string
  setting_content: unknown
}

type SettingsResponse = {
  data: SettingItem[]
}

export type AdminCredentials = {
  account: string
  passwordHash: string
}

export async function fetchAdminCredentials(): Promise<AdminCredentials> {
  const response = await fetch('/setting', {
    method: 'GET',
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new Error(`Failed to request setting: ${response.status}`)
  }

  const payload = await response.json() as SettingsResponse
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected setting response format')
  }

  const settingsMap = new Map(payload.data.map((item) => [item.setting_key, item.setting_content]))
  const account = String(settingsMap.get('user_account') ?? '').trim()
  const passwordHash = String(settingsMap.get('user_account_password') ?? '').trim().toLowerCase()

  return {
    account,
    passwordHash,
  }
}
