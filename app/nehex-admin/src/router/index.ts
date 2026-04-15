/**
 * router/index.ts
 *
 * Automatic routes for `./src/pages/*.vue`
 */

// Composables
import { createRouter, createWebHistory } from 'vue-router'
import { routes } from 'vue-router/auto-routes'
import { clearAuthSession, setAuthSession } from '@/utils/auth'
import { getAdminBasePath, toRouterBase } from '@/utils/path'
import { fetchInstallStatus } from '@/services/install'
import { fetchAdminSession } from '@/services/admin-api'
import { useRouteLoading } from '@/composables/useRouteLoading'

const LOGIN_PATH = '/login'
const INSTALL_PATH = '/install'
const HOME_PATH = '/'

const router = createRouter({
  history: createWebHistory(toRouterBase(getAdminBasePath())),
  routes,
})
const { startRouteLoading, finishRouteLoading } = useRouteLoading()

router.beforeEach(async (to) => {
  startRouteLoading()
  const installStatus = await fetchInstallStatus().catch(() => ({
    installed: true,
    schema_ready: true,
    table_count: 0,
    admin_manager_web: getAdminBasePath(),
  }))

  if (!installStatus.installed) {
    if (to.path !== INSTALL_PATH) {
      return {
        path: INSTALL_PATH,
      }
    }
    return true
  }

  const session = await fetchAdminSession().catch(() => null)
  const authed = Boolean(session?.account)
  if (authed && session?.account) {
    setAuthSession(session.account)
  } else {
    clearAuthSession()
  }

  if (to.path === INSTALL_PATH) {
    return authed ? HOME_PATH : LOGIN_PATH
  }

  if (to.path === LOGIN_PATH) {
    if (authed) {
      const redirect = typeof to.query.redirect === 'string' ? to.query.redirect : HOME_PATH
      return redirect
    }
    return true
  }

  if (!authed) {
    return {
      path: LOGIN_PATH,
      query: { redirect: to.fullPath },
    }
  }

  return true
})

router.afterEach(() => {
  finishRouteLoading()
})

// Workaround for https://github.com/vitejs/vite/issues/11804
router.onError((err, to) => {
  finishRouteLoading()
  if (err?.message?.includes?.('Failed to fetch dynamically imported module')) {
    if (localStorage.getItem('vuetify:dynamic-reload')) {
      console.error('Dynamic import error, reloading page did not fix it', err)
    } else {
      console.log('Reloading page to fix dynamic import error')
      localStorage.setItem('vuetify:dynamic-reload', 'true')
      location.assign(to.fullPath)
    }
  } else {
    console.error(err)
  }
})

router.isReady().then(() => {
  localStorage.removeItem('vuetify:dynamic-reload')
})

export default router
