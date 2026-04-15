import { adminFetch } from '@/services/admin-api'

export type KumaMovieProvider = 'douban' | 'tmdb'

export type KumaMovieItem = {
  cover: string
  title: string
  years: string
  desc: string
  url: string
  score?: string | null
}

type KumaMovieResponse = {
  success?: unknown
  provider?: unknown
  movie_id?: unknown
  data?: KumaMovieItem
}

async function parseJson<T>(response: Response): Promise<T> {
  return await response.json() as T
}

export async function fetchAdminKumaMovie(
  provider: KumaMovieProvider,
  movieId: string,
): Promise<KumaMovieItem> {
  const normalizedMovieId = movieId.trim()
  if (!normalizedMovieId) {
    throw new Error('电影 ID 不能为空')
  }

  const safeMovieId = encodeURIComponent(normalizedMovieId)
  const response = await adminFetch(`/admin-api/settings/kuma-api/${provider}/${safeMovieId}`, {
    method: 'GET',
    cache: 'no-store',
  })

  const payload = await parseJson<KumaMovieResponse>(response)
  if (!payload?.data || typeof payload.data !== 'object') {
    throw new Error('Unexpected kuma movie response format')
  }

  return payload.data
}
