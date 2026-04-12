import { ref } from 'vue'

const DEFAULT_DURATION_MS = 3000
const TICK_MS = 40
type SnackbarTone = 'success' | 'error' | 'info'

const visible = ref(false)
const message = ref('')
const progress = ref(100)
const color = ref('success')
const progressIndeterminate = ref(false)
const countdownSeconds = ref(0)

let timerId: number | null = null
let startedAt = 0
let durationMs = DEFAULT_DURATION_MS

function clampPercent(value: number): number {
  if (!Number.isFinite(value)) {
    return 0
  }
  return Math.max(0, Math.min(100, Math.round(value)))
}

function clearTimer(): void {
  if (timerId !== null) {
    clearInterval(timerId)
    timerId = null
  }
}

function hideGlobalSnackbar(): void {
  clearTimer()
  visible.value = false
  progress.value = 0
  progressIndeterminate.value = false
  countdownSeconds.value = 0
}

function showGlobalToast(
  messageText: string,
  tone: SnackbarTone = 'success',
  duration = DEFAULT_DURATION_MS,
): void {
  clearTimer()

  color.value = tone === 'error' ? 'error' : tone === 'info' ? 'info' : 'success'
  progressIndeterminate.value = false
  durationMs = Math.max(300, Math.floor(duration))
  startedAt = Date.now()

  if (tone === 'error') {
    message.value = messageText.trim() || '操作失败'
  } else if (tone === 'info') {
    message.value = messageText.trim() || '请注意'
  } else {
    message.value = messageText.trim() || '发布成功'
  }
  visible.value = true
  progress.value = 100
  countdownSeconds.value = Math.max(0, durationMs / 1000)

  timerId = window.setInterval(() => {
    const elapsed = Date.now() - startedAt
    const ratio = Math.max(0, 1 - (elapsed / durationMs))
    progress.value = ratio * 100
    countdownSeconds.value = Math.max(0, Math.ceil(((durationMs - elapsed) / 100)) / 10)

    if (elapsed >= durationMs) {
      hideGlobalSnackbar()
    }
  }, TICK_MS)
}

function showGlobalSuccess(messageText: string, duration = DEFAULT_DURATION_MS): void {
  showGlobalToast(messageText, 'success', duration)
}

function showGlobalError(messageText: string, duration = DEFAULT_DURATION_MS): void {
  showGlobalToast(messageText, 'error', duration)
}

function showGlobalInfo(messageText: string, duration = DEFAULT_DURATION_MS): void {
  showGlobalToast(messageText, 'info', duration)
}

function showGlobalProgress(messageText: string, percent = 0): void {
  clearTimer()
  color.value = 'primary'
  progressIndeterminate.value = false
  message.value = messageText.trim() || '正在上传...'
  progress.value = clampPercent(percent)
  countdownSeconds.value = 0
  visible.value = true
}

function updateGlobalProgress(messageText: string, percent: number): void {
  clearTimer()
  color.value = 'primary'
  progressIndeterminate.value = false
  message.value = messageText.trim() || message.value
  progress.value = clampPercent(percent)
  countdownSeconds.value = 0
  visible.value = true
}

export function useGlobalSnackbar() {
  return {
    visible,
    message,
    progress,
    color,
    progressIndeterminate,
    countdownSeconds,
    showGlobalToast,
    showGlobalSuccess,
    showGlobalError,
    showGlobalInfo,
    showGlobalProgress,
    updateGlobalProgress,
    hideGlobalSnackbar,
  }
}
