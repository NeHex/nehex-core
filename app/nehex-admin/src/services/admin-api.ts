const ADMIN_API_CLIENT_ID = 'nehex-vuetify-admin'

type AdminLoginResponse = {
  data?: {
    account?: unknown
    expires_at?: unknown
  }
}

type AdminSessionResponse = {
  data?: {
    account?: unknown
    expires_at?: unknown
  }
}

export type AdminSession = {
  account: string
  expires_at: string
}

type ApiErrorPayload = {
  detail?: unknown
  message?: unknown
}

let adminSessionCache: AdminSession | null = null
let adminSessionPromise: Promise<AdminSession> | null = null

function normalizeApiError(response: Response, payload: ApiErrorPayload | null): string {
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
  return base || `Request failed: ${response.status}`
}

export async function adminFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const headers = new Headers(init.headers)
  headers.set('X-NeHex-Admin-Client', ADMIN_API_CLIENT_ID)

  const body = init.body
  const shouldSetJsonContentType = !!body
    && !(body instanceof FormData)
    && !(body instanceof URLSearchParams)
    && !(body instanceof Blob)
    && !headers.has('Content-Type')

  if (shouldSetJsonContentType) {
    headers.set('Content-Type', 'application/json')
  }

  const response = await fetch(path, {
    ...init,
    credentials: 'same-origin',
    headers,
  })

  if (response.ok) {
    return response
  }

  let payload: ApiErrorPayload | null = null
  try {
    payload = await response.json() as ApiErrorPayload
  } catch {
    payload = null
  }

  throw new Error(normalizeApiError(response, payload))
}

export async function adminLogin(account: string, password: string): Promise<string> {
  const response = await adminFetch('/admin-api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ account, password }),
  })

  const payload = await response.json() as AdminLoginResponse
  const normalizedAccount = typeof payload?.data?.account === 'string'
    ? payload.data.account.trim()
    : ''
  const normalizedExpiresAt = typeof payload?.data?.expires_at === 'string'
    ? payload.data.expires_at.trim()
    : ''
  const effectiveAccount = normalizedAccount || account.trim()

  adminSessionCache = {
    account: effectiveAccount,
    expires_at: normalizedExpiresAt,
  }
  adminSessionPromise = null

  return effectiveAccount
}

export async function adminLogout(): Promise<void> {
  try {
    await adminFetch('/admin-api/auth/logout', {
      method: 'POST',
    })
  } finally {
    resetAdminSessionCache()
  }
}

export function resetAdminSessionCache(): void {
  adminSessionCache = null
  adminSessionPromise = null
}

function normalizeAdminSession(payload: AdminSessionResponse): AdminSession {
  const account = typeof payload?.data?.account === 'string' ? payload.data.account.trim() : ''
  const expiresAt = typeof payload?.data?.expires_at === 'string' ? payload.data.expires_at.trim() : ''

  if (!account) {
    throw new Error('Invalid admin session response')
  }

  return {
    account,
    expires_at: expiresAt,
  }
}

async function requestAdminSession(): Promise<AdminSession> {
  const response = await adminFetch('/admin-api/auth/me', {
    method: 'GET',
  })
  const payload = await response.json() as AdminSessionResponse
  return normalizeAdminSession(payload)
}

export async function fetchAdminSession(force = false): Promise<AdminSession> {
  if (force) {
    resetAdminSessionCache()
  }

  if (adminSessionCache) {
    return { ...adminSessionCache }
  }

  if (!adminSessionPromise) {
    adminSessionPromise = requestAdminSession()
  }

  try {
    adminSessionCache = await adminSessionPromise
    return { ...adminSessionCache }
  } finally {
    adminSessionPromise = null
  }
}
