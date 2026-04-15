import { computed, onBeforeUnmount, onMounted, ref, type Ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'

type LeaveResolver = (allow: boolean) => void

export function useUnsavedChangesGuard(hasUnsavedChanges: Ref<boolean>) {
  const dialogVisible = ref(false)
  let resolver: LeaveResolver | null = null
  let pendingDecision: Promise<boolean> | null = null

  function clearPendingState(): void {
    resolver = null
    pendingDecision = null
  }

  function resolveLeaveDecision(allow: boolean): void {
    dialogVisible.value = false
    if (resolver) {
      resolver(allow)
    }
    clearPendingState()
  }

  function requestLeaveDecision(): Promise<boolean> {
    if (!hasUnsavedChanges.value) {
      return Promise.resolve(true)
    }

    if (pendingDecision) {
      return pendingDecision
    }

    dialogVisible.value = true
    pendingDecision = new Promise<boolean>((resolve) => {
      resolver = resolve
    })
    return pendingDecision
  }

  function confirmLeave(): void {
    resolveLeaveDecision(true)
  }

  function cancelLeave(): void {
    resolveLeaveDecision(false)
  }

  function handleBeforeUnload(event: BeforeUnloadEvent): void {
    if (!hasUnsavedChanges.value) {
      return
    }
    event.preventDefault()
    event.returnValue = ''
  }

  onBeforeRouteLeave(async () => requestLeaveDecision())

  onMounted(() => {
    window.addEventListener('beforeunload', handleBeforeUnload)
  })

  onBeforeUnmount(() => {
    window.removeEventListener('beforeunload', handleBeforeUnload)
    clearPendingState()
  })

  return {
    unsavedLeaveDialogVisible: computed({
      get: () => dialogVisible.value,
      set: (value: boolean) => {
        dialogVisible.value = value
      },
    }),
    confirmUnsavedLeave: confirmLeave,
    cancelUnsavedLeave: cancelLeave,
  }
}
