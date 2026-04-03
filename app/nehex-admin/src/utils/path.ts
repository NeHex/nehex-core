export function normalizeBasePath(value?: string): string {
  const trimmed = (value || '').trim()
  if (!trimmed) {
    return '/'
  }

  let path = trimmed
  if (!path.startsWith('/')) {
    path = `/${path}`
  }

  path = path.replace(/\/+$/, '')
  return path || '/'
}

export function toRouterBase(path: string): string {
  return `${normalizeBasePath(path)}/`
}

export function getAdminBasePath(): string {
  return normalizeBasePath(window.__NEHEX_ADMIN_BASE__ || import.meta.env.BASE_URL)
}
