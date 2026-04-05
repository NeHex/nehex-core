<template>
  <div class="login-page" :style="loginPageStyle">
    <v-card class="login-card" elevation="16" rounded="xl">
      <div class="welcome-wrap">
        <img alt="welcome" class="welcome-image" src="/welcome_zip.png">
      </div>

      <v-alert
        v-if="errorMessage"
        class="error-alert"
        density="comfortable"
        type="error"
        variant="tonal"
      >
        {{ errorMessage }}
      </v-alert>

      <v-form @submit.prevent="handleLogin">
        <div class="field-group">
          <div class="field-label">
            <v-icon class="label-icon" icon="mdi-account" />
            <span>用户名</span>
          </div>
          <v-text-field
            v-model="account"
            autocomplete="username"
            class="login-input"
            hide-details
            placeholder="请输入用户名"
            variant="solo"
          />
        </div>

        <div class="field-group">
          <div class="field-label">
            <v-icon class="label-icon" icon="mdi-lock" />
            <span>密码</span>
          </div>
          <v-text-field
            v-model="password"
            autocomplete="current-password"
            class="login-input"
            hide-details
            placeholder="请输入密码"
            type="password"
            variant="solo"
          />
        </div>

        <div class="field-group">
          <div class="field-label">
            <v-icon class="label-icon" icon="mdi-shield-check" />
            <span>验证码</span>
          </div>
          <div class="captcha-row">
            <v-text-field
              v-model="captchaInput"
              class="login-input captcha-input"
              hide-details
              placeholder="验证码"
              variant="solo"
            />
            <canvas
              ref="captchaCanvas"
              class="captcha-canvas"
              height="50"
              width="152"
              @click="refreshCaptcha"
            />
          </div>
          <button class="captcha-hint" type="button" @click="refreshCaptcha">
            点击验证码图片刷新
          </button>
        </div>

        <v-btn
          block
          class="login-btn mt-6"
          :loading="loading"
          size="x-large"
          type="submit"
        >
          登录
        </v-btn>
      </v-form>
    </v-card>
  </div>
</template>

<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { adminLogin, fetchAdminSession } from '@/services/admin-api'
import { fetchThemeBackgroundUrl } from '@/services/settings'
import { setAuthSession } from '@/utils/auth'

const route = useRoute()
const router = useRouter()

const account = ref('')
const password = ref('')
const captchaInput = ref('')
const captchaCode = ref('')
const captchaCanvas = ref<HTMLCanvasElement | null>(null)
const loading = ref(false)
const errorMessage = ref('')
const loginPageStyle = ref<Record<string, string>>({})

function getRedirectPath(): string {
  const redirect = typeof route.query.redirect === 'string' ? route.query.redirect : '/'
  if (!redirect || redirect === '/login') {
    return '/'
  }
  return redirect
}

function randomInt(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min
}

function makeCaptchaCode(length = 4): string {
  const source = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'
  let code = ''

  for (let i = 0; i < length; i += 1) {
    code += source[randomInt(0, source.length - 1)]
  }
  return code
}

function drawCaptcha(): void {
  const canvas = captchaCanvas.value
  if (!canvas) {
    return
  }

  const ctx = canvas.getContext('2d')
  if (!ctx) {
    return
  }

  const width = canvas.width
  const height = canvas.height

  ctx.clearRect(0, 0, width, height)

  const gradient = ctx.createLinearGradient(0, 0, width, height)
  gradient.addColorStop(0, '#d4fff4')
  gradient.addColorStop(1, '#b7ede2')
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, width, height)

  for (let i = 0; i < 4; i += 1) {
    const x1 = randomInt(0, width)
    const y1 = randomInt(0, height)
    const x2 = randomInt(0, width)
    const y2 = randomInt(0, height)
    ctx.strokeStyle = `rgba(75, 65, 95, ${Math.random() * 0.35 + 0.2})`
    ctx.lineWidth = 1
    ctx.beginPath()
    ctx.moveTo(x1, y1)
    ctx.lineTo(x2, y2)
    ctx.stroke()
  }

  for (let i = 0; i < 36; i += 1) {
    ctx.fillStyle = `rgba(90, 85, 120, ${Math.random() * 0.5 + 0.2})`
    ctx.beginPath()
    ctx.arc(randomInt(0, width), randomInt(0, height), randomInt(1, 2), 0, Math.PI * 2)
    ctx.fill()
  }

  const chars = captchaCode.value.split('')
  const step = width / (chars.length + 1)
  chars.forEach((char, index) => {
    const fontSize = randomInt(24, 30)
    const angle = (randomInt(-24, 24) * Math.PI) / 180
    const x = step * (index + 1)
    const y = randomInt(28, 36)

    ctx.save()
    ctx.translate(x, y)
    ctx.rotate(angle)
    ctx.fillStyle = `rgb(${randomInt(75, 130)}, ${randomInt(70, 120)}, ${randomInt(95, 150)})`
    ctx.font = `700 ${fontSize}px "Trebuchet MS", sans-serif`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(char, 0, 0)
    ctx.restore()
  })
}

function refreshCaptcha(): void {
  captchaCode.value = makeCaptchaCode()
  void nextTick(() => {
    drawCaptcha()
  })
}

async function handleLogin(): Promise<void> {
  errorMessage.value = ''

  if (!account.value.trim() || !password.value) {
    errorMessage.value = '请输入用户名和密码'
    return
  }

  if (!captchaInput.value.trim()) {
    errorMessage.value = '请输入验证码'
    return
  }

  if (captchaInput.value.trim().toUpperCase() !== captchaCode.value.toUpperCase()) {
    errorMessage.value = '验证码错误，请重试'
    captchaInput.value = ''
    refreshCaptcha()
    return
  }

  loading.value = true
  try {
    const normalizedAccount = await adminLogin(account.value.trim(), password.value)
    setAuthSession(normalizedAccount)
    await router.replace(getRedirectPath())
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '登录失败，请稍后重试'
    captchaInput.value = ''
    refreshCaptcha()
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  void fetchAdminSession()
    .then((session) => {
      setAuthSession(session.account)
      return router.replace(getRedirectPath())
    })
    .catch(() => undefined)

  refreshCaptcha()
  void fetchThemeBackgroundUrl()
    .then((url) => {
      if (!url) {
        return
      }
      loginPageStyle.value = {
        backgroundImage: `linear-gradient(rgba(7, 10, 18, 0.66), rgba(7, 10, 18, 0.66)), url("${url}")`,
        backgroundPosition: 'center',
        backgroundRepeat: 'no-repeat',
        backgroundSize: 'cover',
      }
    })
    .catch((error) => {
      console.warn('Failed to load login background from /setting', error)
    })
})
</script>

<style scoped>
.login-page {
  position: relative;
  min-height: 100vh;
  padding: 16px;
  display: grid;
  place-items: center;
  background: #0a0b10;
}

.login-card {
  position: relative;
  z-index: 1;
  width: min(100%, 460px);
  padding: 24px 26px 24px;
  color: #f6f7fb;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(145deg, rgba(25, 25, 29, 0.96), rgba(18, 18, 22, 0.96));
  box-shadow: 0 18px 46px rgba(0, 0, 0, 0.52);
}

.welcome-wrap {
  display: flex;
  justify-content: center;
  margin-bottom: 10px;
}

.welcome-image {
  width: 172px;
  height: auto;
  user-select: none;
  -webkit-user-drag: none;
}

.error-alert {
  margin-bottom: 14px;
  border: 1px solid rgba(255, 117, 153, 0.35);
}

.field-group {
  margin-top: 12px;
}

.field-label {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  font-size: 22px;
  font-weight: 600;
  color: #ffffff;
}

.label-icon {
  color: #ff6fa8;
}

:deep(.login-input .v-field) {
  border-radius: 12px;
  background: rgba(41, 42, 49, 0.86) !important;
  box-shadow: none !important;
}

:deep(.login-input .v-field__input) {
  min-height: 50px;
  color: #f5f5f8;
}

:deep(.login-input input::placeholder) {
  color: #9ea1ac;
  opacity: 1;
}

.captcha-row {
  display: flex;
  gap: 12px;
  align-items: stretch;
}

.captcha-input {
  flex: 1;
}

.captcha-canvas {
  width: 152px;
  min-width: 152px;
  height: 50px;
  border-radius: 12px;
  cursor: pointer;
  border: 1px solid rgba(255, 255, 255, 0.22);
}

.captcha-hint {
  margin-top: 8px;
  padding: 0;
  border: 0;
  background: transparent;
  color: #9ea4b5;
  font-size: 12px;
  cursor: pointer;
}

.login-btn {
  height: 50px !important;
  color: #ffffff;
  font-size: 22px;
  font-weight: 700;
  letter-spacing: 1px;
  border-radius: 12px !important;
  background: linear-gradient(90deg, #f06da7, #f58ab8) !important;
}

.login-btn:hover {
  filter: brightness(1.04);
}

@media (max-width: 680px) {
  .login-card {
    padding: 22px 16px 20px;
  }

  .welcome-image {
    width: 152px;
  }

  .captcha-row {
    flex-direction: column;
  }

  .captcha-canvas {
    width: 100%;
    min-width: 0;
  }
}
</style>
