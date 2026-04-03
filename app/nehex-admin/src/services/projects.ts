import { adminFetch } from '@/services/admin-api'

export type ProjectItem = {
  id: number
  title: string
  cover?: string | null
  category?: string | null
  description?: string | null
  content?: string | null
  tech_stack?: string | null
  project_url?: string | null
  github_url?: string | null
  sort: number
  status: number
  create_time: string
  update_time: string
}

type ProjectListResponse = {
  data: ProjectItem[]
}

type ProjectDetailResponse = {
  data: ProjectItem
}

export type ProjectUpsertPayload = {
  title: string
  cover?: string | null
  category?: string | null
  description?: string | null
  content?: string | null
  tech_stack?: string | null
  project_url?: string | null
  github_url?: string | null
  sort: number
  status: number
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchProjects(): Promise<ProjectItem[]> {
  const response = await fetch('/project', {
    method: 'GET',
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new Error(`Failed to request projects: ${response.status}`)
  }

  const payload = await parseJson<ProjectListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected projects response format')
  }

  return payload.data
}

export async function fetchProjectById(projectId: number): Promise<ProjectItem> {
  const response = await fetch(`/project/${projectId}`, {
    method: 'GET',
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new Error(`Failed to request project detail: ${response.status}`)
  }

  const payload = await parseJson<ProjectDetailResponse>(response)
  if (!payload?.data) {
    throw new Error('Unexpected project detail response format')
  }

  return payload.data
}

export async function createProject(payload: ProjectUpsertPayload): Promise<ProjectItem> {
  const response = await adminFetch('/admin-api/projects', {
    method: 'POST',
    body: JSON.stringify(payload),
  })

  const result = await parseJson<ProjectDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected create project response format')
  }
  return result.data
}

export async function updateProject(projectId: number, payload: ProjectUpsertPayload): Promise<ProjectItem> {
  const response = await adminFetch(`/admin-api/projects/${projectId}`, {
    method: 'PUT',
    body: JSON.stringify(payload),
  })

  const result = await parseJson<ProjectDetailResponse>(response)
  if (!result?.data) {
    throw new Error('Unexpected update project response format')
  }
  return result.data
}

export async function deleteProject(projectId: number): Promise<void> {
  await adminFetch(`/admin-api/projects/${projectId}`, {
    method: 'DELETE',
  })
}
