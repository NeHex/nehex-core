<template>
  <div
    class="admin-layout"
    :class="{
      'admin-layout--with-subnav': hasSecondaryNav && !isMobile,
      'admin-layout--mobile': isMobile,
    }"
  >
    <v-app-bar
      v-if="isMobile"
      class="mobile-topbar"
      color="#161b24"
      density="comfortable"
      flat
    >
      <v-btn
        aria-label="打开导航菜单"
        icon="mdi-menu"
        variant="text"
        @click="toggleMainDrawer"
      />

      <v-app-bar-title class="mobile-topbar-title">{{ adminTitle }}</v-app-bar-title>

      <v-spacer />

      <v-btn
        v-if="hasSecondaryNav"
        aria-label="打开二级导航"
        icon="mdi-tune-variant"
        variant="text"
        @click="toggleSecondaryDrawer"
      />

      <v-menu location="bottom end">
        <template #activator="{ props }">
          <v-btn
            v-bind="props"
            aria-label="更多操作"
            icon="mdi-dots-vertical"
            variant="text"
          />
        </template>

        <v-list density="comfortable" min-width="170">
          <v-list-item
            prepend-icon="mdi-open-in-new"
            title="前往站点"
            @click="handleGoToSiteFromMenu"
          />
          <v-list-item
            prepend-icon="mdi-logout"
            title="登出"
            @click="handleLogout"
          />
        </v-list>
      </v-menu>
    </v-app-bar>

    <v-navigation-drawer
      v-if="isMobile"
      v-model="mobileMainDrawer"
      class="mobile-drawer mobile-drawer--main"
      location="left"
      :scrim="true"
      temporary
      width="288"
    >
      <div class="mobile-drawer-content">
        <div class="sidebar-header sidebar-header--mobile">
          <div class="site-name">{{ adminTitle }}</div>
          <v-btn
            aria-label="关闭导航菜单"
            icon="mdi-close"
            size="small"
            variant="text"
            @click="mobileMainDrawer = false"
          />
        </div>

        <v-list class="menu-list" density="comfortable" nav>
          <template v-for="item in menuItems" :key="item.to">
            <hr v-if="item.dividerBefore" class="menu-divider">
            <v-list-item
              class="menu-item"
              :append-icon="item.children?.length ? (isSubmenuExpanded(item) ? 'mdi-chevron-down' : 'mdi-chevron-right') : undefined"
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

        <div class="sidebar-footer sidebar-footer--mobile">
          <v-btn
            class="site-btn"
            block
            color="primary"
            prepend-icon="mdi-open-in-new"
            variant="tonal"
            @click="handleGoToSiteFromMenu"
          >
            前往站点
          </v-btn>
          <v-btn
            class="logout-btn"
            block
            color="error"
            prepend-icon="mdi-logout"
            variant="tonal"
            @click="handleLogout"
          >
            登出
          </v-btn>
        </div>
      </div>
    </v-navigation-drawer>

    <aside v-else class="sidebar">
      <div class="sidebar-header">
        <div class="site-name">{{ adminTitle }}</div>
      </div>

      <v-list class="menu-list" density="comfortable" nav>
        <template v-for="item in menuItems" :key="item.to">
          <hr v-if="item.dividerBefore" class="menu-divider">
          <v-list-item
            class="menu-item"
            :append-icon="item.children?.length ? (isSubmenuExpanded(item) ? 'mdi-chevron-down' : 'mdi-chevron-right') : undefined"
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
        <v-btn
          class="site-btn"
          color="primary"
          prepend-icon="mdi-open-in-new"
          size="small"
          variant="tonal"
          @click="goToSite"
        >
          前往站点
        </v-btn>
        <v-btn
          class="logout-btn"
          color="error"
          prepend-icon="mdi-logout"
          size="small"
          variant="tonal"
          @click="handleLogout"
        >
          登出
        </v-btn>
      </div>
    </aside>

    <v-navigation-drawer
      v-if="isMobile && hasSecondaryNav"
      v-model="mobileSecondaryDrawer"
      class="mobile-drawer mobile-drawer--secondary"
      location="right"
      :scrim="true"
      temporary
      width="288"
    >
      <div class="mobile-secondary-nav">
        <div class="mobile-secondary-head">
          <div class="mobile-secondary-title">页面导航</div>
          <v-btn
            aria-label="关闭二级导航"
            icon="mdi-close"
            size="small"
            variant="text"
            @click="mobileSecondaryDrawer = false"
          />
        </div>
        <div class="mobile-secondary-body">
          <slot name="secondary-nav" />
        </div>
      </div>
    </v-navigation-drawer>

    <aside v-if="hasSecondaryNav && !isMobile" class="sub-sidebar">
      <slot name="secondary-nav" />
    </aside>

    <main class="content-wrap" :class="{ 'content-wrap--mobile': isMobile }">
      <slot />
    </main>
  </div>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref, useSlots, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDisplay } from 'vuetify'
import { adminLogout, resetAdminSessionCache } from '@/services/admin-api'
import { fetchAdminTitle, fetchSiteUrl, getDefaultAdminTitle } from '@/services/settings'
import { clearAuthSession } from '@/utils/auth'

type MenuChildItem = {
  label: string
  to: string
  parentTo?: string
}

type MenuItem = {
  icon: string
  label: string
  to: string
  dividerBefore?: boolean
  children?: MenuChildItem[]
}

const menuItems: MenuItem[] = [
  { icon: 'mdi-view-dashboard-outline', label: '仪表盘', to: '/' },
  {
    icon: 'mdi-post-outline',
    label: '文章管理',
    to: '/articles',
    dividerBefore: true,
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
  {
    icon: 'mdi-folder-image',
    label: '媒体库',
    to: '/media',
    dividerBefore: true,
  },
  {
    icon: 'mdi-console',
    label: '开发者选项',
    to: '/developer-options',
  },
  {
    icon: 'mdi-cog-outline',
    label: '设定',
    to: '/settings',
    children: [
      { label: '基础设置', to: '/settings', parentTo: '/settings' },
      { label: '通知设置', to: '/settings/mail-notify' },
      { label: '邮件管理', to: '/settings/mail-management' },
      { label: '备份与恢复', to: '/settings/backup-restore' },
    ],
  },
]

const router = useRouter()
const route = useRoute()
const display = useDisplay()

const adminTitle = ref(getDefaultAdminTitle())
const expandedMenuKey = ref<string | null>(getDefaultExpandedMenuKey())
const slots = useSlots()
const hasSecondaryNav = computed(() => Boolean(slots['secondary-nav']))
const isMobile = computed(() => display.mdAndDown.value)
const mobileMainDrawer = ref(false)
const mobileSecondaryDrawer = ref(false)

onMounted(async () => {
  try {
    adminTitle.value = await fetchAdminTitle()
  } catch (error) {
    console.warn('Failed to load admin title from /setting', error)
  }
})

watch(
  () => route.fullPath,
  () => {
    closeMobileDrawers()
  },
)

watch(isMobile, (nextMobile) => {
  if (!nextMobile) {
    mobileMainDrawer.value = false
    mobileSecondaryDrawer.value = false
  }
})

async function handleLogout(): Promise<void> {
  closeMobileDrawers()
  clearAuthSession()
  resetAdminSessionCache()
  try {
    await adminLogout()
  } catch (error) {
    console.warn('Admin logout request failed', error)
  }
  await router.replace('/login')
}

async function goToSite(): Promise<void> {
  let targetUrl = '/'

  try {
    const siteUrl = await fetchSiteUrl()
    if (siteUrl) {
      targetUrl = siteUrl
    }
  } catch (error) {
    console.warn('Failed to load site_url from /setting', error)
  }

  window.open(targetUrl, '_blank', 'noopener')
}

function handleGoToSiteFromMenu(): void {
  closeMobileDrawers()
  void goToSite()
}

function toggleMainDrawer(): void {
  mobileMainDrawer.value = !mobileMainDrawer.value
}

function toggleSecondaryDrawer(): void {
  mobileSecondaryDrawer.value = !mobileSecondaryDrawer.value
}

function isMenuItemActive(item: MenuItem): boolean {
  if (item.children?.length) {
    return hasActiveSubmenuItem(item) || route.path === item.to
  }
  if (item.to === '/') {
    return route.path === '/'
  }
  return route.path === item.to || route.path.startsWith(`${item.to}/`)
}

function isSubmenuItemActive(item: MenuChildItem): boolean {
  if (item.parentTo && item.to === item.parentTo) {
    return route.path === item.parentTo || route.path.startsWith(`${item.parentTo}/edit/`)
  }
  return route.path === item.to || route.path.startsWith(`${item.to}/`)
}

function handleMenuItemClick(item: MenuItem): void {
  if (item.children?.length) {
    expandedMenuKey.value = expandedMenuKey.value === item.to ? null : item.to
    return
  }

  if (route.path === item.to) {
    closeMobileDrawers()
    return
  }

  void router.push(item.to)
}

function handleSubmenuItemClick(item: MenuChildItem): void {
  if (route.path === item.to) {
    closeMobileDrawers()
    return
  }
  void router.push(item.to)
}

function isSubmenuExpanded(item: MenuItem): boolean {
  if (expandedMenuKey.value === item.to) {
    return true
  }
  if (item.children?.length) {
    return route.path === item.to || hasActiveSubmenuItem(item)
  }
  return route.path === item.to || route.path.startsWith(`${item.to}/`)
}

function getDefaultExpandedMenuKey(): string | null {
  const matched = menuItems.find((item) => {
    if (!item.children?.length) {
      return false
    }
    return route.path === item.to || hasActiveSubmenuItem(item)
  })

  return matched?.to ?? null
}

function hasActiveSubmenuItem(item: MenuItem): boolean {
  if (!item.children?.length) {
    return false
  }
  return item.children.some((child) => isSubmenuItemActive(child))
}

function closeMobileDrawers(): void {
  if (!isMobile.value) {
    return
  }
  mobileMainDrawer.value = false
  mobileSecondaryDrawer.value = false
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

.mobile-topbar {
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  backdrop-filter: blur(10px);
}

.mobile-topbar-title {
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 0.3px;
  color: #f2f5ff;
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

.mobile-drawer-content,
.mobile-secondary-nav {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 16px 14px 14px;
  background: linear-gradient(180deg, #161b24 0%, #131821 100%);
  overflow: hidden;
}

.mobile-secondary-nav {
  background: linear-gradient(180deg, #141a24 0%, #111722 100%);
}

.mobile-secondary-title {
  padding: 4px 8px;
  font-size: 15px;
  font-weight: 700;
  color: #f2f5ff;
}

.sidebar-header {
  padding: 6px 8px 2px;
}

.sidebar-header--mobile {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.site-name {
  font-size: 18px;
  font-weight: 700;
  color: #f2f5ff;
  letter-spacing: 0.4px;
}

.mobile-secondary-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.mobile-secondary-body {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  -webkit-overflow-scrolling: touch;
}

.menu-list {
  padding: 6px 0 0;
  flex: 1;
  background: transparent;
}

.mobile-drawer-content .menu-list {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  -webkit-overflow-scrolling: touch;
  padding-bottom: 8px;
}

.menu-divider {
  border: 0;
  height: 1px;
  margin: 6px 8px 10px;
  background: linear-gradient(
    90deg,
    rgba(120, 138, 183, 0) 0%,
    rgba(154, 176, 228, 0.38) 18%,
    rgba(154, 176, 228, 0.38) 82%,
    rgba(120, 138, 183, 0) 100%
  );
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
  justify-content: flex-end;
  gap: 8px;
  border-radius: 14px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.03);
}

.sidebar-footer--mobile {
  flex-direction: column;
  align-items: stretch;
  justify-content: flex-start;
  flex-shrink: 0;
}

.site-btn,
.logout-btn {
  flex-shrink: 0;
}

.content-wrap {
  padding: 22px;
  min-width: 0;
}

@media (max-width: 980px) {
  .admin-layout,
  .admin-layout--with-subnav,
  .admin-layout--mobile {
    display: block;
  }

  .content-wrap {
    padding: 16px 12px;
  }

  .content-wrap--mobile {
    padding-bottom: max(16px, env(safe-area-inset-bottom));
  }

  :deep(.submenu-item) {
    margin-left: 14px;
  }
}
</style>
