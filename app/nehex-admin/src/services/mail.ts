import { adminFetch } from '@/services/admin-api'

export type MailSmtpSecurity = 'none' | 'starttls' | 'ssl'
export type MailLogStatusFilter = 'all' | 'success' | 'failed'

export type AdminMailSmtpTestPayload = {
  smtp_host: string
  smtp_port: number
  smtp_security: MailSmtpSecurity
  smtp_username?: string | null
  smtp_password?: string | null
  smtp_from_email?: string | null
  smtp_from_name?: string | null
  smtp_timeout_seconds?: number
  test_email: string
}

type AdminActionResponse = {
  message?: unknown
}

export type AdminMailLogItem = {
  id: number
  category: string
  template_key: string
  to_email: string
  subject: string
  body: string
  status: string
  error_message?: string | null
  trigger_comment_id?: number | null
  created_at: string
  sent_at?: string | null
}

type AdminMailLogListResponse = {
  data: AdminMailLogItem[]
  pagination?: {
    page?: number
    size?: number
    total?: number
    total_pages?: number
  }
}

export type AdminMailLogListResult = {
  items: AdminMailLogItem[]
  pagination: {
    page: number
    size: number
    total: number
    total_pages: number
  }
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function testAdminMailSmtpConnection(payload: AdminMailSmtpTestPayload): Promise<string> {
  const response = await adminFetch('/admin-api/settings/mail/test', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
  const result = await parseJson<AdminActionResponse>(response)
  return typeof result?.message === 'string' ? result.message : 'SMTP 通信成功'
}

export async function fetchAdminMailLogs(
  status: MailLogStatusFilter,
  page = 1,
  size = 20,
): Promise<AdminMailLogListResult> {
  const params = new URLSearchParams()
  params.set('status', status)
  params.set('page', String(Math.max(1, Math.floor(page))))
  params.set('size', String(Math.max(1, Math.floor(size))))

  const response = await adminFetch(`/admin-api/mail-logs?${params.toString()}`, {
    method: 'GET',
  })
  const payload = await parseJson<AdminMailLogListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected mail log response format')
  }

  const safePage = Number.isFinite(payload.pagination?.page) ? Number(payload.pagination?.page) : Math.max(1, page)
  const safeSize = Number.isFinite(payload.pagination?.size) ? Number(payload.pagination?.size) : Math.max(1, size)
  const safeTotal = Number.isFinite(payload.pagination?.total) ? Number(payload.pagination?.total) : payload.data.length
  const safeTotalPages = Number.isFinite(payload.pagination?.total_pages)
    ? Number(payload.pagination?.total_pages)
    : Math.max(0, Math.ceil(safeTotal / safeSize))

  return {
    items: payload.data,
    pagination: {
      page: Math.max(1, safePage),
      size: Math.max(1, safeSize),
      total: Math.max(0, safeTotal),
      total_pages: Math.max(0, safeTotalPages),
    },
  }
}
