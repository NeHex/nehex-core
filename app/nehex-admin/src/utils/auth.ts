import { getAdminBasePath } from './path'

const ACCOUNT_COOKIE_NAME = 'nehex_admin_account'
const AUTH_COOKIE_MAX_AGE_SECONDS = 60 * 60 * 24 * 30

function getCookie(name: string): string | null {
  const prefix = `${name}=`
  const cookies = document.cookie ? document.cookie.split('; ') : []

  for (const cookie of cookies) {
    if (cookie.startsWith(prefix)) {
      return cookie.substring(prefix.length)
    }
  }

  return null
}

function setCookie(name: string, value: string, maxAgeSeconds: number): void {
  const expires = new Date(Date.now() + maxAgeSeconds * 1000).toUTCString()
  const secure = window.location.protocol === 'https:' ? '; Secure' : ''
  const path = getAdminBasePath()

  document.cookie = `${name}=${encodeURIComponent(value)}; Max-Age=${maxAgeSeconds}; Expires=${expires}; Path=${path}; SameSite=Lax${secure}`
}

function normalizeCookiePath(path: string): string {
  const text = path.trim()
  if (!text) {
    return '/'
  }
  return text.startsWith('/') ? text : `/${text}`
}

function clearCookie(name: string, additionalPaths: string[] = []): void {
  const secure = window.location.protocol === 'https:' ? '; Secure' : ''
  const defaultPath = normalizeCookiePath(getAdminBasePath())
  const paths = Array.from(
    new Set(
      [
        defaultPath,
        '/',
        '/nehex-admin',
        ...additionalPaths.map((item) => normalizeCookiePath(item)),
      ],
    ),
  )

  paths.forEach((path) => {
    document.cookie = `${name}=; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; Path=${path}; SameSite=Lax${secure}`
  })
}

export function setAuthSession(account: string): void {
  setCookie(ACCOUNT_COOKIE_NAME, account, AUTH_COOKIE_MAX_AGE_SECONDS)
}

export function clearAuthSession(): void {
  clearCookie(ACCOUNT_COOKIE_NAME)
}

export function getAuthenticatedAccount(): string {
  const raw = getCookie(ACCOUNT_COOKIE_NAME) || ''
  try {
    return decodeURIComponent(raw)
  } catch {
    return raw
  }
}
