<template>
  <v-app>
    <v-main>
      <router-view />
    </v-main>
    <GlobalToast />
  </v-app>
</template>

<script lang="ts" setup>
import GlobalToast from '@/components/common/GlobalToast.vue'
</script>
