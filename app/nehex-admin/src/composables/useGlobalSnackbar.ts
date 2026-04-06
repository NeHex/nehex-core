import { ref } from 'vue'

const DEFAULT_DURATION_MS = 3000
const TICK_MS = 40

const visible = ref(false)
const message = ref('')
const progress = ref(100)

let timerId: number | null = null
let startedAt = 0
let durationMs = DEFAULT_DURATION_MS

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
}

function showGlobalSuccess(messageText: string, duration = DEFAULT_DURATION_MS): void {
  clearTimer()

  durationMs = Math.max(300, Math.floor(duration))
  startedAt = Date.now()

  message.value = messageText.trim() || '发布成功'
  visible.value = true
  progress.value = 100

  timerId = window.setInterval(() => {
    const elapsed = Date.now() - startedAt
    const ratio = Math.max(0, 1 - (elapsed / durationMs))
    progress.value = ratio * 100

    if (elapsed >= durationMs) {
      hideGlobalSnackbar()
    }
  }, TICK_MS)
}

export function useGlobalSnackbar() {
  return {
    visible,
    message,
    progress,
    showGlobalSuccess,
    hideGlobalSnackbar,
  }
}
