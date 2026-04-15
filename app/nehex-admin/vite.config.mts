// Plugins
import Components from 'unplugin-vue-components/vite'
import Vue from '@vitejs/plugin-vue'
import Vuetify, { transformAssetUrls } from 'vite-plugin-vuetify'
import VueRouter from 'unplugin-vue-router/vite'

// Utilities
import { defineConfig, loadEnv } from 'vite'
import { fileURLToPath, URL } from 'node:url'
import { readFileSync } from 'node:fs'

const adminPackage = JSON.parse(
  readFileSync(new URL('./package.json', import.meta.url), 'utf-8'),
) as { version?: string }
const adminVersion = String(adminPackage.version ?? '').trim() || '1.2.4'

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const backendTarget = env.NEHEX_DEV_API_TARGET || `http://127.0.0.1:${env.APP_PORT || '7878'}`

  return {
    base: './',
    plugins: [
      VueRouter({
        dts: 'src/typed-router.d.ts',
      }),
      Vue({
        template: { transformAssetUrls },
      }),
      // https://github.com/vuetifyjs/vuetify-loader/tree/master/packages/vite-plugin#readme
      Vuetify({
        autoImport: true,
        styles: {
          configFile: 'src/styles/settings.scss',
        },
      }),
      Components({
        dts: 'src/components.d.ts',
      }),
    ],
    optimizeDeps: {
      exclude: [
        'vuetify',
        'vue-router',
        'unplugin-vue-router/runtime',
        'unplugin-vue-router/data-loaders',
        'unplugin-vue-router/data-loaders/basic',
      ],
    },
    define: {
      'process.env': {},
      __NEHEX_ADMIN_VERSION__: JSON.stringify(adminVersion),
    },
    resolve: {
      alias: {
        '@': fileURLToPath(new URL('src', import.meta.url)),
      },
      extensions: [
        '.js',
        '.json',
        '.jsx',
        '.mjs',
        '.ts',
        '.tsx',
        '.vue',
      ],
    },
    server: {
      // Allow LAN devices to access Vite dev server.
      host: '0.0.0.0',
      port: 3000,
      strictPort: true,
      proxy: {
        '^/(admin-api|setting|version|daily|album|project|friend)(/|$)': {
          target: backendTarget,
          changeOrigin: true,
        },
      },
    },
  }
})
