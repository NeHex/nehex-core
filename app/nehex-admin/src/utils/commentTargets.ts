type CommentTargetLike = {
  target_type: string
  target_id: number
}

type CommentLinkLike = CommentTargetLike & {
  id: number
}

type StandalonePageResolver = (pageId: number) => Promise<string | null | undefined>

export function normalizeCommentTargetType(targetType: string): string {
  return String(targetType || '').trim().toLowerCase()
}

export function mapCommentTargetLabel(targetType: string): string {
  const normalizedType = normalizeCommentTargetType(targetType)
  const labelMap: Record<string, string> = {
    article: '文章',
    album: '相册',
    daily: '日常',
    project: '项目',
    singlepage: '独立页',
    friend_page: '友链页',
  }
  return labelMap[normalizedType] || normalizedType || '未知'
}

export function canJumpToCommentTarget(comment: CommentTargetLike): boolean {
  const targetType = normalizeCommentTargetType(comment.target_type)
  if (targetType === 'friend_page') {
    return true
  }

  const targetId = Number(comment.target_id)
  if (!Number.isFinite(targetId) || targetId <= 0) {
    return false
  }

  return targetType === 'article'
    || targetType === 'album'
    || targetType === 'daily'
    || targetType === 'project'
    || targetType === 'singlepage'
}

export function buildCommentTargetPath(
  targetType: string,
  targetId: number,
  pageKey?: string | null,
): string {
  const normalizedType = normalizeCommentTargetType(targetType)
  const normalizedId = Math.max(1, Math.floor(Number(targetId) || 0))

  if (normalizedType === 'article') {
    return `/article/${normalizedId}`
  }
  if (normalizedType === 'album') {
    return `/album/${normalizedId}`
  }
  if (normalizedType === 'daily') {
    return `/daily/${normalizedId}`
  }
  if (normalizedType === 'project') {
    return `/project/${normalizedId}`
  }
  if (normalizedType === 'friend_page') {
    return '/friends'
  }
  if (normalizedType === 'singlepage') {
    const normalizedPageKey = String(pageKey ?? '').trim().replace(/^\/+|\/+$/g, '')
    return normalizedPageKey ? `/${normalizedPageKey}` : `/page/${normalizedId}`
  }
  return ''
}

export async function resolveCommentTargetPath(
  comment: CommentTargetLike,
  resolveStandalonePagePath?: StandalonePageResolver,
): Promise<string> {
  const targetType = normalizeCommentTargetType(comment.target_type)
  const targetId = Math.max(1, Math.floor(Number(comment.target_id) || 0))

  if (targetType !== 'singlepage') {
    return buildCommentTargetPath(targetType, targetId)
  }

  if (resolveStandalonePagePath) {
    const pagePath = await resolveStandalonePagePath(targetId)
    if (pagePath) {
      return buildCommentTargetPath(targetType, targetId, pagePath)
    }
  }

  return buildCommentTargetPath(targetType, targetId)
}

export function joinSiteUrl(baseUrl: string, path: string): string {
  const normalizedPath = path.trim() ? `/${path.trim().replace(/^\/+/, '')}` : '/'
  const normalizedBase = baseUrl.trim().replace(/\/+$/, '')
  return normalizedBase ? `${normalizedBase}${normalizedPath}` : normalizedPath
}

export function withCommentAnchor(url: string, commentId: number): string {
  const normalizedUrl = String(url || '').trim()
  if (!normalizedUrl) {
    return ''
  }
  return `${normalizedUrl}#comment-${Math.max(1, Math.floor(Number(commentId) || 0))}`
}

export async function buildCommentTargetUrl(
  comment: CommentLinkLike,
  siteUrl: string,
  resolveStandalonePagePath?: StandalonePageResolver,
): Promise<string> {
  const path = await resolveCommentTargetPath(comment, resolveStandalonePagePath)
  return withCommentAnchor(joinSiteUrl(siteUrl, path), comment.id)
}
