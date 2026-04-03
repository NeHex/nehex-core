<template>
  <div class="login-page">
    <div class="bg-glow bg-glow-top" />
    <div class="bg-glow bg-glow-bottom" />

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
              height="56"
              width="168"
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
import { fetchAdminCredentials } from '@/services/settings'
import { isAuthenticated, setAuthSession } from '@/utils/auth'

const route = useRoute()
const router = useRouter()

const account = ref('')
const password = ref('')
const captchaInput = ref('')
const captchaCode = ref('')
const captchaCanvas = ref<HTMLCanvasElement | null>(null)
const loading = ref(false)
const errorMessage = ref('')

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
    const fontSize = randomInt(30, 36)
    const angle = (randomInt(-24, 24) * Math.PI) / 180
    const x = step * (index + 1)
    const y = randomInt(35, 44)

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

function bufferToHex(buffer: ArrayBuffer): string {
  return Array.from(new Uint8Array(buffer))
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('')
}

async function sha256Hex(value: string): Promise<string> {
  if (!window.crypto?.subtle) {
    throw new Error('当前环境不支持 Web Crypto，请使用 HTTPS 或 localhost。')
  }

  const data = new TextEncoder().encode(value)
  const hash = await window.crypto.subtle.digest('SHA-256', data)
  return bufferToHex(hash)
}

async function doubleSha256(value: string): Promise<string> {
  const first = await sha256Hex(value)
  return sha256Hex(first)
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
    const { account: expectedAccount, passwordHash } = await fetchAdminCredentials()
    if (!expectedAccount || !passwordHash) {
      throw new Error('setting 中缺少 user_account 或 user_account_password')
    }

    const accountMatches = account.value.trim() === expectedAccount
    const passwordMatches = (await doubleSha256(password.value)).toLowerCase() === passwordHash.toLowerCase()

    if (!accountMatches || !passwordMatches) {
      errorMessage.value = '用户名或密码错误'
      captchaInput.value = ''
      refreshCaptcha()
      return
    }

    setAuthSession(account.value.trim())
    await router.replace(getRedirectPath())
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '登录失败，请稍后重试'
    captchaInput.value = ''
    refreshCaptcha()
  } finally {
    loading.value = false
  }
}

if (isAuthenticated()) {
  void router.replace(getRedirectPath())
}

onMounted(() => {
  refreshCaptcha()
})
</script>

<style scoped>
.login-page {
  position: relative;
  min-height: 100vh;
  padding: 20px;
  display: grid;
  place-items: center;
  background:
    radial-gradient(1200px 500px at 85% -100px, rgba(232, 104, 179, 0.18), transparent 60%),
    radial-gradient(900px 400px at 0% 120%, rgba(80, 107, 255, 0.16), transparent 60%),
    #0a0b10;
}

.bg-glow {
  position: absolute;
  pointer-events: none;
  filter: blur(40px);
  z-index: 0;
}

.bg-glow-top {
  width: 280px;
  height: 280px;
  right: 10%;
  top: 8%;
  background: rgba(233, 111, 183, 0.22);
}

.bg-glow-bottom {
  width: 260px;
  height: 260px;
  left: 8%;
  bottom: 6%;
  background: rgba(81, 125, 255, 0.2);
}

.login-card {
  position: relative;
  z-index: 1;
  width: min(100%, 540px);
  padding: 34px 36px 32px;
  color: #f6f7fb;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: linear-gradient(145deg, rgba(25, 25, 29, 0.96), rgba(18, 18, 22, 0.96));
  box-shadow: 0 28px 70px rgba(0, 0, 0, 0.58);
}

.welcome-wrap {
  display: flex;
  justify-content: center;
  margin-bottom: 12px;
}

.welcome-image {
  width: 220px;
  height: auto;
  user-select: none;
  -webkit-user-drag: none;
}

.error-alert {
  margin-bottom: 14px;
  border: 1px solid rgba(255, 117, 153, 0.35);
}

.field-group {
  margin-top: 14px;
}

.field-label {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
  font-size: 30px;
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
  min-height: 56px;
  color: #f5f5f8;
}

:deep(.login-input input::placeholder) {
  color: #9ea1ac;
  opacity: 1;
}

.captcha-row {
  display: flex;
  gap: 14px;
  align-items: stretch;
}

.captcha-input {
  flex: 1;
}

.captcha-canvas {
  width: 168px;
  min-width: 168px;
  height: 56px;
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
  height: 56px !important;
  color: #ffffff;
  font-size: 28px;
  font-weight: 700;
  letter-spacing: 2px;
  border-radius: 12px !important;
  background: linear-gradient(90deg, #f06da7, #f58ab8) !important;
}

.login-btn:hover {
  filter: brightness(1.04);
}

@media (max-width: 680px) {
  .login-card {
    padding: 26px 20px 24px;
  }

  .welcome-image {
    width: 188px;
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
