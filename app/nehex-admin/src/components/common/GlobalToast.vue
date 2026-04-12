<template>
  <v-snackbar
    v-model="snackbarModel"
    class="publish-snackbar"
    :color="snackbarColor"
    location="top end"
    :timeout="-1"
    transition="fade-transition"
    variant="elevated"
  >
    <div class="publish-snackbar__head">
      <div class="publish-snackbar__message">{{ snackbarMessage }}</div>
      <div v-if="snackbarCountdownSeconds > 0" class="publish-snackbar__countdown">
        {{ snackbarCountdownSeconds.toFixed(1) }}s
      </div>
    </div>
    <v-progress-linear
      class="publish-snackbar__progress"
      color="white"
      height="3"
      :indeterminate="snackbarProgressIndeterminate"
      :model-value="snackbarProgress"
    />
  </v-snackbar>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'

const {
  visible: snackbarVisible,
  message: snackbarMessage,
  progress: snackbarProgress,
  color: snackbarColor,
  progressIndeterminate: snackbarProgressIndeterminate,
  countdownSeconds: snackbarCountdownSeconds,
  hideGlobalSnackbar,
} = useGlobalSnackbar()

const snackbarModel = computed({
  get: () => snackbarVisible.value,
  set: (next) => {
    if (next) {
      snackbarVisible.value = true
    } else {
      hideGlobalSnackbar()
    }
  },
})
</script>

<style scoped>
.publish-snackbar :deep(.v-snackbar__wrapper) {
  min-width: 280px;
  max-width: 420px;
}

@media (max-width: 760px) {
  .publish-snackbar :deep(.v-snackbar__wrapper) {
    min-width: auto;
    width: min(92vw, 420px);
  }
}

.publish-snackbar__message {
  font-weight: 600;
  line-height: 1.4;
}

.publish-snackbar__head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
}

.publish-snackbar__countdown {
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 600;
  opacity: 0.95;
}

.publish-snackbar__progress {
  margin-top: 10px;
}
</style>
