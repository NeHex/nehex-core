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
          <div class="movie-cards-grid">
            <v-card class="movie-card movie-card--creator" rounded="xl">
              <div class="movie-cover movie-cover--placeholder">
                <v-icon icon="mdi-movie-open-plus-outline" size="46" />
                <span>新建电影卡片</span>
              </div>

              <v-card-text class="movie-card-body">
                <div class="movie-input-row">
                  <v-text-field
                    v-model="movieIdInput"
                    density="comfortable"
                    hide-details
                    label="电影 ID"
                    placeholder="例如：1292052 / 278"
                    variant="outlined"
                    @keydown.enter.prevent="createMovieCard"
                  />
                  <v-select
                    v-model="movieProvider"
                    :items="movieProviderOptions"
                    density="comfortable"
                    hide-details
                    item-title="label"
                    item-value="value"
                    label="来源"
                    variant="outlined"
                  />
                </div>

                <v-btn
                  block
                  color="primary"
                  prepend-icon="mdi-plus"
                  :loading="movieCreating"
                  @click="createMovieCard"
                >
                  新建
                </v-btn>

                <v-alert
                  v-if="movieCreateError"
                  class="mt-2"
                  density="comfortable"
                  type="error"
                  variant="tonal"
                >
                  {{ movieCreateError }}
                </v-alert>
              </v-card-text>
            </v-card>

            <v-card
              v-for="card in movieCards"
              :key="card.id"
              class="movie-card"
              rounded="xl"
            >
              <v-img
                v-if="card.cover"
                :src="card.cover"
                class="movie-cover"
                cover
                height="320"
              />
              <div v-else class="movie-cover movie-cover--empty">
                <v-icon icon="mdi-image-off-outline" size="42" />
                <span>无封面</span>
              </div>

              <v-card-text class="movie-card-body">
                <div class="movie-meta-row">
                  <v-chip size="small" variant="tonal">
                    {{ card.provider.toUpperCase() }}
                  </v-chip>
                  <v-chip size="small" variant="tonal">#{{ card.movieId }}</v-chip>
                  <v-chip
                    v-if="card.score"
                    color="warning"
                    size="small"
                    variant="tonal"
                  >
                    评分 {{ card.score }}
                  </v-chip>
                </div>

                <div class="movie-title">
                  {{ card.title }}
                  <span v-if="card.years" class="movie-years">{{ card.years }}</span>
                </div>

                <p class="movie-desc">{{ card.desc || '暂无简介' }}</p>

                <v-btn
                  v-if="card.url"
                  class="movie-link-btn"
                  color="primary"
                  :href="card.url"
                  prepend-icon="mdi-open-in-new"
                  rel="noopener noreferrer"
                  target="_blank"
                  variant="text"
                >
                  打开详情
                </v-btn>
              </v-card-text>
            </v-card>
          </div>
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
import { fetchAdminKumaMovie, type KumaMovieProvider } from '@/services/kuma'

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

type MovieCard = {
  id: number
  provider: KumaMovieProvider
  movieId: string
  cover: string
  title: string
  years: string
  desc: string
  url: string
  score?: string | null
}

const movieProviderOptions: Array<{ label: string, value: KumaMovieProvider }> = [
  { label: '豆瓣', value: 'douban' },
  { label: 'TMDB', value: 'tmdb' },
]

const movieIdInput = ref('')
const movieProvider = ref<KumaMovieProvider>('douban')
const movieCreating = ref(false)
const movieCreateError = ref('')
const movieCards = ref<MovieCard[]>([])
let movieCardSerial = 1

async function createMovieCard(): Promise<void> {
  const movieId = movieIdInput.value.trim()
  if (!movieId) {
    movieCreateError.value = '请先输入电影 ID'
    return
  }

  movieCreating.value = true
  movieCreateError.value = ''
  try {
    const item = await fetchAdminKumaMovie(movieProvider.value, movieId)
    movieCards.value = [
      {
        id: movieCardSerial++,
        provider: movieProvider.value,
        movieId,
        cover: item.cover || '',
        title: item.title || movieId,
        years: item.years || '',
        desc: item.desc || '',
        url: item.url || '',
        score: item.score || null,
      },
      ...movieCards.value,
    ]
    movieIdInput.value = ''
  } catch (error) {
    movieCreateError.value = error instanceof Error ? error.message : '创建电影卡片失败'
  } finally {
    movieCreating.value = false
  }
}
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

.movie-cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 12px;
}

.movie-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, rgba(24, 30, 41, 0.96), rgba(19, 24, 34, 0.96));
  color: #edf1ff;
  overflow: hidden;
}

.movie-cover {
  width: 100%;
  height: 320px;
}

.movie-cover--placeholder,
.movie-cover--empty {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 8px;
  color: #b7c4e4;
  background: linear-gradient(180deg, rgba(39, 47, 64, 0.8), rgba(31, 38, 53, 0.8));
}

.movie-card-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.movie-input-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 132px;
  gap: 8px;
}

.movie-meta-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.movie-title {
  font-size: 18px;
  font-weight: 700;
  color: #f2f5ff;
  line-height: 1.3;
}

.movie-years {
  margin-left: 6px;
  font-size: 14px;
  color: #aeb8cc;
  font-weight: 500;
}

.movie-desc {
  margin: 0;
  color: #d2dcf3;
  line-height: 1.6;
  font-size: 14px;
}

.movie-link-btn {
  align-self: flex-start;
  margin-left: -8px;
}

@media (max-width: 980px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .movie-input-row {
    grid-template-columns: 1fr;
  }
}
</style>
