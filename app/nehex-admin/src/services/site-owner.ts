export type SiteOwnerProfile = {
  avatar: string
  nickname: string
  homepage: string
  email: string
  bio: string
}

type SiteOwnerProfileResponse = {
  data?: Partial<SiteOwnerProfile> | null
}

const DEFAULT_SITE_OWNER_PROFILE: SiteOwnerProfile = {
  avatar: '/images/head.jpg',
  nickname: '站长',
  homepage: '',
  email: '',
  bio: '',
}

function normalizeSiteOwnerProfile(
  source: Partial<SiteOwnerProfile> | null | undefined,
): SiteOwnerProfile {
  return {
    avatar: String(source?.avatar ?? '').trim() || DEFAULT_SITE_OWNER_PROFILE.avatar,
    nickname: String(source?.nickname ?? '').trim() || DEFAULT_SITE_OWNER_PROFILE.nickname,
    homepage: String(source?.homepage ?? '').trim(),
    email: String(source?.email ?? '').trim(),
    bio: String(source?.bio ?? '').trim(),
  }
}

async function requestSiteOwnerProfile(path: string): Promise<SiteOwnerProfile> {
  const response = await fetch(path, {
    method: 'GET',
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new Error(`Failed to request site owner profile: ${response.status}`)
  }

  const payload = await response.json() as SiteOwnerProfileResponse
  return normalizeSiteOwnerProfile(payload?.data)
}

export async function fetchSiteOwnerProfile(): Promise<SiteOwnerProfile> {
  try {
    return await requestSiteOwnerProfile('/site-owner')
  } catch (error) {
    if (error instanceof Error && error.message.endsWith(': 404')) {
      return requestSiteOwnerProfile('/setting/site-owner')
    }
    throw error
  }
}
