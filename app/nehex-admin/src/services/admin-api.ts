const ADMIN_API_CLIENT_ID = 'nehex-vuetify-admin'

type ApiErrorPayload = {
  detail?: unknown
  message?: unknown
}

function normalizeApiError(response: Response, payload: ApiErrorPayload | null): string {
  const detail = typeof payload?.detail === 'string' ? payload.detail : ''
  const message = typeof payload?.message === 'string' ? payload.message : ''
  const base = detail || message
  return base || `Request failed: ${response.status}`
}

export async function adminFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const headers = new Headers(init.headers)
  headers.set('X-NeHex-Admin-Client', ADMIN_API_CLIENT_ID)

  if (init.body && !headers.has('Content-Type')) {
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

export async function adminLogin(account: string, password: string): Promise<void> {
  await adminFetch('/admin-api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ account, password }),
  })
}
