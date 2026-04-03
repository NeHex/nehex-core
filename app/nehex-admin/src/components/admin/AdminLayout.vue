<template>
  <div class="admin-layout">
    <aside class="sidebar">
      <div class="sidebar-header">
        <div class="site-name">NeHex Admin</div>
      </div>

      <v-list class="menu-list" density="comfortable" nav>
        <v-list-item
          v-for="item in menuItems"
          :key="item.to"
          class="menu-item"
          :prepend-icon="item.icon"
          rounded="lg"
          :title="item.label"
          :to="item.to"
        />
      </v-list>

      <div class="sidebar-footer">
        <div class="profile-info">
          <div class="profile-label">个人资料</div>
          <div class="profile-name">{{ accountName || '站长' }}</div>
        </div>
        <v-btn
          class="logout-btn"
          color="error"
          icon="mdi-logout"
          size="small"
          variant="tonal"
          @click="handleLogout"
        />
      </div>
    </aside>

    <main class="content-wrap">
      <slot />
    </main>
  </div>
</template>

<script lang="ts" setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { clearAuthSession, getAuthenticatedAccount } from '@/utils/auth'

type MenuItem = {
  icon: string
  label: string
  to: string
}

const menuItems: MenuItem[] = [
  { icon: 'mdi-view-dashboard-outline', label: '仪表盘', to: '/' },
  { icon: 'mdi-post-outline', label: '文章管理', to: '/articles' },
  { icon: 'mdi-calendar-text-outline', label: '日常管理', to: '/dailies' },
  { icon: 'mdi-image-multiple-outline', label: '相册管理', to: '/albums' },
  { icon: 'mdi-file-document-outline', label: '独立页管理', to: '/pages' },
  { icon: 'mdi-comment-multiple-outline', label: '评论管理', to: '/comments' },
  { icon: 'mdi-briefcase-outline', label: '项目管理', to: '/projects' },
  { icon: 'mdi-cog-outline', label: '站点设置', to: '/settings' },
]

const router = useRouter()
const accountName = ref(getAuthenticatedAccount())

async function handleLogout(): Promise<void> {
  clearAuthSession()
  await router.replace('/login')
}
</script>

<style scoped>
.admin-layout {
  min-height: 100vh;
  display: grid;
  grid-template-columns: 244px minmax(0, 1fr);
  background: #0d1118;
}

.sidebar {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 18px 14px 14px;
  border-right: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, #161b24 0%, #131821 100%);
}

.sidebar-header {
  padding: 6px 8px 2px;
}

.site-name {
  font-size: 18px;
  font-weight: 700;
  color: #f2f5ff;
  letter-spacing: 0.4px;
}

.menu-list {
  padding: 6px 0 0;
  flex: 1;
  background: transparent;
}

:deep(.menu-item .v-list-item-title) {
  font-size: 16px;
  font-weight: 600;
  transition: color 0.22s ease;
}

:deep(.menu-item) {
  min-height: 44px;
  margin-bottom: 6px;
  color: #b8c0d4;
  border: 1px solid transparent;
  transition:
    transform 0.22s ease,
    background-color 0.22s ease,
    border-color 0.22s ease,
    box-shadow 0.22s ease,
    color 0.22s ease;
}

:deep(.menu-item:hover) {
  color: #f4f7ff;
  transform: translateX(4px);
  border-color: rgba(255, 255, 255, 0.16);
  background-color: rgba(255, 255, 255, 0.08);
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.22);
}

:deep(.menu-item.v-list-item--active) {
  color: #ffffff;
  border-color: rgba(255, 255, 255, 0.2);
  background-color: rgba(103, 121, 170, 0.36);
  box-shadow: 0 10px 22px rgba(0, 0, 0, 0.22);
}

:deep(.menu-item.v-list-item--active:hover) {
  transform: translateX(4px);
  background-color: rgba(112, 133, 186, 0.42);
}

.sidebar-footer {
  margin-top: auto;
  padding: 10px 10px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-radius: 14px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.03);
}

.profile-info {
  min-width: 0;
}

.profile-label {
  font-size: 13px;
  color: #aeb8cd;
}

.profile-name {
  font-size: 14px;
  color: #f5f7ff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.logout-btn {
  flex-shrink: 0;
}

.content-wrap {
  padding: 22px;
  min-width: 0;
}

@media (max-width: 980px) {
  .admin-layout {
    grid-template-columns: 1fr;
  }

  .sidebar {
    position: sticky;
    top: 0;
    z-index: 6;
    border-right: 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .menu-list {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  :deep(.menu-item) {
    margin-bottom: 0;
  }
}
</style>
