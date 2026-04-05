<template>
  <AdminLayout>
    <template #secondary-nav>
      <div class="settings-subnav">
        <div class="subnav-title">站点设置</div>
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

      <v-alert v-if="errorMessage" class="mb-4" density="comfortable" type="error" variant="tonal">
        {{ errorMessage }}
      </v-alert>

      <v-alert v-if="successMessage" class="mb-4" density="comfortable" type="success" variant="tonal">
        {{ successMessage }}
      </v-alert>

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
                <div class="block-title">分类设置（小卡片）</div>
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
                <v-alert class="mb-4" density="comfortable" type="warning" variant="tonal">
                  修改密码时无需输入哈希，后端会自动计算并保存。
                </v-alert>

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

        <v-window-item value="theme">
          <v-card class="section-card" rounded="xl">
            <v-card-title>主题配置（JSON直编）</v-card-title>
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
                auto-grow
                class="theme-json-editor"
                label="主题 JSON"
                min-rows="16"
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
</template>

<script lang="ts" setup>
import AdminLayout from '@/components/admin/AdminLayout.vue'
import { useSettingsPage } from '@/pages/settings/useSettingsPage'

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
  resetCurrentSection,
  saveCurrentSection,
} = useSettingsPage()
</script>

<style scoped src="./settings/style.scss"></style>
