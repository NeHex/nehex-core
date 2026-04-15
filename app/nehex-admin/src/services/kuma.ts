import { adminFetch } from '@/services/admin-api'

export type KumaMovieProvider = 'douban' | 'tmdb'

export type KumaMovieCard = {
  id: number
  provider: string
  movie_id: string
  cover: string
  title: string
  years: string
  score?: string | null
  desc: string
  url: string
  create_time: string
  update_time: string
}

type KumaMovieListResponse = {
  data?: KumaMovieCard[]
}

type KumaMovieDetailResponse = {
  data?: KumaMovieCard
}

type KumaMovieCreatePayload = {
  provider: KumaMovieProvider
  movie_id: string
}

type KumaMovieActionResponse = {
  success?: boolean
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchAdminKumaMovies(): Promise<KumaMovieCard[]> {
  const response = await adminFetch('/admin-api/kuma/movies', {
    method: 'GET',
    cache: 'no-store',
  })

  const payload = await parseJson<KumaMovieListResponse>(response)
  if (!Array.isArray(payload?.data)) {
    throw new Error('Unexpected kuma movie list response format')
  }
  return payload.data
}

export async function createAdminKumaMovie(payload: KumaMovieCreatePayload): Promise<KumaMovieCard> {
  const response = await adminFetch('/admin-api/kuma/movies', {
    method: 'POST',
    body: JSON.stringify(payload),
  })

  const result = await parseJson<KumaMovieDetailResponse>(response)
  if (!result?.data || typeof result.data !== 'object') {
    throw new Error('Unexpected kuma movie create response format')
  }
  return result.data
}

export async function deleteAdminKumaMovie(id: number): Promise<void> {
  const response = await adminFetch(`/admin-api/kuma/movies/${id}`, {
    method: 'DELETE',
  })

  const result = await parseJson<KumaMovieActionResponse>(response)
  if (!result?.success) {
    throw new Error('删除电影卡片失败')
  }
}
