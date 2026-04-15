<template>
  <AdminLayout>
    <template #secondary-nav>
      <div class="kuma-subnav">
        <div class="subnav-title">Kuma</div>
        <v-list class="subnav-list" density="comfortable" nav>
          <v-list-item
            v-for="item in sections"
            :key="item.key"
            class="subnav-item"
            :active="activeSection === item.key"
            :prepend-icon="item.icon"
            rounded="lg"
            :title="item.label"
            @click="activeSection = item.key"
          />
        </v-list>
      </div>
    </template>

    <section class="kuma-page">
      <header class="page-header">
        <div class="header-text">
          <h1>{{ activeSectionMeta.label }}</h1>
          <p>{{ activeSectionMeta.description }}</p>
        </div>
      </header>

      <v-window v-model="activeSection" :touch="false" class="section-window">
        <v-window-item value="movie">
          <v-card class="section-card" rounded="xl">
            <v-card-title>电影配置（预留）</v-card-title>
            <v-card-text>
              电影模块配置页面预留中，后续将在这里接入 Kuma 电影相关配置。
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="music">
          <v-card class="section-card" rounded="xl">
            <v-card-title>音乐配置（预留）</v-card-title>
            <v-card-text>
              音乐模块配置页面预留中，后续将在这里接入 Kuma 音乐相关配置。
            </v-card-text>
          </v-card>
        </v-window-item>
      </v-window>
    </section>
  </AdminLayout>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import AdminLayout from '@/components/admin/AdminLayout.vue'

type KumaSectionKey = 'movie' | 'music'

type SectionMeta = {
  key: KumaSectionKey
  label: string
  icon: string
  description: string
}

const sections: SectionMeta[] = [
  {
    key: 'movie',
    label: '电影',
    icon: 'mdi-movie-open-outline',
    description: '管理 Kuma 电影相关配置（待实现）。',
  },
  {
    key: 'music',
    label: '音乐',
    icon: 'mdi-music-note-outline',
    description: '管理 Kuma 音乐相关配置（待实现）。',
  },
]

const activeSection = ref<KumaSectionKey>('movie')
const activeSectionMeta = computed(() => {
  return sections.find((item) => item.key === activeSection.value) || sections[0]!
})
</script>

<style scoped>
.kuma-subnav {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.subnav-title {
  padding: 8px 8px 0;
  font-size: 16px;
  font-weight: 700;
  color: #f2f5ff;
  letter-spacing: 0.3px;
}

.subnav-list {
  padding: 4px 0 0;
  background: transparent;
}

:deep(.subnav-item) {
  min-height: 42px;
  margin-bottom: 6px;
  color: #b6c3de;
  border: 1px solid transparent;
  transition:
    background 0.2s ease,
    color 0.2s ease;
}

:deep(.subnav-item:hover) {
  color: #eef3ff;
  background: linear-gradient(90deg, rgba(103, 121, 170, 0.14) 0%, rgba(112, 133, 186, 0.24) 100%);
}

:deep(.subnav-item.v-list-item--active) {
  color: #ffffff;
  background: linear-gradient(90deg, rgba(103, 121, 170, 0.28) 0%, rgba(112, 133, 186, 0.42) 100%);
}

.kuma-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: 12px;
}

.header-text h1 {
  margin: 0;
  font-size: 28px;
  color: #f1f4ff;
}

.header-text p {
  margin: 6px 0 0;
  color: #aeb8cc;
}

.section-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, rgba(24, 30, 41, 0.96), rgba(19, 24, 34, 0.96));
  color: #edf1ff;
}

@media (max-width: 980px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
