<template>
  <v-app>
    <v-main>
      <router-view />
    </v-main>

    <v-snackbar
      v-model="snackbarModel"
      class="publish-snackbar"
      color="success"
      location="top end"
      :timeout="-1"
      transition="fade-transition"
      variant="elevated"
    >
      <div class="publish-snackbar__message">{{ snackbarMessage }}</div>
      <v-progress-linear
        class="publish-snackbar__progress"
        color="white"
        height="3"
        :model-value="snackbarProgress"
      />
    </v-snackbar>
  </v-app>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'

const {
  visible: snackbarVisible,
  message: snackbarMessage,
  progress: snackbarProgress,
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

.publish-snackbar__message {
  font-weight: 600;
  line-height: 1.4;
}

.publish-snackbar__progress {
  margin-top: 10px;
}
</style>
