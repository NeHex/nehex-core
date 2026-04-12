use std::collections::{HashMap, HashSet};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, Method, header},
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{NaiveDateTime, Utc};
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use url::Url;

use crate::{
    config::normalize_admin_manager_web_path,
    error::{AppError, AppResult},
    startup,
    state::AppState,
};

type HmacSha256 = Hmac<Sha256>;

pub const ADMIN_TOKEN_COOKIE_KEY: &str = "nehex_admin_token";
pub const ADMIN_PUBLIC_MARKER_COOKIE_KEY: &str = "nehex_admin_marker";

const INSTALL_COMPLETED_KEY: &str = "install_completed";
const ADMIN_MANAGER_WEB_KEY: &str = "admin_manager_web";
const ADMIN_PASSWORD_SCHEME: &str = "pbkdf2_sha256";
const ADMIN_PASSWORD_ITERATIONS: u32 = 390_000;
const INSTALL_STATUS_CACHE_KEY: &str = "admin:install:status";
const INSTALL_STATUS_CACHE_TTL_SECONDS: u64 = 5;
const SETTINGS_CACHE_KEY: &str = "settings:list";
const SETTINGS_WITH_THEME_DETAILS_CACHE_KEY: &str = "settings:list:with-theme-details";

#[derive(Serialize)]
pub struct AdminInstallStatusResponse {
    data: AdminInstallStatusData,
}

#[derive(Serialize)]
pub struct AdminInstallStatusData {
    installed: bool,
    schema_ready: bool,
    table_count: i64,
    admin_manager_web: String,
}

#[derive(Deserialize)]
pub struct AdminInstallRequest {
    admin: AdminInstallStepAdmin,
    #[serde(default)]
    nehex: AdminInstallStepNehex,
    #[serde(default)]
    site: AdminInstallStepSite,
}

#[derive(Deserialize)]
pub struct AdminInstallStepAdmin {
    account: String,
    password: String,
    confirm_password: String,
    #[serde(default = "default_admin_manager_web")]
    admin_manager_web: String,
}

#[derive(Deserialize, Default)]
pub struct AdminInstallStepNehex {
    #[serde(default = "default_site_title")]
    site_title: String,
    #[serde(default)]
    site_sub_title: String,
    #[serde(default)]
    site_api_base: String,
    #[serde(default)]
    article_classes: Vec<AdminInstallArticleClassItem>,
}

#[derive(Deserialize)]
pub struct AdminInstallArticleClassItem {
    value: String,
    #[serde(default)]
    label: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct AdminInstallStepSite {
    #[serde(default)]
    site_url: String,
    #[serde(default)]
    site_description: String,
    #[serde(default)]
    site_keywords: String,
    #[serde(default)]
    site_icp: String,
    #[serde(default)]
    site_notice: String,
}

#[derive(Serialize)]
pub struct AdminInstallResponse {
    data: AdminInstallStatusData,
    message: String,
}

#[derive(Deserialize)]
pub struct AdminLoginRequest {
    account: String,
    password: String,
}

#[derive(Serialize)]
pub struct AdminLoginResponse {
    data: AdminLoginData,
}

#[derive(Serialize)]
pub struct AdminLoginData {
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    account: String,
    expires_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct AdminPublicMarkerResponse {
    data: AdminPublicMarkerData,
}

#[derive(Serialize)]
pub struct AdminPublicMarkerData {
    marker: String,
    account: String,
    expires_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct AdminActionResponse {
    success: bool,
    message: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct InstallStatus {
    installed: bool,
    schema_ready: bool,
    table_count: i64,
    admin_manager_web: String,
}

#[derive(Clone)]
pub struct AdminPrincipal {
    pub account: String,
    pub expires_at: i64,
}

#[derive(Serialize, Deserialize)]
struct AdminTokenClaims {
    account: String,
    #[serde(default)]
    client: String,
    exp: i64,
    iat: i64,
    jti: String,
    #[serde(default, rename = "type")]
    token_type: String,
}

fn default_admin_manager_web() -> String {
    "/nehex-admin".to_string()
}

fn default_site_title() -> String {
    "NeHex".to_string()
}

pub async fn admin_install_status(
    State(state): State<AppState>,
) -> AppResult<Json<AdminInstallStatusResponse>> {
    let status = get_install_status(&state)
        .await
        .unwrap_or_else(|_| InstallStatus {
            installed: false,
            schema_ready: false,
            table_count: 0,
            admin_manager_web: state.settings.admin_manager_web.clone(),
        });

    Ok(Json(AdminInstallStatusResponse {
        data: AdminInstallStatusData {
            installed: status.installed,
            schema_ready: status.schema_ready,
            table_count: status.table_count,
            admin_manager_web: status.admin_manager_web,
        },
    }))
}

pub async fn admin_install(
    State(state): State<AppState>,
    Json(payload): Json<AdminInstallRequest>,
) -> AppResult<Json<AdminInstallResponse>> {
    let status = bootstrap_installation(&state, payload).await?;
    Ok(Json(AdminInstallResponse {
        message: "Installation completed".to_string(),
        data: AdminInstallStatusData {
            installed: status.installed,
            schema_ready: status.schema_ready,
            table_count: status.table_count,
            admin_manager_web: status.admin_manager_web,
        },
    }))
}

pub async fn admin_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AdminLoginRequest>,
) -> AppResult<Response> {
    let install_status = get_install_status(&state).await.map_err(|error| {
        AppError::ServiceUnavailable(format!("System database is unavailable: {error}"))
    })?;

    if !install_status.installed {
        return Err(AppError::ServiceUnavailable(
            "System is not installed yet".to_string(),
        ));
    }

    let (expected_account, expected_password_hash) = get_admin_credentials(&state).await?;
    if expected_account.is_empty() || expected_password_hash.is_empty() {
        return Err(AppError::ServiceUnavailable(
            "Admin account is not configured".to_string(),
        ));
    }

    let account = payload.account.trim().to_string();
    let password = payload.password.trim().to_string();
    if account.is_empty() || password.is_empty() {
        return Err(AppError::Unauthorized(
            "Invalid admin credentials".to_string(),
        ));
    }

    let (password_matched, should_upgrade_hash) =
        verify_admin_password(&password, &expected_password_hash);
    if account != expected_account || !password_matched {
        return Err(AppError::Unauthorized(
            "Invalid admin credentials".to_string(),
        ));
    }

    if should_upgrade_hash {
        let upgraded_hash = hash_admin_password(&password, ADMIN_PASSWORD_ITERATIONS);
        upsert_setting(
            &state,
            "user_account_password",
            "string",
            Some(upgraded_hash),
            Some("管理员密码（哈希）"),
        )
        .await?;
    }

    let now = Utc::now().timestamp();
    let expires_at = now + state.settings.admin_api_token_ttl_seconds.max(300);
    let token = create_admin_token(&state, &expected_account, expires_at, "")?;

    let marker_expires_at = now + state.settings.admin_api_token_ttl_seconds.max(300);
    let marker_token = create_admin_token(
        &state,
        &expected_account,
        marker_expires_at,
        "admin_public_marker",
    )?;

    let secure = is_request_secure(&headers);
    let max_age = (expires_at - now).max(60);
    let marker_max_age = (marker_expires_at - now).max(60);

    let admin_cookie_domain = resolve_cookie_domain(&state.settings.admin_cookie_domain, &headers);
    let public_cookie_domain = resolve_cookie_domain(
        if state.settings.admin_public_cookie_domain.trim().is_empty() {
            &state.settings.admin_cookie_domain
        } else {
            &state.settings.admin_public_cookie_domain
        },
        &headers,
    );

    let admin_cookie = build_cookie(
        ADMIN_TOKEN_COOKIE_KEY,
        &token,
        max_age,
        secure,
        true,
        Some("lax"),
        admin_cookie_domain.as_deref(),
    );
    let marker_cookie = build_cookie(
        ADMIN_PUBLIC_MARKER_COOKIE_KEY,
        &marker_token,
        marker_max_age,
        secure,
        false,
        Some("lax"),
        public_cookie_domain.as_deref(),
    );

    let mut response = Json(AdminLoginResponse {
        data: AdminLoginData {
            token: Some(token),
            account: expected_account,
            expires_at: ts_to_naive_datetime(expires_at),
        },
    })
    .into_response();

    append_set_cookie(&mut response, admin_cookie);
    append_set_cookie(&mut response, marker_cookie);
    Ok(response)
}

pub async fn admin_me(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> AppResult<Json<AdminLoginResponse>> {
    let principal = require_admin_principal(&state, &method, &headers)?;
    Ok(Json(AdminLoginResponse {
        data: AdminLoginData {
            token: None,
            account: principal.account,
            expires_at: ts_to_naive_datetime(principal.expires_at),
        },
    }))
}

pub async fn admin_public_marker(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> AppResult<Response> {
    let principal = require_admin_principal(&state, &method, &headers)?;

    let now = Utc::now().timestamp();
    let marker_expires_at = now + state.settings.admin_api_token_ttl_seconds.max(300);
    let marker_token = create_admin_token(
        &state,
        &principal.account,
        marker_expires_at,
        "admin_public_marker",
    )?;

    let marker_max_age = (marker_expires_at - now).max(60);
    let public_cookie_domain = resolve_cookie_domain(
        if state.settings.admin_public_cookie_domain.trim().is_empty() {
            &state.settings.admin_cookie_domain
        } else {
            &state.settings.admin_public_cookie_domain
        },
        &headers,
    );

    let marker_cookie = build_cookie(
        ADMIN_PUBLIC_MARKER_COOKIE_KEY,
        &marker_token,
        marker_max_age,
        is_request_secure(&headers),
        false,
        Some("lax"),
        public_cookie_domain.as_deref(),
    );

    let mut response = Json(AdminPublicMarkerResponse {
        data: AdminPublicMarkerData {
            marker: marker_token,
            account: principal.account,
            expires_at: ts_to_naive_datetime(marker_expires_at),
        },
    })
    .into_response();

    append_set_cookie(&mut response, marker_cookie);
    Ok(response)
}

pub async fn admin_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let secure = is_request_secure(&headers);

    let mut response = Json(AdminActionResponse {
        success: true,
        message: "Logged out".to_string(),
    })
    .into_response();

    for cookie in build_delete_cookies(
        ADMIN_TOKEN_COOKIE_KEY,
        secure,
        &state.settings.admin_cookie_domain,
        &headers,
    ) {
        append_set_cookie(&mut response, cookie);
    }

    let marker_domain_setting = if state.settings.admin_public_cookie_domain.trim().is_empty() {
        &state.settings.admin_cookie_domain
    } else {
        &state.settings.admin_public_cookie_domain
    };

    for cookie in build_delete_cookies(
        ADMIN_PUBLIC_MARKER_COOKIE_KEY,
        secure,
        marker_domain_setting,
        &headers,
    ) {
        append_set_cookie(&mut response, cookie);
    }

    Ok(response)
}

pub fn require_admin_principal(
    state: &AppState,
    method: &Method,
    headers: &HeaderMap,
) -> AppResult<AdminPrincipal> {
    let admin_client = headers
        .get("x-nehex-admin-client")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .to_string();

    if admin_client != state.settings.admin_api_client_id {
        return Err(AppError::Forbidden(
            "Admin client is not allowed".to_string(),
        ));
    }

    let bearer = extract_bearer_token(headers.get(header::AUTHORIZATION));
    let cookie_token = get_cookie_value(headers.get(header::COOKIE), ADMIN_TOKEN_COOKIE_KEY);
    let token = bearer.clone().or(cookie_token).unwrap_or_default();

    if token.is_empty() {
        return Err(AppError::Unauthorized("Missing admin token".to_string()));
    }

    if bearer.is_none() {
        enforce_csrf_same_origin(method, headers)?;
    }

    decode_admin_token(state, &token)
}

pub fn resolve_admin_account_from_public_marker(
    state: &AppState,
    marker: Option<&str>,
) -> Option<String> {
    let marker = marker?.trim();
    if marker.is_empty() {
        return None;
    }

    let claims = decode_token_claims(&state.settings.admin_api_secret, marker).ok()?;
    if claims.token_type != "admin_public_marker" {
        return None;
    }
    if claims.exp <= Utc::now().timestamp() {
        return None;
    }
    if claims.account.trim().is_empty() {
        return None;
    }
    Some(claims.account)
}

pub fn resolve_admin_identity_from_headers(
    state: &AppState,
    headers: &HeaderMap,
    marker_header: Option<&str>,
) -> bool {
    if let Some(account) = resolve_admin_account_from_public_marker(state, marker_header) {
        if !account.trim().is_empty() {
            return true;
        }
    }

    if let Some(marker_cookie) =
        get_cookie_value(headers.get(header::COOKIE), ADMIN_PUBLIC_MARKER_COOKIE_KEY)
    {
        if resolve_admin_account_from_public_marker(state, Some(&marker_cookie)).is_some() {
            return true;
        }
    }

    if let Some(admin_token) = get_cookie_value(headers.get(header::COOKIE), ADMIN_TOKEN_COOKIE_KEY)
    {
        if decode_admin_token(state, &admin_token).is_ok() {
            return true;
        }
    }

    false
}

fn decode_admin_token(state: &AppState, token: &str) -> AppResult<AdminPrincipal> {
    let claims = decode_token_claims(&state.settings.admin_api_secret, token)
        .map_err(|_| AppError::Unauthorized("Invalid admin token".to_string()))?;

    if claims.client != state.settings.admin_api_client_id {
        return Err(AppError::Unauthorized(
            "Invalid admin token client".to_string(),
        ));
    }
    if claims.account.trim().is_empty() || claims.jti.trim().is_empty() {
        return Err(AppError::Unauthorized(
            "Invalid admin token fields".to_string(),
        ));
    }
    if claims.exp <= Utc::now().timestamp() {
        return Err(AppError::Unauthorized("Admin token expired".to_string()));
    }

    Ok(AdminPrincipal {
        account: claims.account,
        expires_at: claims.exp,
    })
}

async fn get_admin_credentials(state: &AppState) -> AppResult<(String, String)> {
    let rows = sqlx::query(
        "SELECT setting_key, setting_content FROM settings WHERE setting_key = ANY($1)",
    )
    .bind(vec!["user_account", "user_account_password"])
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to load admin credentials: {error}")))?;

    let mut account = String::new();
    let mut password_hash = String::new();

    for row in rows {
        let key = row.try_get::<String, _>("setting_key").unwrap_or_default();
        let value = row
            .try_get::<Option<String>, _>("setting_content")
            .ok()
            .flatten()
            .unwrap_or_default()
            .trim()
            .to_string();
        if key == "user_account" {
            account = value;
        } else if key == "user_account_password" {
            password_hash = value.to_lowercase();
        }
    }

    Ok((account, password_hash))
}

async fn get_install_status(state: &AppState) -> AppResult<InstallStatus> {
    if let Some(cached) = state
        .runtime_cache
        .get::<InstallStatus>(INSTALL_STATUS_CACHE_KEY)
        .await
    {
        return Ok(cached);
    }

    let table_rows = sqlx::query(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list database tables: {error}")))?;

    let table_names = table_rows
        .into_iter()
        .filter_map(|row| row.try_get::<String, _>("table_name").ok())
        .collect::<HashSet<_>>();

    let schema_ready = table_names.contains("settings");
    let mut installed = false;
    let mut admin_manager_web = state.settings.admin_manager_web.clone();

    if schema_ready {
        let rows = sqlx::query(
            "SELECT setting_key, setting_content FROM settings WHERE setting_key = ANY($1)",
        )
        .bind(vec![
            INSTALL_COMPLETED_KEY,
            ADMIN_MANAGER_WEB_KEY,
            "user_account",
            "user_account_password",
        ])
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to read install status: {error}")))?;

        let mut settings = HashMap::<String, String>::new();
        for row in rows {
            let key = row.try_get::<String, _>("setting_key").unwrap_or_default();
            let value = row
                .try_get::<Option<String>, _>("setting_content")
                .ok()
                .flatten()
                .unwrap_or_default();
            settings.insert(key, value);
        }

        let install_completed = parse_boolean(settings.get(INSTALL_COMPLETED_KEY));
        let has_account = settings
            .get("user_account")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        let has_password = settings
            .get("user_account_password")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        installed = install_completed || (has_account && has_password);

        admin_manager_web = normalize_admin_manager_web_path(
            settings
                .get(ADMIN_MANAGER_WEB_KEY)
                .map(|value| value.as_str()),
            &state.settings.admin_manager_web,
        );
    }

    let status = InstallStatus {
        installed,
        schema_ready,
        table_count: table_names.len() as i64,
        admin_manager_web,
    };
    state
        .runtime_cache
        .set(
            INSTALL_STATUS_CACHE_KEY,
            status.clone(),
            INSTALL_STATUS_CACHE_TTL_SECONDS,
        )
        .await;
    Ok(status)
}

async fn bootstrap_installation(
    state: &AppState,
    payload: AdminInstallRequest,
) -> AppResult<InstallStatus> {
    startup::ensure_installation_schema_bootstrap(&state.db_pool)
        .await
        .map_err(|_| {
            AppError::Conflict(
                "Database schema initialization failed during install. Please run DB migrations first or grant CREATE TABLE privilege."
                    .to_string(),
            )
        })?;

    let current_status = get_install_status(state).await?;
    if current_status.installed {
        return Err(AppError::Conflict(
            "System is already installed".to_string(),
        ));
    }

    let account = payload.admin.account.trim().to_string();
    let password = payload.admin.password.trim().to_string();
    let confirm_password = payload.admin.confirm_password.trim().to_string();

    if account.is_empty() {
        return Err(AppError::Conflict("Admin account is required".to_string()));
    }
    if password.is_empty() {
        return Err(AppError::Conflict("Admin password is required".to_string()));
    }
    if password != confirm_password {
        return Err(AppError::Conflict(
            "password and confirm_password do not match".to_string(),
        ));
    }

    let table_rows = sqlx::query(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list database tables: {error}")))?;
    let table_names = table_rows
        .into_iter()
        .filter_map(|row| row.try_get::<String, _>("table_name").ok())
        .collect::<HashSet<_>>();

    let required_tables = ["settings", "article", "singlepage"];
    let missing_tables = required_tables
        .iter()
        .filter(|name| !table_names.contains(**name))
        .copied()
        .collect::<Vec<_>>();
    if !missing_tables.is_empty() {
        return Err(AppError::Conflict(format!(
            "Database schema is not initialized. Missing tables: {}. Run DB migrations first.",
            missing_tables.join(", "),
        )));
    }

    let normalized_admin_manager_web = normalize_admin_manager_web_path(
        Some(payload.admin.admin_manager_web.trim()),
        &state.settings.admin_manager_web,
    );

    let site_title = {
        let text = payload.nehex.site_title.trim();
        if text.is_empty() {
            "NeHex".to_string()
        } else {
            text.to_string()
        }
    };

    let mut class_map = HashMap::<String, String>::new();
    for item in payload.nehex.article_classes {
        let value = item.value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        let label = item
            .label
            .unwrap_or_else(|| value.clone())
            .trim()
            .to_string();
        class_map.insert(value.clone(), if label.is_empty() { value } else { label });
    }
    if class_map.is_empty() {
        class_map.insert("default".to_string(), "默认分类".to_string());
    }
    let primary_article_class = class_map
        .keys()
        .next()
        .cloned()
        .unwrap_or_else(|| "default".to_string());

    let nehex_article_class = serde_json::json!({ "class": class_map });
    let now_iso = Utc::now().to_rfc3339();
    let site_notice = if payload.site.site_notice.trim().is_empty() {
        "站点初始化完成，欢迎使用 NeHex。".to_string()
    } else {
        payload.site.site_notice.trim().to_string()
    };

    let theme_profiles = default_theme_profiles();
    let default_theme = theme_profiles
        .get("rei.json")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let password_hash = hash_admin_password(&password, ADMIN_PASSWORD_ITERATIONS);

    let mut tx = state.db_pool.begin().await.map_err(|error| {
        AppError::internal(format!("Failed to start installation transaction: {error}"))
    })?;

    for (key, setting_type, value, description) in [
        (
            "user_account",
            "string",
            Some(account.clone()),
            Some("管理员账号".to_string()),
        ),
        (
            "user_account_password",
            "string",
            Some(password_hash.clone()),
            Some("管理员密码（哈希）".to_string()),
        ),
        (
            ADMIN_MANAGER_WEB_KEY,
            "string",
            Some(normalized_admin_manager_web.clone()),
            Some("后台路径".to_string()),
        ),
        (
            INSTALL_COMPLETED_KEY,
            "boolean",
            Some("true".to_string()),
            Some("首次安装完成标记".to_string()),
        ),
        (
            "installed_at",
            "string",
            Some(now_iso.clone()),
            Some("首次安装完成时间".to_string()),
        ),
        (
            "site_title",
            "string",
            Some(site_title.clone()),
            Some("站点标题".to_string()),
        ),
        (
            "site_sub_title",
            "string",
            Some(payload.nehex.site_sub_title.trim().to_string()),
            Some("站点副标题".to_string()),
        ),
        (
            "site_api_base",
            "string",
            Some(payload.nehex.site_api_base.trim().to_string()),
            Some("API基础路径".to_string()),
        ),
        (
            "nehex_article_class",
            "json",
            Some(nehex_article_class.to_string()),
            Some("文章分类配置".to_string()),
        ),
        (
            "site_url",
            "string",
            Some(payload.site.site_url.trim().to_string()),
            Some("站点地址".to_string()),
        ),
        (
            "site_description",
            "string",
            Some(payload.site.site_description.trim().to_string()),
            Some("站点描述".to_string()),
        ),
        (
            "site_keywords",
            "string",
            Some(payload.site.site_keywords.trim().to_string()),
            Some("站点关键词".to_string()),
        ),
        (
            "site_icp",
            "string",
            Some(payload.site.site_icp.trim().to_string()),
            Some("ICP备案".to_string()),
        ),
        (
            "site_notice",
            "string",
            Some(site_notice),
            Some("站点公告".to_string()),
        ),
        (
            "theme_background",
            "string",
            Some(
                default_theme
                    .get("background_images")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
            ),
            Some("主题背景".to_string()),
        ),
        (
            "theme_primary",
            "string",
            Some(
                default_theme
                    .get("primary")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
            ),
            Some("主题主色".to_string()),
        ),
        (
            "theme_banner",
            "string",
            Some(
                default_theme
                    .get("banner")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
            ),
            Some("主题横幅".to_string()),
        ),
        (
            "theme_card_style",
            "string",
            Some(
                default_theme
                    .get("card_style")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
            ),
            Some("主题卡片风格".to_string()),
        ),
        (
            "theme_active_profile",
            "string",
            Some("rei.json".to_string()),
            Some("主题当前配置文件".to_string()),
        ),
        (
            "theme_profiles",
            "json",
            Some(theme_profiles.to_string()),
            Some("主题配置集合".to_string()),
        ),
    ] {
        upsert_setting_tx(&mut tx, key, setting_type, value, description).await?;
    }

    let article_exists = sqlx::query_scalar::<_, i64>("SELECT id::bigint FROM article LIMIT 1")
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to inspect article seed status: {error}"))
        })?;
    if article_exists.is_none() {
        sqlx::query(
            r#"
            INSERT INTO article (
                title,
                class,
                "articleTopImage",
                read,
                like_count,
                tag,
                top,
                status,
                content
            )
            VALUES ($1, $2, NULL, 0, 0, $3, 1, 1, $4)
            "#,
        )
        .bind(format!("{} 已完成初始化", site_title))
        .bind(primary_article_class)
        .bind("公告,示例")
        .bind(
            "欢迎使用 NeHex。\n\n这是系统在首次安装时自动创建的示例文章，你可以在后台管理中编辑或删除。",
        )
        .execute(&mut *tx)
        .await
        .map_err(|error| AppError::internal(format!("Failed to seed initial article: {error}")))?;
    }

    let page_exists = sqlx::query_scalar::<_, i64>("SELECT id::bigint FROM singlepage LIMIT 1")
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to inspect singlepage seed status: {error}"))
        })?;
    if page_exists.is_none() {
        sqlx::query(
            r#"
            INSERT INTO singlepage (
                page_key,
                title,
                cover_image,
                content,
                sort,
                status
            )
            VALUES ($1, $2, NULL, $3, 0, 1)
            "#,
        )
        .bind("about")
        .bind("关于本站")
        .bind(format!(
            "# 关于 {}\n\n本站已完成首次安装。\n\n这是系统自动创建的示例页面，你可以在后台管理中继续完善内容。",
            site_title,
        ))
        .execute(&mut *tx)
        .await
        .map_err(|error| AppError::internal(format!("Failed to seed initial page: {error}")))?;
    }

    tx.commit().await.map_err(|error| {
        AppError::internal(format!(
            "Failed to commit installation transaction: {error}"
        ))
    })?;
    state.runtime_cache.delete(SETTINGS_CACHE_KEY).await;
    state
        .runtime_cache
        .delete(SETTINGS_WITH_THEME_DETAILS_CACHE_KEY)
        .await;
    state.runtime_cache.delete(INSTALL_STATUS_CACHE_KEY).await;
    {
        let mut guard = state.admin_path_cache.write().await;
        *guard = None;
    }

    get_install_status(state).await
}

async fn upsert_setting(
    state: &AppState,
    key: &str,
    setting_type: &str,
    value: Option<String>,
    description: Option<&str>,
) -> AppResult<()> {
    let mut tx = state.db_pool.begin().await.map_err(|error| {
        AppError::internal(format!("Failed to start setting transaction: {error}"))
    })?;
    upsert_setting_tx(
        &mut tx,
        key,
        setting_type,
        value,
        description.map(|value| value.to_string()),
    )
    .await?;
    tx.commit().await.map_err(|error| {
        AppError::internal(format!("Failed to commit setting transaction: {error}"))
    })?;
    Ok(())
}

async fn upsert_setting_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: &str,
    setting_type: &str,
    value: Option<String>,
    description: Option<String>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO settings (setting_key, setting_type, setting_content, description)
        VALUES ($1, $2::setting_type, $3, $4)
        ON CONFLICT (setting_key)
        DO UPDATE SET
            setting_type = EXCLUDED.setting_type,
            setting_content = EXCLUDED.setting_content,
            description = EXCLUDED.description,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(key)
    .bind(setting_type)
    .bind(value)
    .bind(description)
    .execute(&mut **tx)
    .await
    .map_err(|error| AppError::internal(format!("Failed to upsert setting `{key}`: {error}")))?;

    Ok(())
}

fn parse_boolean(value: Option<&String>) -> bool {
    value
        .map(|value| {
            matches!(
                value.trim().to_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn hash_admin_password(password: &str, iterations: u32) -> String {
    let normalized = password.trim();
    let mut salt = [0_u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let mut digest = [0_u8; 32];
    pbkdf2_hmac::<Sha256>(
        normalized.as_bytes(),
        &salt,
        iterations.max(100_000),
        &mut digest,
    );

    format!(
        "{}${}${}${}",
        ADMIN_PASSWORD_SCHEME,
        iterations.max(100_000),
        hex::encode(salt),
        hex::encode(digest)
    )
}

pub fn hash_admin_password_for_setting(password: &str) -> String {
    hash_admin_password(password, ADMIN_PASSWORD_ITERATIONS)
}

fn verify_admin_password(password: &str, stored_password_hash: &str) -> (bool, bool) {
    let raw = stored_password_hash.trim();
    let normalized_password = password.trim();
    if raw.is_empty() || normalized_password.is_empty() {
        return (false, false);
    }

    let scheme_prefix = format!("{}$", ADMIN_PASSWORD_SCHEME);
    if raw.starts_with(&scheme_prefix) {
        let parts = raw.split('$').collect::<Vec<_>>();
        if parts.len() != 4 {
            return (false, false);
        }

        let iterations = parts[1].parse::<u32>().ok().unwrap_or(100_000).max(100_000);
        let salt = hex::decode(parts[2]).ok();
        let expected_digest = hex::decode(parts[3]).ok();
        let (Some(salt), Some(expected_digest)) = (salt, expected_digest) else {
            return (false, false);
        };

        let mut calculated = vec![0_u8; expected_digest.len()];
        pbkdf2_hmac::<Sha256>(
            normalized_password.as_bytes(),
            &salt,
            iterations,
            &mut calculated,
        );
        return (calculated == expected_digest, false);
    }

    let legacy_hash = raw.to_lowercase();
    if legacy_hash.len() == 64 && legacy_hash.chars().all(|ch| ch.is_ascii_hexdigit()) {
        let first = Sha256::digest(normalized_password.as_bytes());
        let second = Sha256::digest(hex::encode(first).as_bytes());
        let matched = hex::encode(second).to_lowercase() == legacy_hash;
        return (matched, matched);
    }

    (false, false)
}

fn create_admin_token(
    state: &AppState,
    account: &str,
    expires_at: i64,
    token_type: &str,
) -> AppResult<String> {
    let now = Utc::now().timestamp();
    let claims = AdminTokenClaims {
        account: account.to_string(),
        client: state.settings.admin_api_client_id.clone(),
        exp: expires_at,
        iat: now,
        jti: random_token_id(16),
        token_type: token_type.to_string(),
    };
    encode_token_claims(&state.settings.admin_api_secret, &claims)
}

fn encode_token_claims(secret: &str, claims: &AdminTokenClaims) -> AppResult<String> {
    let payload = serde_json::to_vec(claims)
        .map_err(|error| AppError::internal(format!("Failed to encode token payload: {error}")))?;
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload);

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|error| {
        AppError::internal(format!("Failed to initialize token signer: {error}"))
    })?;
    mac.update(payload_b64.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = URL_SAFE_NO_PAD.encode(signature);
    Ok(format!("{payload_b64}.{signature_b64}"))
}

fn decode_token_claims(secret: &str, token: &str) -> Result<AdminTokenClaims, AppError> {
    let (payload_b64, signature_b64) = token
        .split_once('.')
        .ok_or_else(|| AppError::Unauthorized("Invalid admin token".to_string()))?;

    let signature = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|_| AppError::Unauthorized("Invalid admin token".to_string()))?;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| AppError::Unauthorized("Invalid admin token".to_string()))?;
    mac.update(payload_b64.as_bytes());
    mac.verify_slice(&signature)
        .map_err(|_| AppError::Unauthorized("Invalid admin token".to_string()))?;

    let payload = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| AppError::Unauthorized("Invalid admin token".to_string()))?;
    serde_json::from_slice::<AdminTokenClaims>(&payload)
        .map_err(|_| AppError::Unauthorized("Invalid admin token".to_string()))
}

fn random_token_id(bytes: usize) -> String {
    let mut raw = vec![0_u8; bytes.max(8)];
    rand::thread_rng().fill_bytes(&mut raw);
    URL_SAFE_NO_PAD.encode(raw)
}

fn is_request_secure(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|value| value.trim().eq_ignore_ascii_case("https"))
        .unwrap_or(false)
}

fn append_set_cookie(response: &mut Response, cookie: String) {
    if let Ok(value) = header::HeaderValue::from_str(&cookie) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}

fn build_cookie(
    key: &str,
    value: &str,
    max_age: i64,
    secure: bool,
    http_only: bool,
    same_site: Option<&str>,
    domain: Option<&str>,
) -> String {
    let mut cookie = format!("{key}={value}; Path=/; Max-Age={}", max_age.max(0));
    if let Some(domain) = domain {
        if !domain.trim().is_empty() {
            cookie.push_str(&format!("; Domain={}", domain.trim()));
        }
    }
    if let Some(same_site) = same_site {
        cookie.push_str(&format!("; SameSite={same_site}"));
    }
    if http_only {
        cookie.push_str("; HttpOnly");
    }
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

fn extract_bearer_token(value: Option<&header::HeaderValue>) -> Option<String> {
    let text = value?.to_str().ok()?.trim();
    let (scheme, token) = text.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("bearer") && !token.trim().is_empty() {
        return Some(token.trim().to_string());
    }
    None
}

fn get_cookie_value(cookie_header: Option<&header::HeaderValue>, key: &str) -> Option<String> {
    let raw = cookie_header?.to_str().ok()?;
    for chunk in raw.split(';') {
        let (cookie_key, cookie_value) = chunk.trim().split_once('=')?;
        if cookie_key.trim() == key {
            let value = cookie_value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn ts_to_naive_datetime(timestamp: i64) -> NaiveDateTime {
    chrono::DateTime::<Utc>::from_timestamp(timestamp, 0)
        .map(|value| value.naive_utc())
        .unwrap_or_else(|| Utc::now().naive_utc())
}

fn default_theme_profiles() -> serde_json::Value {
    serde_json::json!({
        "rei.json": default_rei_theme_profile()
    })
}

fn default_rei_theme_profile() -> serde_json::Value {
    serde_json::from_str(include_str!("../defaults/rei_theme.json")).unwrap_or_else(|_| {
        serde_json::json!({
            "head_pic": "/images/head.jpg",
            "background_images": "/images/background-2k.png",
            "headmsg": "hi"
        })
    })
}

fn extract_forwarded_value(header_value: Option<&str>) -> String {
    header_value
        .unwrap_or_default()
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn normalize_request_host(value: Option<&str>) -> String {
    let host = value.unwrap_or_default().trim().to_lowercase();
    if host.is_empty() {
        return String::new();
    }
    if host.starts_with('[') {
        if let Some(end_index) = host.find(']') {
            if end_index > 0 {
                return host[1..end_index].to_string();
            }
        }
    }
    if host.matches(':').count() == 1 {
        return host
            .split(':')
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
    }
    host
}

fn parse_cookie_domain_candidates(raw_value: &str) -> Vec<String> {
    let normalized = raw_value.trim().to_lowercase();
    if normalized.is_empty() {
        return vec![];
    }

    let mut parsed = Vec::new();
    for chunk in normalized.replace(';', ",").split(',') {
        let mut value = chunk.trim().trim_end_matches('.').to_string();
        if value.is_empty() {
            continue;
        }
        if value.starts_with("*.") {
            value = value[2..].to_string();
        }
        value = value.trim_start_matches('.').to_string();
        if !value.is_empty() {
            parsed.push(value);
        }
    }
    parsed
}

fn resolve_cookie_domain(raw_setting: &str, headers: &HeaderMap) -> Option<String> {
    let candidates = parse_cookie_domain_candidates(raw_setting);
    if candidates.is_empty() {
        return None;
    }

    let forwarded_host = extract_forwarded_value(
        headers
            .get("x-forwarded-host")
            .and_then(|value| value.to_str().ok()),
    );
    let request_host = normalize_request_host(Some(&forwarded_host)).if_empty_then(|| {
        normalize_request_host(
            headers
                .get(header::HOST)
                .and_then(|value| value.to_str().ok()),
        )
    });

    if request_host.is_empty() {
        return candidates.first().cloned();
    }

    for domain in &candidates {
        if request_host == *domain || request_host.ends_with(&format!(".{domain}")) {
            return Some(domain.clone());
        }
    }

    None
}

fn build_delete_cookies(
    key: &str,
    secure: bool,
    raw_domain_setting: &str,
    headers: &HeaderMap,
) -> Vec<String> {
    let mut cookies = vec![
        build_cookie(key, "", 0, false, false, Some("lax"), None),
        build_cookie(key, "", 0, secure, false, Some("lax"), None),
    ];

    let matched_domain = resolve_cookie_domain(raw_domain_setting, headers);
    if let Some(domain) = matched_domain {
        cookies.push(build_cookie(
            key,
            "",
            0,
            false,
            false,
            Some("lax"),
            Some(&domain),
        ));
        cookies.push(build_cookie(
            key,
            "",
            0,
            secure,
            false,
            Some("lax"),
            Some(&domain),
        ));
    }

    let all_domains = parse_cookie_domain_candidates(raw_domain_setting);
    for domain in all_domains {
        cookies.push(build_cookie(
            key,
            "",
            0,
            false,
            false,
            Some("lax"),
            Some(&domain),
        ));
        cookies.push(build_cookie(
            key,
            "",
            0,
            secure,
            false,
            Some("lax"),
            Some(&domain),
        ));
    }

    cookies
}

fn request_origin(headers: &HeaderMap) -> String {
    let forwarded_proto = extract_forwarded_value(
        headers
            .get("x-forwarded-proto")
            .and_then(|value| value.to_str().ok()),
    )
    .to_lowercase();

    let forwarded_host = extract_forwarded_value(
        headers
            .get("x-forwarded-host")
            .and_then(|value| value.to_str().ok()),
    );

    let host = if !forwarded_host.is_empty() {
        forwarded_host
    } else {
        headers
            .get(header::HOST)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .trim()
            .to_string()
    };

    if host.is_empty() {
        return String::new();
    }

    let scheme = if forwarded_proto.is_empty() {
        "http".to_string()
    } else {
        forwarded_proto
    };

    format!("{scheme}://{host}").to_lowercase()
}

fn origin_from_header(value: &str) -> String {
    let normalized = value.trim();
    if normalized.is_empty() {
        return String::new();
    }

    let Ok(parsed) = Url::parse(normalized) else {
        return String::new();
    };

    let host = parsed.host_str().unwrap_or_default();
    if host.is_empty() {
        return String::new();
    }

    if let Some(port) = parsed.port() {
        format!("{}://{}:{}", parsed.scheme(), host, port).to_lowercase()
    } else {
        format!("{}://{}", parsed.scheme(), host).to_lowercase()
    }
}

fn enforce_csrf_same_origin(method: &Method, headers: &HeaderMap) -> AppResult<()> {
    if matches!(
        *method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    ) {
        return Ok(());
    }

    let expected_origin = request_origin(headers);
    if expected_origin.is_empty() {
        return Ok(());
    }

    let origin_header = headers
        .get("origin")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .to_string();
    let referer_header = headers
        .get("referer")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .trim()
        .to_string();

    if !origin_header.is_empty() {
        if origin_from_header(&origin_header) != expected_origin {
            return Err(AppError::Forbidden("CSRF origin mismatch".to_string()));
        }
        return Ok(());
    }

    if !referer_header.is_empty() {
        if origin_from_header(&referer_header) != expected_origin {
            return Err(AppError::Forbidden("CSRF referer mismatch".to_string()));
        }
        return Ok(());
    }

    Err(AppError::Forbidden(
        "Missing CSRF origin headers".to_string(),
    ))
}

trait StringExt {
    fn if_empty_then(self, f: impl FnOnce() -> String) -> String;
}

impl StringExt for String {
    fn if_empty_then(self, f: impl FnOnce() -> String) -> String {
        if self.is_empty() { f() } else { self }
    }
}
