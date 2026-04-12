<template>
  <AdminLayout>
    <section v-if="isManageRoute" class="projects-page">
      <header class="page-header">
        <div class="header-text">
          <h1>项目管理</h1>
          <p>统一管理项目条目，支持编辑、删除和新增。</p>
        </div>
        <v-btn
          class="quick-create-btn"
          color="primary"
          prepend-icon="mdi-plus"
          variant="flat"
          @click="openCreatePage"
        >
          新增项目
        </v-btn>
      </header>

      <v-progress-linear
        v-if="loading"
        class="mb-4"
        color="primary"
        indeterminate
      />

      <div class="projects-grid">
        <v-card
          v-for="project in projects"
          :key="project.id"
          class="project-card"
          :style="getProjectCardStyle(project)"
          rounded="xl"
        >
          <div class="project-content">
            <div class="card-actions">
              <v-btn
                class="icon-btn"
                color="white"
                icon="mdi-pencil-outline"
                size="small"
                variant="text"
                @click.stop="openEditPage(project)"
              />
              <v-btn
                class="icon-btn"
                color="error"
                icon="mdi-delete-outline"
                size="small"
                variant="text"
                @click.stop="openDeleteDialog(project)"
              />
            </div>

            <div class="card-footer">
              <div class="project-title">{{ project.title }}</div>
              <div class="project-meta">
                <span>{{ project.category || '未分类' }}</span>
                <span>{{ project.status > 0 ? '启用' : '禁用' }}</span>
                <span>Sort {{ project.sort }}</span>
              </div>
            </div>
          </div>
        </v-card>

        <v-card
          class="add-card"
          rounded="xl"
          @click="openCreatePage"
        >
          <v-icon class="add-icon" icon="mdi-plus-circle-outline" size="40" />
          <div class="add-label">新增项目</div>
        </v-card>
      </div>
    </section>

    <v-dialog v-if="isManageRoute" v-model="deleteDialog" max-width="420">
      <v-card class="dialog-card" rounded="xl">
        <v-card-title class="dialog-title">确认删除</v-card-title>
        <v-card-text>
          即将删除项目《{{ pendingDelete?.title || '' }}》，删除后不可恢复。
        </v-card-text>
        <v-card-actions class="dialog-actions">
          <v-spacer />
          <v-btn variant="text" @click="closeDeleteDialog">取消</v-btn>
          <v-btn color="error" :loading="deleting" @click="confirmDelete">
            确认删除
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <RouterView v-else />
  </AdminLayout>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref, watch } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import {
  deleteProject,
  fetchProjects,
  type ProjectItem,
} from '@/services/projects'

const router = useRouter()
const route = useRoute()
const isManageRoute = computed(() => route.path === '/projects')
const { showGlobalSuccess, showGlobalError } = useGlobalSnackbar()

const loading = ref(false)
const deleting = ref(false)
const errorMessage = ref('')

const projects = ref<ProjectItem[]>([])
const deleteDialog = ref(false)
const pendingDelete = ref<ProjectItem | null>(null)

function openCreatePage(): void {
  void router.push('/projects/new')
}

function openEditPage(project: ProjectItem): void {
  void router.push(`/projects/edit/${project.id}`)
}

function openDeleteDialog(project: ProjectItem): void {
  pendingDelete.value = project
  deleteDialog.value = true
}

function closeDeleteDialog(force = false): void {
  if (deleting.value && !force) {
    return
  }
  deleteDialog.value = false
  pendingDelete.value = null
}

async function loadProjects(): Promise<void> {
  loading.value = true
  errorMessage.value = ''
  try {
    projects.value = await fetchProjects()
  } catch (error) {
    const message = error instanceof Error ? error.message : '加载项目失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    loading.value = false
  }
}

async function confirmDelete(): Promise<void> {
  if (!pendingDelete.value) {
    return
  }

  deleting.value = true
  errorMessage.value = ''
  try {
    await deleteProject(pendingDelete.value.id)
    closeDeleteDialog(true)
    showGlobalSuccess('项目已删除')
    await loadProjects()
  } catch (error) {
    const message = error instanceof Error ? error.message : '删除项目失败'
    errorMessage.value = message
    showGlobalError(message)
  } finally {
    deleting.value = false
  }
}

function getProjectCardStyle(project: ProjectItem): Record<string, string> {
  const cover = (project.cover || '').trim()
  if (!cover) {
    return {
      background: 'linear-gradient(140deg, #22243f 0%, #161b2d 100%)',
    }
  }

  const safeUrl = cover.replace(/"/g, '\\"')
  return {
    backgroundImage: `url("${safeUrl}")`,
    backgroundPosition: 'center',
    backgroundRepeat: 'no-repeat',
    backgroundSize: 'cover',
  }
}

onMounted(async () => {
  if (isManageRoute.value) {
    await loadProjects()
  }
})

watch(isManageRoute, async (active, previous) => {
  if (active && !previous) {
    await loadProjects()
  }
})
</script>

<style scoped>
.projects-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.page-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 14px;
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

.quick-create-btn {
  flex-shrink: 0;
}

.projects-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 16px;
}

.project-card,
.add-card {
  position: relative;
  min-height: 240px;
  border: 1px solid rgba(255, 255, 255, 0.12);
}

.project-card {
  overflow: hidden;
  transition:
    transform 0.24s ease,
    box-shadow 0.24s ease;
}

.project-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 16px 30px rgba(0, 0, 0, 0.28);
}

.project-content {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 12px;
}

.card-actions {
  display: flex;
  justify-content: flex-end;
  gap: 4px;
}

.icon-btn {
  border-radius: 10px;
}

.card-footer {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
}

.project-title {
  font-size: 20px;
  font-weight: 700;
  color: #ffffff;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.45);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #d4dfff;
  font-size: 13px;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.45);
}

.add-card {
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 8px;
  border-style: dashed;
  color: #b8c5e6;
  background: linear-gradient(140deg, rgba(28, 34, 47, 0.92), rgba(19, 24, 36, 0.92));
  transition:
    border-color 0.2s ease,
    color 0.2s ease,
    filter 0.2s ease;
}

.add-card:hover {
  border-color: rgba(255, 255, 255, 0.34);
  color: #e8efff;
  filter: brightness(1.04);
}

.add-icon {
  opacity: 0.95;
  font-size: 52px !important;
}

.add-label {
  font-size: 15px;
  font-weight: 600;
}

.dialog-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, rgba(24, 30, 41, 0.96), rgba(19, 24, 34, 0.96));
  color: #edf1ff;
}

.dialog-title {
  font-size: 18px;
  font-weight: 700;
}

.dialog-actions {
  padding: 8px 16px 14px;
}

@media (max-width: 760px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .quick-create-btn {
    width: 100%;
  }
}
</style>
