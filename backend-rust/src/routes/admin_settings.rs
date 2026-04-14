use std::collections::HashMap;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, Method},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize, de::Error as DeError};
use serde_json::Value;
use sqlx::Row;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

use super::{admin_auth, admin_mail, sync_api};

const SENSITIVE_ADMIN_SETTING_KEYS: &[&str] = &["user_account", "user_account_password"];
const SETTINGS_CACHE_KEY: &str = "settings:list";
const SETTINGS_WITH_THEME_DETAILS_CACHE_KEY: &str = "settings:list:with-theme-details";
const INSTALL_STATUS_CACHE_KEY: &str = "admin:install:status";

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
