<template>
  <v-app>
    <v-main>
      <div class="app-route-shell">
        <transition name="route-fade" mode="out-in">
          <router-view :key="route.fullPath" />
        </transition>

        <transition name="route-loading-fade">
          <div
            v-if="isRouteLoading"
            class="route-loading-overlay"
            :class="{
              'route-loading-overlay--dark': isDarkTheme,
              'route-loading-overlay--light': !isDarkTheme,
            }"
          >
            <div class="route-loading-card">
              <v-progress-circular
                color="primary"
                indeterminate
                size="22"
                width="3"
              />
              <span>页面加载中...</span>
            </div>
          </div>
        </transition>
      </div>
    </v-main>
    <GlobalToast />
  </v-app>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useTheme } from 'vuetify'
import GlobalToast from '@/components/common/GlobalToast.vue'
import { useRouteLoading } from '@/composables/useRouteLoading'

const route = useRoute()
const { isRouteLoading } = useRouteLoading()
const vuetifyTheme = useTheme()
const isDarkTheme = computed(() => String(vuetifyTheme.global.name.value).includes('dark'))
</script>

<style scoped>
.app-route-shell {
  position: relative;
  min-height: 100%;
}

.route-loading-overlay {
  --overlay-bg: rgba(8, 12, 18, 0.36);
  --card-border: rgba(162, 184, 236, 0.35);
  --card-bg: rgba(20, 28, 40, 0.95);
  --card-text: #f3f7ff;
  position: fixed;
  inset: 0;
  z-index: 3000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: var(--overlay-bg);
  backdrop-filter: blur(2px);
}

.route-loading-overlay--light {
  --overlay-bg: rgba(74, 111, 165, 0.2);
  --card-border: rgba(74, 111, 165, 0.28);
  --card-bg: rgba(255, 255, 255, 0.95);
  --card-text: #2a4267;
}

.route-loading-card {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border-radius: 999px;
  border: 1px solid var(--card-border);
  background: var(--card-bg);
  color: var(--card-text);
  font-size: 13px;
  font-weight: 600;
}

.route-fade-enter-active,
.route-fade-leave-active {
  transition: opacity 0.2s ease;
}

.route-fade-enter-from,
.route-fade-leave-to {
  opacity: 0;
}

.route-loading-fade-enter-active,
.route-loading-fade-leave-active {
  transition: opacity 0.16s ease;
}

.route-loading-fade-enter-from,
.route-loading-fade-leave-to {
  opacity: 0;
}
</style>
