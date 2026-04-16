import { computed, ref } from 'vue'

const MIN_LOADING_VISIBLE_MS = 220

const routeLoading = ref(false)
let loadingStartedAt = 0
let finishTimer: number | null = null

function clearFinishTimer(): void {
  if (finishTimer === null) {
    return
  }
  clearTimeout(finishTimer)
  finishTimer = null
}

function startRouteLoading(): void {
  clearFinishTimer()
  if (!routeLoading.value) {
    routeLoading.value = true
    loadingStartedAt = Date.now()
  }
}

function finishRouteLoading(): void {
  if (!routeLoading.value) {
    return
  }

  const elapsed = Date.now() - loadingStartedAt
  const remaining = Math.max(0, MIN_LOADING_VISIBLE_MS - elapsed)
  if (remaining === 0) {
    routeLoading.value = false
    return
  }

  finishTimer = window.setTimeout(() => {
    routeLoading.value = false
    finishTimer = null
  }, remaining)
}

export function useRouteLoading() {
  return {
    isRouteLoading: computed(() => routeLoading.value),
    startRouteLoading,
    finishRouteLoading,
  }
}
