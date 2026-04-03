<template>
  <div class="admin-layout" :class="{ 'admin-layout--with-subnav': hasSecondaryNav }">
    <aside class="sidebar">
      <div class="sidebar-header">
        <div class="site-name">{{ adminTitle }}</div>
      </div>

      <v-list class="menu-list" density="comfortable" nav>
        <template v-for="item in menuItems" :key="item.to">
          <v-list-item
            class="menu-item"
            :prepend-icon="item.icon"
            rounded="lg"
            :title="item.label"
            :active="isMenuItemActive(item)"
            @click="handleMenuItemClick(item)"
          />

          <div v-if="item.children?.length && isSubmenuExpanded(item)" class="submenu-wrap">
            <v-list-item
              v-for="child in item.children"
              :key="child.to"
              class="submenu-item"
              rounded="lg"
              :title="child.label"
              :active="isSubmenuItemActive(child)"
              @click="handleSubmenuItemClick(child)"
            />
          </div>
        </template>
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

    <aside v-if="hasSecondaryNav" class="sub-sidebar">
      <slot name="secondary-nav" />
    </aside>

    <main class="content-wrap">
      <slot />
    </main>
  </div>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref, useSlots } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { fetchAdminTitle, getDefaultAdminTitle } from '@/services/settings'
import { clearAuthSession, getAuthenticatedAccount } from '@/utils/auth'

type MenuChildItem = {
  label: string
  to: string
  parentTo?: string
}

type MenuItem = {
  icon: string
  label: string
  to: string
  children?: MenuChildItem[]
}

const menuItems: MenuItem[] = [
  { icon: 'mdi-view-dashboard-outline', label: '仪表盘', to: '/' },
  {
    icon: 'mdi-post-outline',
    label: '文章管理',
    to: '/articles',
    children: [
      { label: '管理', to: '/articles', parentTo: '/articles' },
      { label: '新增', to: '/articles/new' },
    ],
  },
  {
    icon: 'mdi-calendar-text-outline',
    label: '日常管理',
    to: '/dailies',
    children: [
      { label: '管理', to: '/dailies', parentTo: '/dailies' },
      { label: '新增', to: '/dailies/new' },
    ],
  },
  {
    icon: 'mdi-image-multiple-outline',
    label: '相册管理',
    to: '/albums',
    children: [
      { label: '管理', to: '/albums', parentTo: '/albums' },
      { label: '新增', to: '/albums/new' },
    ],
  },
  {
    icon: 'mdi-file-document-outline',
    label: '独立页管理',
    to: '/pages',
    children: [
      { label: '管理', to: '/pages', parentTo: '/pages' },
      { label: '新增', to: '/pages/new' },
    ],
  },
  { icon: 'mdi-comment-multiple-outline', label: '评论管理', to: '/comments' },
  { icon: 'mdi-link-variant', label: '友链管理', to: '/friends' },
  {
    icon: 'mdi-briefcase-outline',
    label: '项目管理',
    to: '/projects',
    children: [
      { label: '管理', to: '/projects', parentTo: '/projects' },
      { label: '新增', to: '/projects/new' },
    ],
  },
  { icon: 'mdi-cog-outline', label: '站点设置', to: '/settings' },
]

const router = useRouter()
const route = useRoute()
const accountName = ref(getAuthenticatedAccount())
const adminTitle = ref(getDefaultAdminTitle())
const expandedMenuKey = ref<string | null>(getDefaultExpandedMenuKey())
const slots = useSlots()
const hasSecondaryNav = computed(() => Boolean(slots['secondary-nav']))

onMounted(async () => {
  try {
    adminTitle.value = await fetchAdminTitle()
  } catch (error) {
    console.warn('Failed to load admin title from /setting', error)
  }
})

async function handleLogout(): Promise<void> {
  clearAuthSession()
  await router.replace('/login')
}

function isMenuItemActive(item: MenuItem): boolean {
  if (item.to === '/') {
    return route.path === '/'
  }
  return route.path === item.to || route.path.startsWith(`${item.to}/`)
}

function isSubmenuItemActive(item: MenuChildItem): boolean {
  if (item.parentTo && item.to === item.parentTo) {
    return route.path === item.parentTo || route.path.startsWith(`${item.parentTo}/edit/`)
  }
  return route.path === item.to
}

function handleMenuItemClick(item: MenuItem): void {
  if (item.children?.length) {
    expandedMenuKey.value = item.to

    if (route.path !== item.to) {
      void router.push(item.to)
    }
    return
  }

  if (route.path === item.to) {
    return
  }
  void router.push(item.to)
}

function handleSubmenuItemClick(item: MenuChildItem): void {
  if (route.path === item.to) {
    return
  }
  void router.push(item.to)
}

function isSubmenuExpanded(item: MenuItem): boolean {
  if (expandedMenuKey.value === item.to) {
    return true
  }
  return route.path === item.to || route.path.startsWith(`${item.to}/`)
}

function getDefaultExpandedMenuKey(): string | null {
  const matched = menuItems.find((item) => {
    if (!item.children?.length) {
      return false
    }
    return route.path === item.to || route.path.startsWith(`${item.to}/`)
  })

  return matched?.to ?? null
}
</script>

<style scoped>
.admin-layout {
  min-height: 100vh;
  display: grid;
  grid-template-columns: 244px minmax(0, 1fr);
  background: #0d1118;
}

.admin-layout--with-subnav {
  grid-template-columns: 244px 224px minmax(0, 1fr);
}

.sidebar {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 18px 14px 14px;
  border-right: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, #161b24 0%, #131821 100%);
}

.sub-sidebar {
  padding: 18px 14px 14px;
  border-right: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(180deg, #141a24 0%, #111722 100%);
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
    background 0.22s ease,
    color 0.22s ease;
}

:deep(.menu-item:hover) {
  color: #f4f7ff;
  background: linear-gradient(90deg, rgba(103, 121, 170, 0.16) 0%, rgba(112, 133, 186, 0.3) 100%);
}

:deep(.menu-item.v-list-item--active) {
  color: #ffffff;
  background: linear-gradient(90deg, rgba(103, 121, 170, 0.3) 0%, rgba(112, 133, 186, 0.48) 100%);
}

:deep(.menu-item.v-list-item--active:hover) {
  background: linear-gradient(90deg, rgba(103, 121, 170, 0.34) 0%, rgba(112, 133, 186, 0.52) 100%);
}

.submenu-wrap {
  margin: -2px 0 6px;
}

:deep(.submenu-item .v-list-item-title) {
  font-size: 14px;
  font-weight: 600;
}

:deep(.submenu-item) {
  min-height: 38px;
  margin: 0 0 4px 28px;
  color: #aab7d5;
  border: 1px solid transparent;
  transition:
    background 0.22s ease,
    color 0.22s ease;
}

:deep(.submenu-item:hover) {
  color: #eef3ff;
  background: linear-gradient(90deg, rgba(90, 108, 151, 0.16) 0%, rgba(107, 128, 184, 0.24) 100%);
}

:deep(.submenu-item.v-list-item--active) {
  color: #ffffff;
  background: linear-gradient(90deg, rgba(103, 121, 170, 0.28) 0%, rgba(112, 133, 186, 0.44) 100%);
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
  .admin-layout,
  .admin-layout--with-subnav {
    grid-template-columns: 1fr;
  }

  .sidebar {
    position: sticky;
    top: 0;
    z-index: 6;
    border-right: 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .sub-sidebar {
    position: static;
    border-right: 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    padding: 12px 14px;
  }

  .menu-list {
    display: block;
  }

  :deep(.menu-item) {
    margin-bottom: 0;
  }

  :deep(.submenu-item) {
    margin-left: 14px;
  }
}
</style>
