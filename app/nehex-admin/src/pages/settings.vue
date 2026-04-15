<template>
  <AdminLayout v-if="isRootSettingsRoute">
    <template #secondary-nav>
      <div class="settings-subnav">
        <div class="subnav-title">基础设置</div>
        <v-list class="subnav-list" density="comfortable" nav>
          <v-list-item
            v-for="item in sections"
            :key="item.key"
            class="subnav-item"
            :active="activeSectionKey === item.key"
            :prepend-icon="item.icon"
            rounded="lg"
            :title="item.label"
            @click="activeSectionKey = item.key"
          />
        </v-list>
      </div>
    </template>

    <section class="settings-page">
      <header class="page-header">
        <div class="header-text">
          <h1>{{ activeSection.label }}</h1>
          <p>{{ activeSection.description }}</p>
        </div>
        <div class="header-actions">
          <v-btn prepend-icon="mdi-restore" variant="text" @click="resetCurrentSection">
            重置当前分组
          </v-btn>
          <v-btn
            color="primary"
            prepend-icon="mdi-content-save-outline"
            :disabled="!canSaveCurrentSection"
            :loading="saving"
            @click="saveCurrentSection"
          >
            保存当前分组
          </v-btn>
        </div>
      </header>

      <v-progress-linear v-if="loading" class="mb-4" color="primary" indeterminate />

      <v-window v-model="activeSectionKey" :touch="false" class="section-window">
        <v-window-item value="nehex">
          <v-card class="section-card" rounded="xl">
            <v-card-title>NeHex配置</v-card-title>
            <v-card-text>
              <div class="stack-block">
                <div class="block-title">后台地址</div>
                <v-text-field
                  v-model="nehexForm.adminManagerWeb"
                  :error-messages="adminManagerWebValidationMessage ? [adminManagerWebValidationMessage] : []"
                  label="后台地址（admin_manager_web）"
                  placeholder="/nehex-admin"
                  variant="outlined"
                />
                <div v-if="adminManagerWebHint" class="path-hint">{{ adminManagerWebHint }}</div>
              </div>

              <v-divider class="my-4" />

              <div class="stack-block">
                <div class="block-title">后台登录页背景</div>
                <v-text-field
                  v-model="nehexForm.adminLoginBackground"
                  label="后台登录页背景（admin_login_background）"
                  placeholder="/images/background-2k.png"
                  variant="outlined"
                />
              </div>

              <v-divider class="my-4" />

              <div class="stack-block">
                <div class="block-title">Kuma-API 地址</div>
                <div class="field-action-row">
                  <v-text-field
                    v-model="nehexForm.kumaApiUrl"
                    hide-details
                    label="Kuma-API 地址（kuma_api_url）"
                    placeholder="https://kuma-api.example.com"
                    variant="outlined"
                  />
                  <v-btn
                    class="field-action-btn"
                    color="primary"
                    :loading="kumaApiTesting"
                    variant="text"
                    @click="testKumaApiUrl"
                  >
                    测试链接
                  </v-btn>
                </div>

                <v-alert
                  v-if="kumaApiTestResult"
                  density="comfortable"
                  type="success"
                  variant="tonal"
                >
                  {{ kumaApiTestResult }}
                </v-alert>
                <v-alert
                  v-if="kumaApiTestError"
                  density="comfortable"
                  type="error"
                  variant="tonal"
                >
                  {{ kumaApiTestError }}
                </v-alert>
              </div>

              <v-divider class="my-4" />

              <div class="stack-block">
                <div class="block-title">分类设置</div>
                <div class="class-editor-row">
                  <v-text-field
                    v-model="newClassLabel"
                    density="comfortable"
                    hide-details
                    label="分类名称"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="newClassValue"
                    density="comfortable"
                    hide-details
                    label="英文值（唯一）"
                    variant="outlined"
                  />
                  <v-btn color="primary" prepend-icon="mdi-plus" @click="addArticleClass">
                    添加
                  </v-btn>
                </div>

                <div v-if="nehexClasses.length > 0" class="class-card-grid">
                  <v-card
                    v-for="(item, index) in nehexClasses"
                    :key="`class-${index}`"
                    class="class-item-card"
                    rounded="lg"
                    variant="outlined"
                  >
                    <v-card-text class="class-item-card-content">
                      <v-text-field
                        v-model="item.label"
                        density="comfortable"
                        hide-details
                        label="分类名称"
                        variant="outlined"
                      />
                      <v-text-field
                        v-model="item.value"
                        density="comfortable"
                        hide-details
                        label="英文值"
                        variant="outlined"
                      />
                      <div class="class-item-remove">
                        <v-btn
                          color="error"
                          icon="mdi-delete-outline"
                          size="small"
                          variant="text"
                          @click="removeArticleClass(index)"
                        />
                      </div>
                    </v-card-text>
                  </v-card>
                </div>

                <v-alert
                  v-else
                  class="mt-2"
                  density="comfortable"
                  type="info"
                  variant="tonal"
                >
                  暂无分类，请添加至少一个分类项。
                </v-alert>
              </div>

              <v-divider class="my-4" />

              <div class="stack-block">
                <div class="update-header">
                  <div>
                    <div class="block-title">检查更新</div>
                    <div class="block-subtitle">通过 GitHub Releases API 检测最新版本</div>
                  </div>
                  <v-btn
                    color="primary"
                    prepend-icon="mdi-update"
                    :loading="updateChecking"
                    @click="checkLatestRelease"
                  >
                    检查更新
                  </v-btn>
                </div>

                <v-alert
                  v-if="releaseStatusText"
                  class="mt-2"
                  :type="hasNewRelease ? 'warning' : 'success'"
                  variant="tonal"
                >
                  {{ releaseStatusText }}
                </v-alert>

                <v-alert
                  v-if="updateCheckError"
                  class="mt-2"
                  density="comfortable"
                  type="error"
                  variant="tonal"
                >
                  {{ updateCheckError }}
                </v-alert>

                <div v-if="latestRelease" class="release-meta">
                  <div>Release: {{ latestRelease.name }} ({{ latestRelease.tagName }})</div>
                  <div>发布时间: {{ latestRelease.publishedAt || '-' }}</div>
                  <div>当前版本: {{ currentVersion || 'unknown' }}</div>
                  <v-btn
                    v-if="latestRelease.htmlUrl"
                    class="mt-2"
                    color="primary"
                    :href="latestRelease.htmlUrl"
                    prepend-icon="mdi-open-in-new"
                    rel="noopener noreferrer"
                    target="_blank"
                    variant="text"
                  >
                    查看发布详情
                  </v-btn>
                </div>
              </div>

              <v-divider class="my-4" />

              <div class="stack-block">
                <div class="block-title">后台帐号密码设置</div>

                <div class="form-grid">
                  <v-text-field
                    v-model="accountForm.account"
                    label="管理员账号（user_account）"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="accountForm.newPassword"
                    autocomplete="new-password"
                    label="新密码（可选）"
                    type="password"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="accountForm.confirmPassword"
                    autocomplete="new-password"
                    label="确认新密码"
                    type="password"
                    variant="outlined"
                  />
                </div>
              </div>
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="site">
          <v-card class="section-card" rounded="xl">
            <v-card-title>网站配置</v-card-title>
            <v-card-text>
              <div class="form-grid">
                <v-text-field v-model="siteForm.siteTitle" label="站点标题（site_title）" variant="outlined" />
                <v-text-field
                  v-model="siteForm.siteSubtitle"
                  label="站点副标题（site_sub_title）"
                  variant="outlined"
                />
                <v-text-field v-model="siteForm.siteUrl" label="站点地址（site_url）" variant="outlined" />
                <v-text-field
                  v-model="siteForm.siteKeywords"
                  label="站点关键词（site_keywords）"
                  variant="outlined"
                />
                <v-text-field v-model="siteForm.siteIcp" label="ICP备案（site_icp）" variant="outlined" />
                <v-text-field
                  v-model="siteForm.siteFavicon"
                  label="站点 favicon（site_favicon）"
                  placeholder="/favicon.ico"
                  variant="outlined"
                />
              </div>

              <v-textarea
                v-model="siteForm.siteDescription"
                auto-grow
                class="mt-2"
                label="站点描述（site_description）"
                min-rows="4"
                variant="outlined"
              />
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="storage">
          <v-card class="section-card" rounded="xl">
            <v-card-title>存储设置</v-card-title>
            <v-card-text>
              <div class="stack-block">
                <div class="block-title">存储平台</div>
                <v-select
                  v-model="storageForm.provider"
                  :items="storageProviderOptions"
                  item-title="label"
                  item-value="value"
                  label="存储平台"
                  variant="outlined"
                />
                <div class="block-subtitle">保存后默认启用存储设置。</div>
              </div>

              <v-divider class="my-4" />

              <div class="stack-block">
                <div class="block-title">公共访问地址（可选）</div>
                <v-text-field
                  v-model="storageForm.publicBaseUrl"
                  label="公共访问 Base URL（如 https://cdn.example.com）"
                  variant="outlined"
                />
              </div>

              <v-divider class="my-4" />

              <div class="stack-block">
                <div class="block-title">上传路径规则（全部存储类型生效）</div>
                <v-text-field
                  v-model="storageForm.localPathRule"
                  label="路径规则"
                  placeholder="/{year}-{month}/{day}/{random_name}.{file_type}"
                  variant="outlined"
                />
                <v-alert class="mt-2" density="comfortable" type="info" variant="tonal">
                  路径规则支持占位符：{year} {month} {day} {hour} {minute} {second} {timestamp} {random_name} {file_type}
                </v-alert>
              </div>

              <v-divider class="my-4" />

              <div v-if="showLocalStorageFields" class="stack-block">
                <div class="block-title">本机存储配置</div>
                <div class="form-grid">
                  <v-text-field
                    v-model="storageForm.localRoot"
                    label="本机存储目录（相对项目根目录）"
                    placeholder="storage"
                    variant="outlined"
                  />
                </div>
              </div>

              <div v-if="showR2StorageFields" class="stack-block">
                <div class="block-title">Cloudflare R2 配置</div>
                <div class="form-grid">
                  <v-text-field
                    v-model="storageForm.r2Endpoint"
                    label="Endpoint"
                    placeholder="https://<accountid>.r2.cloudflarestorage.com"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.r2Bucket"
                    label="Bucket"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.r2AccessKeyId"
                    label="Access Key ID"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.r2SecretAccessKey"
                    label="Secret Access Key"
                    type="password"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.r2Region"
                    label="Region（默认 auto）"
                    variant="outlined"
                  />
                </div>
              </div>

              <div v-if="showS3StorageFields" class="stack-block">
                <div class="block-title">S3 对象存储配置（COS/OSS/B2）</div>
                <div class="form-grid">
                  <v-text-field
                    v-model="storageForm.s3Endpoint"
                    label="Endpoint"
                    placeholder="https://s3.ap-guangzhou.myqcloud.com"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.s3Bucket"
                    label="Bucket"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.s3AccessKeyId"
                    label="Access Key ID"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.s3SecretAccessKey"
                    label="Access Key Secret"
                    type="password"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.s3Region"
                    label="Region（可选）"
                    placeholder="ap-guangzhou"
                    variant="outlined"
                  />
                </div>
              </div>

              <div v-if="showHi168S3StorageFields" class="stack-block">
                <div class="block-title">HI168 S3 配置（强制路径样式）</div>
                <div class="form-grid">
                  <v-text-field
                    v-model="storageForm.hi168S3Endpoint"
                    label="Endpoint"
                    placeholder="https://s3.hi168.com"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.hi168S3Bucket"
                    label="Bucket"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.hi168S3AccessKeyId"
                    label="Access Key ID"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.hi168S3SecretAccessKey"
                    label="Access Key Secret"
                    type="password"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.hi168S3Region"
                    label="Region（建议填写）"
                    placeholder="us-east-1"
                    variant="outlined"
                  />
                </div>
              </div>

              <div v-if="showAliyunOssStorageFields" class="stack-block">
                <div class="block-title">阿里云 OSS 配置</div>
                <div class="form-grid">
                  <v-text-field
                    v-model="storageForm.aliyunOssEndpoint"
                    label="Endpoint"
                    placeholder="https://oss-cn-hangzhou.aliyuncs.com"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.aliyunOssBucket"
                    label="Bucket"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.aliyunOssAccessKeyId"
                    label="Access Key ID"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.aliyunOssSecretAccessKey"
                    label="Access Key Secret"
                    type="password"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="storageForm.aliyunOssRegion"
                    label="Region（可选）"
                    placeholder="cn-hangzhou"
                    variant="outlined"
                  />
                </div>
              </div>
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="owner">
          <v-card class="section-card" rounded="xl">
            <v-card-title>站长资料</v-card-title>
            <v-card-text>
              <div class="form-grid">
                <v-text-field
                  v-model="ownerForm.avatar"
                  label="头像（site_owner_avatar）"
                  placeholder="/images/head.jpg"
                  variant="outlined"
                />
                <v-text-field
                  v-model="ownerForm.nickname"
                  label="昵称（site_owner_nickname）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="ownerForm.homepage"
                  label="主页（site_owner_homepage）"
                  placeholder="https://example.com"
                  variant="outlined"
                />
                <v-text-field
                  v-model="ownerForm.email"
                  label="邮箱（site_owner_email）"
                  placeholder="owner@example.com"
                  variant="outlined"
                />
              </div>

              <v-textarea
                v-model="ownerForm.bio"
                auto-grow
                class="mt-2"
                label="简介（site_owner_bio）"
                min-rows="4"
                variant="outlined"
              />
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="theme">
          <v-card class="section-card" rounded="xl">
            <v-card-title>主题配置</v-card-title>
            <v-card-text>
              <div class="theme-toolbar">
                <v-select
                  :model-value="selectedThemeFile"
                  class="theme-template-select"
                  :items="themeFileOptions"
                  item-title="label"
                  item-value="value"
                  label="主题模板"
                  variant="outlined"
                  @update:model-value="handleThemeTemplateSelect"
                />

                <v-btn color="primary" prepend-icon="mdi-plus" @click="addThemeProfile">
                  新建模板
                </v-btn>
                <v-btn
                  color="error"
                  prepend-icon="mdi-delete-outline"
                  :disabled="themeProfiles.length <= 1"
                  @click="removeCurrentThemeProfile"
                >
                  删除
                </v-btn>
              </div>

              <v-textarea
                v-model="themeEditorJson"
                class="theme-json-editor"
                label="主题 JSON"
                rows="18"
                no-resize
                spellcheck="false"
                variant="outlined"
              />

              <div class="theme-editor-actions">
                <v-btn prepend-icon="mdi-code-json" variant="text" @click="formatThemeEditorJson">
                  格式化 JSON
                </v-btn>
                <span class="theme-editor-hint">保存当前分组时会校验 JSON 并写入 theme_profiles</span>
              </div>

              <v-alert
                v-if="themeEditorError"
                class="mt-2"
                density="comfortable"
                type="error"
                variant="tonal"
              >
                {{ themeEditorError }}
              </v-alert>
            </v-card-text>
          </v-card>
        </v-window-item>
      </v-window>

      <v-dialog v-model="themeCreateDialog" max-width="460">
        <v-card rounded="xl">
          <v-card-title>新建主题模板</v-card-title>
          <v-card-text>
            <v-text-field
              v-model="themeCreateName"
              autofocus
              label="模板名称"
              placeholder="my-theme.json"
              variant="outlined"
              @keyup.enter="confirmCreateThemeProfile"
            />
            <v-alert
              v-if="themeCreateError"
              class="mt-2"
              density="comfortable"
              type="error"
              variant="tonal"
            >
              {{ themeCreateError }}
            </v-alert>
          </v-card-text>
          <v-card-actions class="theme-create-actions">
            <v-spacer />
            <v-btn variant="text" @click="cancelCreateThemeProfile">取消</v-btn>
            <v-btn color="primary" prepend-icon="mdi-content-save-outline" @click="confirmCreateThemeProfile">
              保存
            </v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>
    </section>
  </AdminLayout>
  <RouterView v-else />
</template>

<script lang="ts" setup>
import { computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import AdminLayout from '@/components/admin/AdminLayout.vue'
import { useGlobalSnackbar } from '@/composables/useGlobalSnackbar'
import { useSettingsPage } from '@/pages/settings/useSettingsPage'

const route = useRoute()
const isRootSettingsRoute = computed(() => route.path === '/settings')
const { showGlobalSuccess, showGlobalError } = useGlobalSnackbar()

const {
  sections,
  activeSection,
  activeSectionKey,
  loading,
  saving,
  errorMessage,
  successMessage,
  canSaveCurrentSection,

  nehexForm,
  nehexClasses,
  newClassValue,
  newClassLabel,
  accountForm,

  siteForm,
  ownerForm,
  storageForm,
  storageProviderOptions,
  showLocalStorageFields,
  showR2StorageFields,
  showS3StorageFields,
  showHi168S3StorageFields,
  showAliyunOssStorageFields,

  themeProfiles,
  selectedThemeFile,
  themeCreateDialog,
  themeCreateName,
  themeCreateError,
  themeFileOptions,
  themeEditorJson,
  themeEditorError,

  updateChecking,
  updateCheckError,
  latestRelease,
  currentVersion,
  hasNewRelease,
  releaseStatusText,
  kumaApiTesting,
  kumaApiTestResult,
  kumaApiTestError,
  adminManagerWebValidationMessage,
  adminManagerWebHint,

  addThemeProfile,
  cancelCreateThemeProfile,
  confirmCreateThemeProfile,
  handleThemeTemplateSelect,
  removeCurrentThemeProfile,
  formatThemeEditorJson,
  addArticleClass,
  removeArticleClass,
  checkLatestRelease,
  testKumaApiUrl,
  resetCurrentSection,
  saveCurrentSection,
} = useSettingsPage()

watch(successMessage, (nextMessage) => {
  const text = nextMessage.trim()
  if (!text) {
    return
  }
  showGlobalSuccess(text)
})

watch(errorMessage, (nextMessage) => {
  const text = nextMessage.trim()
  if (!text) {
    return
  }
  showGlobalError(text)
})
</script>

<style scoped src="./settings/style.scss"></style>
