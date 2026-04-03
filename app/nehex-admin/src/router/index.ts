/**
 * router/index.ts
 *
 * Automatic routes for `./src/pages/*.vue`
 */

// Composables
import { createRouter, createWebHistory } from 'vue-router'
import { routes } from 'vue-router/auto-routes'
import { isAuthenticated } from '@/utils/auth'
import { getAdminBasePath, toRouterBase } from '@/utils/path'

const LOGIN_PATH = '/login'
const HOME_PATH = '/'

const router = createRouter({
  history: createWebHistory(toRouterBase(getAdminBasePath())),
  routes,
})

router.beforeEach((to) => {
  const authed = isAuthenticated()

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

// Workaround for https://github.com/vitejs/vite/issues/11804
router.onError((err, to) => {
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
