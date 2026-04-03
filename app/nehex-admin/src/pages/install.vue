<template>
  <div class="install-page">
    <v-card class="install-card" rounded="xl" elevation="18">
      <header class="install-header">
        <h1>NeHex 首次安装</h1>
        <p>请按步骤完成基础初始化，完成后即可进入后台登录。</p>
      </header>

      <v-alert
        v-if="errorMessage"
        class="mb-4"
        type="error"
        variant="tonal"
      >
        {{ errorMessage }}
      </v-alert>

      <v-alert
        v-if="successMessage"
        class="mb-4"
        type="success"
        variant="tonal"
      >
        {{ successMessage }}
      </v-alert>

      <div class="step-indicator">
        <button
          v-for="item in steps"
          :key="item.value"
          class="step-item"
          :class="{ active: currentStep === item.value }"
          type="button"
          @click="jumpToStep(item.value)"
        >
          <span class="step-index">{{ item.value }}</span>
          <span class="step-title">{{ item.label }}</span>
        </button>
      </div>

      <v-window v-model="currentStep" :touch="false">
        <v-window-item :value="1">
          <section class="step-content">
            <h2>第一步：管理员与后台路径</h2>
            <div class="form-grid">
              <v-text-field
                v-model="adminForm.account"
                label="管理员账号"
                variant="outlined"
              />
              <v-text-field
                v-model="adminForm.adminManagerWeb"
                label="后台路径"
                variant="outlined"
              />
              <v-text-field
                v-model="adminForm.password"
                label="管理员密码"
                type="password"
                autocomplete="new-password"
                variant="outlined"
              />
              <v-text-field
                v-model="adminForm.confirmPassword"
                label="确认管理员密码"
                type="password"
                autocomplete="new-password"
                variant="outlined"
              />
            </div>
          </section>
        </v-window-item>

        <v-window-item :value="2">
          <section class="step-content">
            <h2>第二步：NeHex 配置</h2>
            <div class="form-grid">
              <v-text-field
                v-model="nehexForm.siteTitle"
                label="站点标题（site_title）"
                variant="outlined"
              />
              <v-text-field
                v-model="nehexForm.siteSubTitle"
                label="副标题（site_sub_title）"
                variant="outlined"
              />
              <v-text-field
                v-model="nehexForm.siteApiBase"
                label="API 基础路径（site_api_base）"
                variant="outlined"
              />
            </div>
            <v-textarea
              v-model="nehexForm.articleClassesInput"
              auto-grow
              label="文章分类（每行一个，格式 value:label）"
              min-rows="6"
              variant="outlined"
            />
          </section>
        </v-window-item>

        <v-window-item :value="3">
          <section class="step-content">
            <h2>第三步：站点配置</h2>
            <div class="form-grid">
              <v-text-field
                v-model="siteForm.siteUrl"
                label="站点地址（site_url）"
                variant="outlined"
              />
              <v-text-field
                v-model="siteForm.siteKeywords"
                label="关键词（site_keywords）"
                variant="outlined"
              />
              <v-text-field
                v-model="siteForm.siteIcp"
                label="备案信息（site_icp）"
                variant="outlined"
              />
            </div>
            <v-textarea
              v-model="siteForm.siteDescription"
              auto-grow
              label="站点描述（site_description）"
              min-rows="3"
              variant="outlined"
            />
            <v-textarea
              v-model="siteForm.siteNotice"
              auto-grow
              label="站点公告（site_notice）"
              min-rows="4"
              variant="outlined"
            />
          </section>
        </v-window-item>
      </v-window>

      <footer class="step-actions">
        <v-btn
          variant="text"
          :disabled="currentStep <= 1 || loading"
          @click="prevStep"
        >
          上一步
        </v-btn>
        <v-spacer />
        <v-btn
          v-if="currentStep < 3"
          color="primary"
          :disabled="loading"
          @click="nextStep"
        >
          下一步
        </v-btn>
        <v-btn
          v-else
          color="primary"
          :loading="loading"
          @click="submitInstallation"
        >
          完成安装
        </v-btn>
      </footer>
    </v-card>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  fetchInstallStatus,
  resetInstallStatusCache,
  submitInstall,
  type InstallArticleClassItem,
} from '@/services/install'
import { adminLogout } from '@/services/admin-api'
import { resetSettingsCache } from '@/services/settings'
import { clearAuthSession } from '@/utils/auth'
import { getAdminBasePath, normalizeBasePath } from '@/utils/path'

type StepItem = {
  value: 1 | 2 | 3
  label: string
}

const router = useRouter()
const loading = ref(false)
const currentStep = ref<1 | 2 | 3>(1)
const errorMessage = ref('')
const successMessage = ref('')

const steps: StepItem[] = [
  { value: 1, label: '管理员账号' },
  { value: 2, label: 'NeHex配置' },
  { value: 3, label: '站点配置' },
]

const adminForm = reactive({
  account: '',
  password: '',
  confirmPassword: '',
  adminManagerWeb: '/nehex-admin',
})

const nehexForm = reactive({
  siteTitle: 'NeHex',
  siteSubTitle: '',
  siteApiBase: '',
  articleClassesInput: 'default:默认分类',
})

const siteForm = reactive({
  siteUrl: '',
  siteDescription: '',
  siteKeywords: '',
  siteIcp: '',
  siteNotice: '',
})

function normalizeAdminManagerWeb(raw: string): string {
  const path = normalizeBasePath(raw || '/nehex-admin')
  if (path === '/') {
    return '/nehex-admin'
  }
  return path
}

function parseArticleClasses(raw: string): InstallArticleClassItem[] {
  const lines = raw
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)

  if (lines.length === 0) {
    return [{ value: 'default', label: '默认分类' }]
  }

  const result = new Map<string, InstallArticleClassItem>()
  lines.forEach((line) => {
    const [left, ...right] = line.split(':')
    const value = left?.trim() || ''
    if (!value) {
      return
    }
    const label = right.join(':').trim() || value
    result.set(value, { value, label })
  })

  if (result.size === 0) {
    return [{ value: 'default', label: '默认分类' }]
  }
  return Array.from(result.values())
}

function validateStep(step: number): boolean {
  errorMessage.value = ''
  successMessage.value = ''

  if (step === 1) {
    if (!adminForm.account.trim()) {
      errorMessage.value = '请填写管理员账号'
      return false
    }
    if (!adminForm.password.trim()) {
      errorMessage.value = '请填写管理员密码'
      return false
    }
    if (adminForm.password !== adminForm.confirmPassword) {
      errorMessage.value = '两次输入的管理员密码不一致'
      return false
    }
  }

  if (step === 2) {
    const classes = parseArticleClasses(nehexForm.articleClassesInput)
    if (classes.length === 0) {
      errorMessage.value = '请至少提供一个文章分类'
      return false
    }
  }

  return true
}

function jumpToStep(step: 1 | 2 | 3): void {
  if (loading.value) {
    return
  }

  if (step > currentStep.value) {
    for (let index = currentStep.value; index < step; index += 1) {
      if (!validateStep(index)) {
        return
      }
    }
  }

  currentStep.value = step
}

function nextStep(): void {
  if (!validateStep(currentStep.value)) {
    return
  }
  currentStep.value = (Math.min(3, currentStep.value + 1) as 1 | 2 | 3)
}

function prevStep(): void {
  currentStep.value = (Math.max(1, currentStep.value - 1) as 1 | 2 | 3)
}

async function submitInstallation(): Promise<void> {
  if (!validateStep(1) || !validateStep(2) || !validateStep(3)) {
    return
  }

  loading.value = true
  errorMessage.value = ''
  successMessage.value = ''

  try {
    const payload = {
      admin: {
        account: adminForm.account.trim(),
        password: adminForm.password,
        confirm_password: adminForm.confirmPassword,
        admin_manager_web: normalizeAdminManagerWeb(adminForm.adminManagerWeb),
      },
      nehex: {
        site_title: nehexForm.siteTitle.trim(),
        site_sub_title: nehexForm.siteSubTitle.trim(),
        site_api_base: nehexForm.siteApiBase.trim(),
        article_classes: parseArticleClasses(nehexForm.articleClassesInput),
      },
      site: {
        site_url: siteForm.siteUrl.trim(),
        site_description: siteForm.siteDescription.trim(),
        site_keywords: siteForm.siteKeywords.trim(),
        site_icp: siteForm.siteIcp.trim(),
        site_notice: siteForm.siteNotice.trim(),
      },
    }

    const status = await submitInstall(payload)
    clearAuthSession()
    await adminLogout().catch(() => undefined)
    resetInstallStatusCache()
    resetSettingsCache()
    successMessage.value = '初始化完成，正在跳转到登录页'

    const targetAdminPath = normalizeAdminManagerWeb(status.admin_manager_web)
    const currentAdminPath = normalizeBasePath(getAdminBasePath())
    if (targetAdminPath === currentAdminPath) {
      await router.replace('/login')
      return
    }

    window.location.assign(`${targetAdminPath}/login`)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : '安装失败，请稍后重试'
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  try {
    const status = await fetchInstallStatus(true)
    if (status.installed) {
      await router.replace('/login')
      return
    }
    adminForm.adminManagerWeb = normalizeAdminManagerWeb(status.admin_manager_web || '/nehex-admin')
  } catch {
    adminForm.adminManagerWeb = '/nehex-admin'
  }
})
</script>

<style scoped>
.install-page {
  min-height: 100vh;
  padding: 20px 14px;
  background: radial-gradient(circle at top, #24314f 0%, #131b2d 48%, #0c111d 100%);
  display: grid;
  place-items: center;
}

.install-card {
  width: min(980px, 100%);
  padding: 22px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: #e8eefc;
  background: linear-gradient(180deg, rgba(14, 22, 37, 0.96), rgba(9, 15, 27, 0.96));
}

.install-header h1 {
  margin: 0;
  font-size: 30px;
  color: #f7fbff;
}

.install-header p {
  margin: 8px 0 0;
  color: #b8c4de;
}

.step-indicator {
  margin: 18px 0 16px;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.step-item {
  display: flex;
  align-items: center;
  gap: 10px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(26, 36, 58, 0.8);
  color: #cdd8ef;
  border-radius: 10px;
  padding: 10px 12px;
  cursor: pointer;
}

.step-item.active {
  border-color: rgba(117, 178, 255, 0.86);
  background: linear-gradient(120deg, rgba(30, 50, 82, 0.9), rgba(26, 43, 70, 0.92));
  color: #ffffff;
}

.step-index {
  width: 24px;
  height: 24px;
  display: inline-grid;
  place-items: center;
  border-radius: 50%;
  font-size: 13px;
  font-weight: 700;
  background: rgba(121, 166, 230, 0.35);
}

.step-title {
  font-weight: 600;
  font-size: 14px;
}

.step-content h2 {
  margin: 0 0 12px;
  font-size: 21px;
  color: #f5f9ff;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.step-actions {
  margin-top: 18px;
  display: flex;
  align-items: center;
}

@media (max-width: 760px) {
  .install-card {
    padding: 16px;
  }

  .step-indicator {
    grid-template-columns: 1fr;
  }

  .form-grid {
    grid-template-columns: 1fr;
  }
}
</style>
