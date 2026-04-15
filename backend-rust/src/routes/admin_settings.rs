use std::{collections::HashMap, time::Duration};

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, Method},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize, de::Error as DeError};
use serde_json::Value;
use sqlx::Row;
use url::Url;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

use super::{admin_auth, admin_mail, sync_api};

const SENSITIVE_ADMIN_SETTING_KEYS: &[&str] = &["user_account", "user_account_password"];
const SETTINGS_CACHE_KEY: &str = "settings:list";
const SETTINGS_WITH_THEME_DETAILS_CACHE_KEY: &str = "settings:list:with-theme-details";
const INSTALL_STATUS_CACHE_KEY: &str = "admin:install:status";
const KUMA_API_DEFAULT_HELLO: &str =
    "hello,welcome to Kuma API; Visite: https://github.com/nehex/kuma-api";
const KUMA_API_TEST_TIMEOUT_SECONDS: u64 = 8;
const KUMA_API_MOVIE_TIMEOUT_SECONDS: u64 = 10;

#[derive(Serialize)]
struct SettingItem {
    setting_key: String,
    setting_type: String,
    setting_content: Value,
    description: Option<String>,
    updated_at: NaiveDateTime,
    created_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct AdminSettingListResponse {
    data: Vec<SettingItem>,
}

#[derive(Deserialize)]
pub struct AdminSettingsUpdateRequest {
    items: Vec<AdminSettingUpdateItem>,
}

#[derive(Deserialize)]
pub struct AdminSettingUpdateItem {
    setting_key: String,
    setting_content: Value,
    setting_type: Option<String>,
    #[serde(default)]
    description: OptionalField<String>,
}

#[derive(Deserialize)]
pub struct AdminAccountSettingsUpdateRequest {
    account: Option<String>,
    new_password: Option<String>,
    confirm_password: Option<String>,
}

#[derive(Deserialize)]
pub struct AdminKumaApiTestRequest {
    url: String,
}

#[derive(Serialize)]
pub struct AdminKumaApiTestResponse {
    success: bool,
    message: String,
    normalized_url: String,
    response_preview: String,
}

#[derive(Serialize)]
pub struct AdminKumaMovieResponse {
    success: bool,
    provider: String,
    movie_id: String,
    data: AdminKumaMovieItem,
}

#[derive(Serialize)]
pub struct AdminKumaMovieItem {
    cover: String,
    title: String,
    years: String,
    desc: String,
    url: String,
    score: Option<String>,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct AdminKumaMovieCard {
    id: i64,
    provider: String,
    movie_id: String,
    cover: String,
    title: String,
    years: String,
    score: Option<String>,
    desc: String,
    url: String,
    create_time: NaiveDateTime,
    update_time: NaiveDateTime,
}

#[derive(Serialize)]
pub struct AdminKumaMovieListResponse {
    data: Vec<AdminKumaMovieCard>,
}

#[derive(Serialize)]
pub struct AdminKumaMovieDetailResponse {
    data: AdminKumaMovieCard,
}

#[derive(Serialize)]
pub struct AdminKumaMovieActionResponse {
    success: bool,
    message: String,
}

#[derive(Deserialize)]
pub struct AdminKumaMovieCreateRequest {
    provider: String,
    movie_id: String,
}

#[derive(Debug, Clone, Default)]
enum OptionalField<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<'de, T> Deserialize<'de> for OptionalField<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = Option::<Value>::deserialize(deserializer)?;
        match raw {
            None => Ok(Self::Null),
            Some(value) => T::deserialize(value)
                .map(Self::Value)
                .map_err(D::Error::custom),
        }
    }
}

pub async fn admin_list_settings(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> AppResult<Json<AdminSettingListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let data = list_admin_settings(&state).await?;
    Ok(Json(AdminSettingListResponse { data }))
}

pub async fn admin_update_settings(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<AdminSettingsUpdateRequest>,
) -> AppResult<Json<AdminSettingListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    if payload.items.is_empty() {
        return Err(AppError::Unprocessable("items is required".to_string()));
    }

    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|error| AppError::internal(format!("Failed to begin settings tx: {error}")))?;

    let mut pending = Vec::<(String, AdminSettingUpdateItem)>::new();
    for item in payload.items {
        let key = item.setting_key.trim().to_string();
        if key.is_empty() {
            continue;
        }
        if SENSITIVE_ADMIN_SETTING_KEYS.contains(&key.as_str()) {
            return Err(AppError::Unprocessable(format!(
                "Setting {key} must be updated via /admin-api/settings/account"
            )));
        }
        pending.push((key, item));
    }

    if pending.is_empty() {
        return Err(AppError::Unprocessable(
            "No valid setting items to update".to_string(),
        ));
    }

    let existing_rows = sqlx::query(
        "SELECT setting_key, setting_type::text AS setting_type FROM settings WHERE setting_key = ANY($1)",
    )
    .bind(
        pending
            .iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<String>>(),
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|error| AppError::internal(format!("Failed to preload setting types: {error}")))?;

    let mut existing_type_map = HashMap::<String, String>::new();
    for row in existing_rows {
        if let (Ok(key), Ok(setting_type)) = (
            row.try_get::<String, _>("setting_key"),
            row.try_get::<String, _>("setting_type"),
        ) {
            existing_type_map.insert(key, setting_type);
        }
    }

    for (key, item) in pending {
        let effective_type = item
            .setting_type
            .clone()
            .unwrap_or_else(|| {
                existing_type_map
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| "string".to_string())
            })
            .trim()
            .to_lowercase();

        if !is_supported_setting_type(&effective_type) {
            return Err(AppError::Unprocessable(format!(
                "Unsupported setting_type: {effective_type}"
            )));
        }

        let serialized_content = serialize_setting_content(&effective_type, item.setting_content)?;
        let (description, has_description) = match item.description {
            OptionalField::Missing => (None, false),
            OptionalField::Null => (None, true),
            OptionalField::Value(value) => (normalize_optional_text(value), true),
        };

        sqlx::query(
            r#"
            INSERT INTO settings (setting_key, setting_type, setting_content, description)
            VALUES ($1, $2::setting_type, $3, $4)
            ON CONFLICT (setting_key)
            DO UPDATE SET
                setting_type = EXCLUDED.setting_type,
                setting_content = EXCLUDED.setting_content,
                description = CASE
                    WHEN $5 THEN EXCLUDED.description
                    ELSE settings.description
                END,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&key)
        .bind(&effective_type)
        .bind(serialized_content)
        .bind(description)
        .bind(has_description)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to update setting `{key}`: {error}"))
        })?;
        existing_type_map.insert(key, effective_type);
    }

    tx.commit()
        .await
        .map_err(|error| AppError::internal(format!("Failed to commit settings tx: {error}")))?;
    invalidate_settings_cache(&state).await;
    admin_mail::invalidate_mail_settings_cache();
    {
        let mut guard = state.admin_path_cache.write().await;
        *guard = None;
    }
    sync_api::record_content_change_best_effort(&state, "setting", "update", vec![]).await;

    let data = list_admin_settings(&state).await?;
    Ok(Json(AdminSettingListResponse { data }))
}

pub async fn admin_update_account_settings(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<AdminAccountSettingsUpdateRequest>,
) -> AppResult<Json<AdminSettingListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let has_account = payload.account.is_some();
    let has_password = payload.new_password.is_some();
    if !has_account && !has_password {
        return Err(AppError::Unprocessable(
            "No account settings fields to update".to_string(),
        ));
    }

    let normalized_account = payload.account.and_then(normalize_optional_text);
    let normalized_password = payload.new_password.and_then(normalize_optional_text);

    if normalized_password.is_some() {
        let confirm_password = payload
            .confirm_password
            .and_then(normalize_optional_text)
            .ok_or_else(|| {
                AppError::Unprocessable(
                    "confirm_password is required when new_password is provided".to_string(),
                )
            })?;
        if confirm_password != normalized_password.clone().unwrap_or_default() {
            return Err(AppError::Unprocessable(
                "new_password and confirm_password do not match".to_string(),
            ));
        }
    }

    let mut tx = state.db_pool.begin().await.map_err(|error| {
        AppError::internal(format!("Failed to begin account settings tx: {error}"))
    })?;

    if let Some(account) = normalized_account {
        sqlx::query(
            r#"
            INSERT INTO settings (setting_key, setting_type, setting_content, description)
            VALUES ('user_account', 'string', $1, '管理员账号')
            ON CONFLICT (setting_key)
            DO UPDATE SET
                setting_type = 'string',
                setting_content = EXCLUDED.setting_content,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(account)
        .execute(&mut *tx)
        .await
        .map_err(|error| AppError::internal(format!("Failed to update user_account: {error}")))?;
    }

    if let Some(password) = normalized_password {
        let hashed = admin_auth::hash_admin_password_for_setting(&password);
        sqlx::query(
            r#"
            INSERT INTO settings (setting_key, setting_type, setting_content, description)
            VALUES ('user_account_password', 'string', $1, '管理员密码（哈希）')
            ON CONFLICT (setting_key)
            DO UPDATE SET
                setting_type = 'string',
                setting_content = EXCLUDED.setting_content,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(hashed)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to update user_account_password: {error}"))
        })?;
    }

    tx.commit().await.map_err(|error| {
        AppError::internal(format!("Failed to commit account settings tx: {error}"))
    })?;
    invalidate_settings_cache(&state).await;
    admin_mail::invalidate_mail_settings_cache();
    sync_api::record_content_change_best_effort(&state, "setting", "update", vec![]).await;

    let data = list_admin_settings(&state).await?;
    Ok(Json(AdminSettingListResponse { data }))
}

pub async fn admin_test_kuma_api_url(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<AdminKumaApiTestRequest>,
) -> AppResult<Json<AdminKumaApiTestResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let normalized_url = normalize_kuma_api_url(payload.url)?;

    let mut client_builder =
        reqwest::Client::builder().timeout(Duration::from_secs(KUMA_API_TEST_TIMEOUT_SECONDS));
    if should_bypass_env_proxy_for_kuma() {
        // Loopback proxies (127.0.0.1/localhost) are frequently invalid inside containers and
        // can break outbound connectivity to public Kuma endpoints.
        client_builder = client_builder.no_proxy();
    }
    let client = client_builder
        .build()
        .map_err(|error| AppError::internal(format!("Failed to build HTTP client: {error}")))?;

    let response = client
        .get(normalized_url.as_str())
        .send()
        .await
        .map_err(|error| {
            AppError::Unprocessable(format!("Kuma API 请求失败: {error} (debug: {error:?})"))
        })?;

    if !response.status().is_success() {
        return Err(AppError::Unprocessable(format!(
            "Kuma API 返回状态异常: {}",
            response.status()
        )));
    }

    let body = response
        .json::<Value>()
        .await
        .map_err(|error| AppError::Unprocessable(format!("Kuma API 返回非 JSON: {error}")))?;

    let first_message = body
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();

    if first_message != KUMA_API_DEFAULT_HELLO {
        return Err(AppError::Unprocessable(format!(
            "Kuma API 返回内容不符合预期: {first_message}"
        )));
    }

    Ok(Json(AdminKumaApiTestResponse {
        success: true,
        message: "连接成功，Kuma-API 可用".to_string(),
        normalized_url,
        response_preview: first_message,
    }))
}

pub async fn admin_get_kuma_movie(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path((provider_raw, movie_id_raw)): Path<(String, String)>,
) -> AppResult<Json<AdminKumaMovieResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let provider = normalize_kuma_movie_provider(provider_raw)?;
    let movie_id = normalize_kuma_movie_id(movie_id_raw)?;
    let kuma_api_url = load_kuma_api_url_from_settings(&state).await?;
    let item = request_kuma_movie_item(&provider, &movie_id, &kuma_api_url).await?;

    Ok(Json(AdminKumaMovieResponse {
        success: true,
        provider,
        movie_id,
        data: item,
    }))
}

pub async fn admin_list_kuma_movies(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> AppResult<Json<AdminKumaMovieListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    ensure_kuma_movie_table(&state).await?;

    let rows = sqlx::query_as::<_, AdminKumaMovieCard>(
        r#"
        SELECT
            id::bigint AS id,
            provider,
            movie_id,
            COALESCE(cover, '') AS cover,
            title,
            COALESCE(years, '') AS years,
            score,
            COALESCE(description, '') AS desc,
            COALESCE(source_url, '') AS url,
            create_time,
            update_time
        FROM kuma_movie
        ORDER BY create_time DESC, id DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list kuma movies: {error}")))?;

    Ok(Json(AdminKumaMovieListResponse { data: rows }))
}

pub async fn admin_create_kuma_movie(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<AdminKumaMovieCreateRequest>,
) -> AppResult<Json<AdminKumaMovieDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    ensure_kuma_movie_table(&state).await?;

    let provider = normalize_kuma_movie_provider(payload.provider)?;
    let movie_id = normalize_kuma_movie_id(payload.movie_id)?;
    let kuma_api_url = load_kuma_api_url_from_settings(&state).await?;
    let item = request_kuma_movie_item(&provider, &movie_id, &kuma_api_url).await?;

    let existing_id = sqlx::query_scalar::<_, i64>(
        "SELECT id::bigint FROM kuma_movie WHERE provider = $1 AND movie_id = $2 LIMIT 1",
    )
    .bind(&provider)
    .bind(&movie_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to query kuma movie: {error}")))?;

    let row = if let Some(id) = existing_id {
        sqlx::query_as::<_, AdminKumaMovieCard>(
            r#"
            UPDATE kuma_movie
            SET
                cover = $2,
                title = $3,
                years = $4,
                score = $5,
                description = $6,
                source_url = $7,
                update_time = CURRENT_TIMESTAMP
            WHERE id = $1
            RETURNING
                id::bigint AS id,
                provider,
                movie_id,
                COALESCE(cover, '') AS cover,
                title,
                COALESCE(years, '') AS years,
                score,
                COALESCE(description, '') AS desc,
                COALESCE(source_url, '') AS url,
                create_time,
                update_time
            "#,
        )
        .bind(id)
        .bind(item.cover)
        .bind(item.title)
        .bind(item.years)
        .bind(item.score)
        .bind(item.desc)
        .bind(item.url)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to update kuma movie: {error}")))?
    } else {
        sqlx::query_as::<_, AdminKumaMovieCard>(
            r#"
            INSERT INTO kuma_movie (
                provider,
                movie_id,
                cover,
                title,
                years,
                score,
                description,
                source_url
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            RETURNING
                id::bigint AS id,
                provider,
                movie_id,
                COALESCE(cover, '') AS cover,
                title,
                COALESCE(years, '') AS years,
                score,
                COALESCE(description, '') AS desc,
                COALESCE(source_url, '') AS url,
                create_time,
                update_time
            "#,
        )
        .bind(&provider)
        .bind(&movie_id)
        .bind(item.cover)
        .bind(item.title)
        .bind(item.years)
        .bind(item.score)
        .bind(item.desc)
        .bind(item.url)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to create kuma movie: {error}")))?
    };

    Ok(Json(AdminKumaMovieDetailResponse { data: row }))
}

pub async fn admin_delete_kuma_movie(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> AppResult<Json<AdminKumaMovieActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    ensure_kuma_movie_table(&state).await?;

    let result = sqlx::query("DELETE FROM kuma_movie WHERE id = $1")
        .bind(id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to delete kuma movie: {error}")))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("电影卡片不存在或已删除".to_string()));
    }

    Ok(Json(AdminKumaMovieActionResponse {
        success: true,
        message: "电影卡片已删除".to_string(),
    }))
}

async fn list_admin_settings(state: &AppState) -> AppResult<Vec<SettingItem>> {
    let rows = sqlx::query(
        r#"
        SELECT setting_key, setting_type::text AS setting_type, setting_content, description, updated_at, created_at
        FROM settings
        WHERE setting_key <> ALL($1)
        ORDER BY setting_key ASC
        "#,
    )
    .bind(SENSITIVE_ADMIN_SETTING_KEYS)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list settings: {error}")))?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let setting_type = row
            .try_get::<String, _>("setting_type")
            .unwrap_or_else(|_| "string".to_string())
            .to_lowercase();
        let raw_content = row
            .try_get::<Option<String>, _>("setting_content")
            .ok()
            .flatten();

        items.push(SettingItem {
            setting_key: row.try_get::<String, _>("setting_key").unwrap_or_default(),
            setting_type: setting_type.clone(),
            setting_content: parse_setting_content(&setting_type, raw_content.as_deref()),
            description: row
                .try_get::<Option<String>, _>("description")
                .ok()
                .flatten(),
            updated_at: row
                .try_get::<NaiveDateTime, _>("updated_at")
                .unwrap_or_else(|_| UtcNow::now()),
            created_at: row
                .try_get::<NaiveDateTime, _>("created_at")
                .unwrap_or_else(|_| UtcNow::now()),
        });
    }

    Ok(items)
}

fn parse_setting_content(setting_type: &str, raw_content: Option<&str>) -> Value {
    let raw = raw_content.unwrap_or_default();
    match setting_type {
        "int" => raw
            .trim()
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        "float" => raw
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or_else(|| Value::String(raw.to_string())),
        "boolean" => Value::Bool(matches!(
            raw.trim().to_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )),
        "json" => serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string())),
        _ => Value::String(raw.to_string()),
    }
}

fn serialize_setting_content(setting_type: &str, value: Value) -> AppResult<Option<String>> {
    let result = match setting_type {
        "string" => Some(value_to_string(value)),
        "int" => Some(value_to_i64_string(value)?),
        "float" => Some(value_to_f64_string(value)?),
        "boolean" => Some(value_to_bool_string(value)),
        "json" => Some(value_to_json_string(value)?),
        _ => {
            return Err(AppError::Unprocessable(
                "Unsupported setting_type".to_string(),
            ));
        }
    };
    Ok(result)
}

fn is_supported_setting_type(value: &str) -> bool {
    matches!(value, "string" | "int" | "float" | "boolean" | "json")
}

fn should_bypass_env_proxy_for_kuma() -> bool {
    [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ]
    .iter()
    .filter_map(|key| std::env::var(key).ok())
    .any(|value| is_loopback_proxy(&value))
}

fn is_loopback_proxy(raw: &str) -> bool {
    let text = raw.trim();
    if text.is_empty() {
        return false;
    }

    let normalized = if text.contains("://") {
        text.to_string()
    } else {
        format!("http://{text}")
    };

    Url::parse(&normalized)
        .ok()
        .and_then(|url| url.host_str().map(|host| host.to_ascii_lowercase()))
        .map(|host| matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1" | "0.0.0.0"))
        .unwrap_or(false)
}

fn normalize_kuma_api_url(raw: String) -> AppResult<String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err(AppError::Unprocessable(
            "Kuma API 地址不能为空".to_string(),
        ));
    }

    let normalized = if text.starts_with("http://") || text.starts_with("https://") {
        text.to_string()
    } else if text.starts_with("//") {
        format!("https:{text}")
    } else {
        format!("https://{}", text.trim_start_matches('/'))
    };

    let parsed = Url::parse(&normalized)
        .map_err(|error| AppError::Unprocessable(format!("Kuma API 地址格式错误: {error}")))?;

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(AppError::Unprocessable(
            "Kuma API 地址仅支持 http/https".to_string(),
        ));
    }
    if parsed.host_str().is_none() {
        return Err(AppError::Unprocessable(
            "Kuma API 地址缺少主机名".to_string(),
        ));
    }

    Ok(parsed.to_string())
}

async fn load_kuma_api_url_from_settings(state: &AppState) -> AppResult<String> {
    let row = sqlx::query("SELECT setting_content FROM settings WHERE setting_key = $1 LIMIT 1")
        .bind("kuma_api_url")
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to load kuma_api_url from settings: {error}"))
        })?;

    let raw_value = row
        .and_then(|item| item.try_get::<Option<String>, _>("setting_content").ok().flatten())
        .unwrap_or_default();

    if raw_value.trim().is_empty() {
        return Err(AppError::Unprocessable(
            "请先前往设定 -> NeHex配置，配置 Kuma-API 地址".to_string(),
        ));
    }

    normalize_kuma_api_url(raw_value)
}

async fn ensure_kuma_movie_table(state: &AppState) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS kuma_movie (
            id BIGSERIAL PRIMARY KEY,
            provider VARCHAR(20) NOT NULL,
            movie_id VARCHAR(120) NOT NULL,
            cover VARCHAR(1200),
            title VARCHAR(500) NOT NULL,
            years VARCHAR(120),
            score VARCHAR(60),
            description TEXT,
            source_url VARCHAR(1200),
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            update_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to ensure kuma_movie table: {error}")))?;

    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS uq_kuma_movie_provider_movie_id ON kuma_movie (provider, movie_id)",
    )
    .execute(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to ensure kuma_movie unique index: {error}")))?;

    Ok(())
}

fn normalize_kuma_movie_provider(raw: String) -> AppResult<String> {
    let provider = raw.trim().to_lowercase();
    if matches!(provider.as_str(), "douban" | "tmdb") {
        Ok(provider)
    } else {
        Err(AppError::Unprocessable(
            "provider 仅支持 douban / tmdb".to_string(),
        ))
    }
}

fn normalize_kuma_movie_id(raw: String) -> AppResult<String> {
    let movie_id = raw.trim().to_string();
    if movie_id.is_empty() {
        return Err(AppError::Unprocessable("movie_id 不能为空".to_string()));
    }

    if !movie_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(AppError::Unprocessable(
            "movie_id 仅支持字母、数字、-、_".to_string(),
        ));
    }

    Ok(movie_id)
}

fn parse_kuma_movie_payload(source: Value) -> AppResult<AdminKumaMovieItem> {
    let object = source
        .as_object()
        .ok_or_else(|| AppError::Unprocessable("Kuma API 返回数据格式异常".to_string()))?;

    let read_text = |key: &str| -> String {
        object
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string()
    };

    let title = read_text("title");
    if title.is_empty() {
        return Err(AppError::Unprocessable(
            "Kuma API 返回缺少 title 字段".to_string(),
        ));
    }

    let score = read_text("score");

    Ok(AdminKumaMovieItem {
        cover: read_text("cover"),
        title,
        years: read_text("years"),
        desc: read_text("desc"),
        url: read_text("url"),
        score: if score.is_empty() { None } else { Some(score) },
    })
}

async fn request_kuma_movie_item(
    provider: &str,
    movie_id: &str,
    kuma_api_url: &str,
) -> AppResult<AdminKumaMovieItem> {
    let endpoint_url = format!(
        "{}/{}/{}",
        kuma_api_url.trim_end_matches('/'),
        provider,
        movie_id
    );

    let mut client_builder =
        reqwest::Client::builder().timeout(Duration::from_secs(KUMA_API_MOVIE_TIMEOUT_SECONDS));
    if should_bypass_env_proxy_for_kuma() {
        client_builder = client_builder.no_proxy();
    }
    let client = client_builder
        .build()
        .map_err(|error| AppError::internal(format!("Failed to build HTTP client: {error}")))?;

    let response = client
        .get(endpoint_url)
        .send()
        .await
        .map_err(|error| AppError::Unprocessable(format!("Kuma API 请求失败: {error}")))?;

    if !response.status().is_success() {
        return Err(AppError::Unprocessable(format!(
            "Kuma API 返回状态异常: {}",
            response.status()
        )));
    }

    let body = response
        .json::<Value>()
        .await
        .map_err(|error| AppError::Unprocessable(format!("Kuma API 返回非 JSON: {error}")))?;

    let source = body
        .get(provider)
        .ok_or_else(|| AppError::Unprocessable(format!("Kuma API 响应缺少 `{provider}` 字段")))?
        .clone();

    parse_kuma_movie_payload(source)
}

fn normalize_optional_text(value: String) -> Option<String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn value_to_string(value: Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text,
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => {
            if flag {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        other => other.to_string(),
    }
}

fn value_to_i64_string(value: Value) -> AppResult<String> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .map(|item| item.to_string())
            .ok_or_else(|| AppError::Unprocessable("Invalid integer setting value".to_string())),
        Value::String(text) => text
            .trim()
            .parse::<i64>()
            .map(|item| item.to_string())
            .map_err(|_| AppError::Unprocessable("Invalid integer setting value".to_string())),
        _ => Err(AppError::Unprocessable(
            "Invalid integer setting value".to_string(),
        )),
    }
}

fn value_to_f64_string(value: Value) -> AppResult<String> {
    match value {
        Value::Number(number) => number
            .as_f64()
            .map(|item| item.to_string())
            .ok_or_else(|| AppError::Unprocessable("Invalid float setting value".to_string())),
        Value::String(text) => text
            .trim()
            .parse::<f64>()
            .map(|item| item.to_string())
            .map_err(|_| AppError::Unprocessable("Invalid float setting value".to_string())),
        _ => Err(AppError::Unprocessable(
            "Invalid float setting value".to_string(),
        )),
    }
}

fn value_to_bool_string(value: Value) -> String {
    match value {
        Value::Bool(flag) => {
            if flag {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(number) => {
            if number.as_i64().unwrap_or_default() != 0 {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::String(text) => {
            if matches!(
                text.trim().to_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ) {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        _ => "false".to_string(),
    }
}

fn value_to_json_string(value: Value) -> AppResult<String> {
    if let Value::String(text) = value {
        let normalized = text.trim();
        if normalized.is_empty() {
            return Ok(String::new());
        }
        if let Ok(parsed) = serde_json::from_str::<Value>(normalized) {
            return serde_json::to_string(&parsed).map_err(|error| {
                AppError::internal(format!("Failed to encode JSON setting: {error}"))
            });
        }
        return Ok(text);
    }

    serde_json::to_string(&value)
        .map_err(|error| AppError::internal(format!("Failed to encode JSON setting: {error}")))
}

struct UtcNow;

impl UtcNow {
    fn now() -> NaiveDateTime {
        chrono::Utc::now().naive_utc()
    }
}

async fn invalidate_settings_cache(state: &AppState) {
    state.runtime_cache.delete(SETTINGS_CACHE_KEY).await;
    state
        .runtime_cache
        .delete(SETTINGS_WITH_THEME_DETAILS_CACHE_KEY)
        .await;
    state.runtime_cache.delete(INSTALL_STATUS_CACHE_KEY).await;
}
