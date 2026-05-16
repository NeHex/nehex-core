import { adminFetch } from '@/services/admin-api'

type AdminArticleSummaryGenerateResponse = {
  data?: {
    summary?: unknown
  }
}

export async function generateArticleSummary(articleContent: string): Promise<string> {
  const response = await adminFetch('/admin-api/ai/article-summary', {
    method: 'POST',
    body: JSON.stringify({
      article_content: articleContent,
    }),
  })

  const payload = await response.json() as AdminArticleSummaryGenerateResponse
  const summary = typeof payload?.data?.summary === 'string' ? payload.data.summary.trim() : ''
  if (!summary) {
    throw new Error('AI 未返回有效总结')
  }
  return summary
}
