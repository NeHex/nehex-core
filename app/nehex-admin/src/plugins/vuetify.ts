/**
 * plugins/vuetify.ts
 *
 * Framework documentation: https://vuetifyjs.com`
 */

// Styles
import '@mdi/font/css/materialdesignicons.css'
import 'vuetify/styles'

// Composables
import { createVuetify } from 'vuetify'

// https://vuetifyjs.com/en/introduction/why-vuetify/#feature-guides
export default createVuetify({
  theme: {
    defaultTheme: 'nehex-dark',
    themes: {
      'nehex-dark': {
        dark: true,
        colors: {
          background: '#0D1118',
          surface: '#111826',
          primary: '#7EA8FF',
          secondary: '#5F7BC0',
          success: '#4CAF50',
          warning: '#FB8C00',
          error: '#CF6679',
          info: '#2196F3',
        },
      },
      'nehex-light': {
        dark: false,
        colors: {
          background: '#F8F4E9',
          surface: '#FFFFFF',
          primary: '#4A6FA5',
          secondary: '#6B84B1',
          success: '#4CAF50',
          warning: '#FB8C00',
          error: '#B00020',
          info: '#2196F3',
        },
      },
    },
  },
})
