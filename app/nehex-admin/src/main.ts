/**
 * main.ts
 *
 * Bootstraps Vuetify and other plugins then mounts the App`
 */

// Plugins
import { registerPlugins } from '@/plugins'
import { fetchAdminTitle, getDefaultAdminTitle } from '@/services/settings'

// Components
import App from './App.vue'

// Composables
import { createApp } from 'vue'

// Styles

const app = createApp(App)

registerPlugins(app)

app.mount('#app')

document.title = getDefaultAdminTitle()
void fetchAdminTitle()
  .then((title) => {
    document.title = title
  })
  .catch((error) => {
    console.warn('Failed to load admin title from /setting', error)
  })
