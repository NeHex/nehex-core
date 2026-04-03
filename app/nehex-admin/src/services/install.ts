import { adminFetch } from '@/services/admin-api'

export type InstallStatus = {
  installed: boolean
  schema_ready: boolean
  table_count: number
  admin_manager_web: string
}

export type InstallArticleClassItem = {
  value: string
  label?: string
}

export type InstallPayload = {
  admin: {
    account: string
    password: string
    confirm_password: string
    admin_manager_web: string
  }
  nehex: {
    site_title?: string
    site_sub_title?: string
    site_api_base?: string
    article_classes: InstallArticleClassItem[]
  }
  site: {
    site_url?: string
    site_description?: string
    site_keywords?: string
    site_icp?: string
    site_notice?: string
  }
}

type InstallStatusResponse = {
  data: InstallStatus
}

type InstallSubmitResponse = {
  data: InstallStatus
  message?: string
}

let installStatusCache: InstallStatus | null = null
let installStatusPromise: Promise<InstallStatus> | null = null

function normalizeInstallStatus(raw: unknown): InstallStatus {
  const source = raw as Partial<InstallStatus> | null
  return {
    installed: Boolean(source?.installed),
    schema_ready: Boolean(source?.schema_ready),
    table_count: Number.isFinite(source?.table_count) ? Number(source?.table_count) : 0,
    admin_manager_web: String(source?.admin_manager_web || '/nehex-admin').trim() || '/nehex-admin',
  }
}

export function resetInstallStatusCache(): void {
  installStatusCache = null
  installStatusPromise = null
}

async function requestInstallStatus(): Promise<InstallStatus> {
  const response = await adminFetch('/admin-api/install/status', {
    method: 'GET',
  })

  const payload = await response.json() as InstallStatusResponse
  if (!payload || typeof payload !== 'object') {
    throw new Error('Unexpected install status response format')
  }

  return normalizeInstallStatus(payload.data)
}

export async function fetchInstallStatus(force = false): Promise<InstallStatus> {
  if (force) {
    resetInstallStatusCache()
  }

  if (installStatusCache) {
    return { ...installStatusCache }
  }

  if (!installStatusPromise) {
    installStatusPromise = requestInstallStatus()
  }

  try {
    installStatusCache = await installStatusPromise
    return { ...installStatusCache }
  } finally {
    installStatusPromise = null
  }
}

export async function submitInstall(payload: InstallPayload): Promise<InstallStatus> {
  const response = await adminFetch('/admin-api/install', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
  const result = await response.json() as InstallSubmitResponse
  const statusData = normalizeInstallStatus(result?.data)
  installStatusCache = statusData
  installStatusPromise = null
  return { ...statusData }
}
