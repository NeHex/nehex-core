<template>
  <v-card class="section-card" rounded="lg">
    <v-card-title class="text-h5 font-weight-bold">{{ title }}</v-card-title>
    <v-card-subtitle class="pt-1">{{ subtitle }}</v-card-subtitle>
    <v-divider class="my-3" />
    <v-card-text>
      <slot>
        <div class="placeholder">{{ description }}</div>
      </slot>
    </v-card-text>
  </v-card>
</template>

<script lang="ts" setup>
defineProps<{
  title: string
  subtitle: string
  description: string
}>()
</script>

<style scoped>
.section-card {
  border: 1px solid var(--admin-border-soft);
  background: var(--admin-card-bg);
  color: var(--admin-text-primary);
}

.placeholder {
  min-height: 220px;
  display: grid;
  place-items: center;
  border-radius: 12px;
  border: 1px dashed var(--admin-empty-border);
  color: var(--admin-text-muted);
  text-align: center;
  padding: 20px;
}
</style>
