import { adminFetch } from '@/services/admin-api'

export type DeveloperCliEngine = 'postgresql' | 'docker'

export type DeveloperCliExecuteResult = {
  engine: string
  command: string
  output: string
  exit_code: number
  duration_ms: number
  truncated: boolean
}

type DeveloperCliExecuteResponse = {
  data?: Partial<DeveloperCliExecuteResult>
}

type DeveloperLogListResponse = {
  data?: unknown
  total?: unknown
}

export type DeveloperLogListResult = {
  logs: string[]
  total: number
}

export async function executeDeveloperCli(
  engine: DeveloperCliEngine,
  command: string,
): Promise<DeveloperCliExecuteResult> {
  const response = await adminFetch('/admin-api/developer/cli/execute', {
    method: 'POST',
    body: JSON.stringify({
      engine,
      command,
    }),
  })
  const payload = await response.json() as DeveloperCliExecuteResponse
  const data = payload?.data ?? {}
  const output = typeof data.output === 'string' ? data.output : ''

  return {
    engine: typeof data.engine === 'string' ? data.engine : engine,
    command: typeof data.command === 'string' ? data.command : command,
    output: output || '(no output)',
    exit_code: Number.isFinite(data.exit_code) ? Number(data.exit_code) : -1,
    duration_ms: Number.isFinite(data.duration_ms) ? Number(data.duration_ms) : 0,
    truncated: Boolean(data.truncated),
  }
}

export async function fetchDeveloperLogs(limit = 300, keyword = ''): Promise<DeveloperLogListResult> {
  const safeLimit = Math.min(2000, Math.max(1, Math.floor(limit)))
  const params = new URLSearchParams()
  params.set('limit', String(safeLimit))
  if (keyword.trim()) {
    params.set('keyword', keyword.trim())
  }

  const response = await adminFetch(`/admin-api/developer/logs?${params.toString()}`, {
    method: 'GET',
  })
  const payload = await response.json() as DeveloperLogListResponse
  const rows = Array.isArray(payload?.data) ? payload.data : []
  const logs = rows.map((item) => String(item ?? ''))
  const total = Number.isFinite(payload?.total) ? Number(payload.total) : logs.length

  return {
    logs,
    total: Math.max(0, total),
  }
}
