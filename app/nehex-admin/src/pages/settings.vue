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
          <v-btn
            prepend-icon="mdi-restore"
            variant="text"
            @click="resetCurrentSection"
          >
            重置当前分组
          </v-btn>
          <v-btn
            color="primary"
            prepend-icon="mdi-content-save-outline"
            :loading="saving"
            @click="saveCurrentSection"
          >
            保存当前分组
          </v-btn>
        </div>
      </header>

      <v-alert
        v-if="errorMessage"
        class="mb-4"
        density="comfortable"
        type="error"
        variant="tonal"
      >
        {{ errorMessage }}
      </v-alert>

      <v-alert
        v-if="successMessage"
        class="mb-4"
        density="comfortable"
        type="success"
        variant="tonal"
      >
        {{ successMessage }}
      </v-alert>

      <v-progress-linear
        v-if="loading"
        class="mb-4"
        color="primary"
        indeterminate
      />

      <v-window v-model="activeSectionKey" :touch="false" class="section-window">
        <v-window-item value="nehex">
          <v-card class="section-card" rounded="xl">
            <v-card-title>NeHex配置</v-card-title>
            <v-card-text>
              <div class="form-grid">
                <v-text-field
                  v-model="nehexForm.siteTitle"
                  label="站点标题（site_title）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="nehexForm.siteSubtitle"
                  label="副标题（site_sub_title）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="nehexForm.apiBase"
                  label="API 基础路径（site_api_base）"
                  variant="outlined"
                />
              </div>

              <div class="class-toolbar">
                <div class="class-toolbar-title">文章分类配置（nehex_article_class）</div>
                <div class="class-toolbar-actions">
                  <v-text-field
                    v-model="newClassValue"
                    class="class-add-input"
                    density="comfortable"
                    hide-details
                    label="分类值"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="newClassLabel"
                    class="class-add-input"
                    density="comfortable"
                    hide-details
                    label="分类名称（可选）"
                    variant="outlined"
                  />
                  <v-btn color="primary" prepend-icon="mdi-plus" @click="addArticleClass">
                    添加分类
                  </v-btn>
                </div>
              </div>

              <div v-if="nehexClasses.length > 0" class="class-list">
                <div
                  v-for="(item, index) in nehexClasses"
                  :key="`class-${index}`"
                  class="class-row"
                >
                  <v-text-field
                    v-model="item.value"
                    density="comfortable"
                    hide-details
                    label="分类值"
                    variant="outlined"
                  />
                  <v-text-field
                    v-model="item.label"
                    density="comfortable"
                    hide-details
                    label="分类名称"
                    variant="outlined"
                  />
                  <v-btn
                    color="error"
                    icon="mdi-delete-outline"
                    variant="text"
                    @click="removeArticleClass(index)"
                  />
                </div>
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
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="site">
          <v-card class="section-card" rounded="xl">
            <v-card-title>站点配置</v-card-title>
            <v-card-text>
              <div class="form-grid">
                <v-text-field
                  v-model="siteForm.siteUrl"
                  label="站点地址（site_url）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="siteForm.siteIcp"
                  label="备案信息（site_icp）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="siteForm.siteKeywords"
                  label="关键词（site_keywords）"
                  variant="outlined"
                />
              </div>

              <v-textarea
                v-model="siteForm.siteDescription"
                auto-grow
                label="站点描述（site_description）"
                min-rows="4"
                variant="outlined"
              />

              <v-textarea
                v-model="siteForm.siteNotice"
                auto-grow
                label="公告（site_notice）"
                min-rows="4"
                variant="outlined"
              />
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="theme">
          <v-card class="section-card" rounded="xl">
            <v-card-title>主题设置</v-card-title>
            <v-card-text>
              <div class="theme-file-row">
                <v-select
                  v-model="selectedThemeFile"
                  class="theme-file-select"
                  :items="themeFileOptions"
                  item-title="label"
                  item-value="value"
                  label="主题配置文件"
                  variant="outlined"
                />

                <v-text-field
                  v-model="newThemeFile"
                  class="theme-file-input"
                  label="新增主题文件名"
                  placeholder="my-theme.json"
                  variant="outlined"
                />

                <v-btn color="primary" prepend-icon="mdi-plus" @click="addThemeProfile">
                  新增
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

              <div class="form-grid">
                <v-text-field
                  v-model="themeForm.background"
                  label="背景图（theme_background）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="themeForm.primary"
                  label="主色（theme_primary）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="themeForm.banner"
                  label="横幅图（theme_banner）"
                  variant="outlined"
                />
                <v-text-field
                  v-model="themeForm.cardStyle"
                  label="卡片样式（theme_card_style）"
                  variant="outlined"
                />
              </div>

              <div class="theme-preview" :style="themePreviewStyle">
                <div class="theme-preview-mask">
                  <div class="theme-preview-title">主题预览</div>
                  <div class="theme-preview-meta">
                    配置文件: {{ selectedThemeFile || 'default.json' }}
                  </div>
                </div>
              </div>
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="account">
          <v-card class="section-card" rounded="xl">
            <v-card-title>帐号设置</v-card-title>
            <v-card-text>
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
            </v-card-text>
          </v-card>
        </v-window-item>
      </v-window>
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
  nehexForm,
  nehexClasses,
  newClassValue,
  newClassLabel,
  siteForm,
  themeForm,
  themeProfiles,
  selectedThemeFile,
  newThemeFile,
  accountForm,
  themeFileOptions,
  themePreviewStyle,
  addThemeProfile,
  removeCurrentThemeProfile,
  addArticleClass,
  removeArticleClass,
  resetCurrentSection,
  saveCurrentSection,
} = useSettingsPage()
</script>

<style scoped src="./settings/style.scss"></style>
