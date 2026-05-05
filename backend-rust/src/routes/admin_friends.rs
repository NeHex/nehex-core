use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, Method},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use url::Url;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

use super::{admin_auth, sync_api};

const FRIENDS_CACHE_KEY: &str = "friends:list";
const FRIEND_EXCHANGE_SITE_TITLE_KEY: &str = "friend_exchange_site_title";
const FRIEND_EXCHANGE_SITE_URL_KEY: &str = "friend_exchange_site_url";
const FRIEND_EXCHANGE_SITE_ICON_KEY: &str = "friend_exchange_site_icon";
const FRIEND_EXCHANGE_SITE_DESCRIPTION_KEY: &str = "friend_exchange_site_description";
const SITE_TITLE_KEY: &str = "site_title";
const SITE_URL_KEY: &str = "site_url";
const SITE_FAVICON_KEY: &str = "site_favicon";
const SITE_DESCRIPTION_KEY: &str = "site_description";

#[derive(Serialize)]
pub struct AdminActionResponse {
    success: bool,
    message: String,
}

#[derive(sqlx::FromRow, Serialize)]
struct FriendItem {
    id: i64,
    title: String,
    description: Option<String>,
    category: String,
    favicon: Option<String>,
    url: String,
    status: String,
    create_time: NaiveDateTime,
}

#[derive(sqlx::FromRow, Serialize)]
struct FriendApplyItem {
    id: i64,
    site_title: String,
    site_url: String,
    site_description: Option<String>,
    site_icon: Option<String>,
    contact: Option<String>,
    status: String,
    ip: Option<String>,
    user_agent: Option<String>,
    create_time: NaiveDateTime,
    update_time: NaiveDateTime,
}

#[derive(Serialize)]
pub struct AdminFriendListResponse {
    data: Vec<FriendItem>,
}

#[derive(Serialize)]
pub struct AdminFriendDetailResponse {
    data: FriendItem,
}

#[derive(Serialize)]
pub struct AdminFriendApplyListResponse {
    data: Vec<FriendApplyItem>,
}

#[derive(Serialize)]
pub struct AdminFriendApplyDetailResponse {
    data: FriendApplyItem,
}

#[derive(Serialize, Clone)]
pub struct FriendExchangeInfoItem {
    site_title: String,
    site_url: String,
    site_icon: String,
    site_description: String,
}

#[derive(Serialize)]
pub struct FriendExchangeInfoResponse {
    data: FriendExchangeInfoItem,
}

#[derive(Deserialize)]
pub struct AdminFriendListQuery {
    keyword: Option<String>,
}

#[derive(Deserialize)]
pub struct AdminFriendApplyListQuery {
    #[serde(rename = "status")]
    status_filter: Option<String>,
    keyword: Option<String>,
}

#[derive(Deserialize)]
pub struct AdminFriendCreatePayload {
    title: String,
    description: Option<String>,
    category: String,
    favicon: Option<String>,
    url: String,
    status: String,
    overwrite_existing: Option<bool>,
}

#[derive(Deserialize)]
pub struct AdminFriendUpdatePayload {
    title: Option<String>,
    description: Option<String>,
    category: Option<String>,
    favicon: Option<String>,
    url: Option<String>,
    status: Option<String>,
}

#[derive(Deserialize)]
pub struct AdminFriendApplyStatusPayload {
    status: String,
    create_friend: Option<bool>,
    friend_category: Option<String>,
}

#[derive(Deserialize)]
pub struct FriendExchangeInfoUpdatePayload {
    site_title: String,
    site_url: String,
    site_icon: String,
    site_description: String,
}

pub async fn admin_list_friends(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Query(query): Query<AdminFriendListQuery>,
) -> AppResult<Json<AdminFriendListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let keyword = query.keyword.unwrap_or_default().trim().to_string();
    let like_pattern = format!("%{keyword}%");

    let data = if keyword.is_empty() {
        sqlx::query_as::<_, FriendItem>(
            r#"
            SELECT id::bigint AS id, title, description, category, favicon, url, status, create_time
            FROM friends
            ORDER BY
                CASE
                    WHEN status = 'ok' THEN 0
                    WHEN status = 'missing' THEN 1
                    WHEN status = 'blocked' THEN 2
                    ELSE 3
                END,
                create_time DESC,
                id DESC
            "#,
        )
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list friends: {error}")))?
    } else {
        sqlx::query_as::<_, FriendItem>(
            r#"
            SELECT id::bigint AS id, title, description, category, favicon, url, status, create_time
            FROM friends
            WHERE
                title LIKE $1
                OR COALESCE(description, '') LIKE $1
                OR category LIKE $1
                OR url LIKE $1
                OR status LIKE $1
                OR CAST(id AS TEXT) LIKE $1
            ORDER BY
                CASE
                    WHEN status = 'ok' THEN 0
                    WHEN status = 'missing' THEN 1
                    WHEN status = 'blocked' THEN 2
                    ELSE 3
                END,
                create_time DESC,
                id DESC
            "#,
        )
        .bind(&like_pattern)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list friends: {error}")))?
    };

    Ok(Json(AdminFriendListResponse { data }))
}

pub async fn admin_get_friend_exchange_info(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> AppResult<Json<FriendExchangeInfoResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let data = load_friend_exchange_info(&state).await?;
    Ok(Json(FriendExchangeInfoResponse { data }))
}

pub async fn admin_update_friend_exchange_info(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<FriendExchangeInfoUpdatePayload>,
) -> AppResult<Json<FriendExchangeInfoResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let site_title = normalize_optional_text(Some(payload.site_title));
    let site_url = match normalize_optional_text(Some(payload.site_url)) {
        Some(url) => Some(
            normalize_http_url(&url, "site_url", false)?
                .ok_or_else(|| AppError::Unprocessable("site_url is required".to_string()))?,
        ),
        None => None,
    };
    let site_icon = match normalize_optional_text(Some(payload.site_icon)) {
        Some(icon) => normalize_http_url(&icon, "site_icon", true)?,
        None => None,
    };
    let site_description = normalize_optional_text(Some(payload.site_description));

    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|error| AppError::internal(format!("Failed to begin friend exchange tx: {error}")))?;

    upsert_setting(
        &mut tx,
        FRIEND_EXCHANGE_SITE_TITLE_KEY,
        "string",
        site_title,
        "友链交换信息：站点标题",
    )
    .await?;
    upsert_setting(
        &mut tx,
        FRIEND_EXCHANGE_SITE_URL_KEY,
        "string",
        site_url,
        "友链交换信息：站点链接",
    )
    .await?;
    upsert_setting(
        &mut tx,
        FRIEND_EXCHANGE_SITE_ICON_KEY,
        "string",
        site_icon,
        "友链交换信息：站点图标",
    )
    .await?;
    upsert_setting(
        &mut tx,
        FRIEND_EXCHANGE_SITE_DESCRIPTION_KEY,
        "string",
        site_description,
        "友链交换信息：站点描述",
    )
    .await?;

    tx.commit()
        .await
        .map_err(|error| AppError::internal(format!("Failed to commit friend exchange settings: {error}")))?;

    let data = load_friend_exchange_info(&state).await?;
    sync_api::record_content_change_best_effort(&state, "setting", "update", vec![]).await;
    Ok(Json(FriendExchangeInfoResponse { data }))
}

pub async fn admin_create_friend(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<AdminFriendCreatePayload>,
) -> AppResult<Json<AdminFriendDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let title = normalize_required_text(payload.title, "title")?;
    let category = normalize_required_text(payload.category, "category")?;
    let url = normalize_http_url(&payload.url, "url", false)?
        .ok_or_else(|| AppError::Unprocessable("url is required".to_string()))?;
    let favicon = normalize_http_url(
        payload.favicon.as_deref().unwrap_or_default(),
        "favicon",
        true,
    )?;
    let status = normalize_friend_status(payload.status)?;
    let overwrite_existing = payload.overwrite_existing.unwrap_or(false);

    let existing = sqlx::query("SELECT id::bigint AS id FROM friends WHERE url = $1 LIMIT 1")
        .bind(&url)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to inspect existing friend: {error}"))
        })?;

    let (data, event_action) = if let Some(existing_row) = existing {
        if !overwrite_existing {
            return Err(AppError::Conflict("Friend URL already exists".to_string()));
        }

        let friend_id = existing_row.try_get::<i64, _>("id").unwrap_or_default();
        let data = sqlx::query_as::<_, FriendItem>(
            r#"
            UPDATE friends
            SET
                title = $2,
                description = $3,
                category = $4,
                favicon = $5,
                url = $6,
                status = $7
            WHERE id = $1
            RETURNING id::bigint AS id, title, description, category, favicon, url, status, create_time
            "#,
        )
        .bind(friend_id)
        .bind(title)
        .bind(normalize_optional_text(payload.description))
        .bind(category)
        .bind(favicon)
        .bind(url)
        .bind(status)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to overwrite friend: {error}")))?;
        (data, "update")
    } else {
        let data = sqlx::query_as::<_, FriendItem>(
            r#"
            INSERT INTO friends (title, description, category, favicon, url, status)
            VALUES ($1,$2,$3,$4,$5,$6)
            RETURNING id::bigint AS id, title, description, category, favicon, url, status, create_time
            "#,
        )
        .bind(title)
        .bind(normalize_optional_text(payload.description))
        .bind(category)
        .bind(favicon)
        .bind(url)
        .bind(status)
        .fetch_one(&state.db_pool)
        .await
        .map_err(map_unique_violation("Friend URL already exists"))?;
        (data, "create")
    };
    invalidate_friends_cache(&state).await;
    sync_api::record_content_change_best_effort(&state, "friend", event_action, vec![data.id])
        .await;

    Ok(Json(AdminFriendDetailResponse { data }))
}

pub async fn admin_update_friend(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(friend_id): Path<i64>,
    Json(payload): Json<AdminFriendUpdatePayload>,
) -> AppResult<Json<AdminFriendDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let current = sqlx::query_as::<_, FriendItem>(
        "SELECT id::bigint AS id, title, description, category, favicon, url, status, create_time FROM friends WHERE id = $1 LIMIT 1",
    )
    .bind(friend_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to load friend: {error}")))?
    .ok_or_else(|| AppError::not_found("Friend not found"))?;
    let current_url = current.url.clone();

    let next_title = payload
        .title
        .map(|value| normalize_required_text(value, "title"))
        .transpose()?
        .unwrap_or(current.title);
    let next_category = payload
        .category
        .map(|value| normalize_required_text(value, "category"))
        .transpose()?
        .unwrap_or(current.category);
    let next_favicon = if let Some(favicon) = payload.favicon {
        normalize_http_url(&favicon, "favicon", true)?
    } else {
        current.favicon
    };
    let next_url = if let Some(url) = payload.url {
        normalize_http_url(&url, "url", false)?
            .ok_or_else(|| AppError::Unprocessable("url cannot be empty".to_string()))?
    } else {
        current_url.clone()
    };
    let next_status = if let Some(status) = payload.status {
        normalize_friend_status(status)?
    } else {
        current.status
    };
    let next_description = payload
        .description
        .map(|value| normalize_optional_text(Some(value)))
        .unwrap_or(current.description);

    if next_url != current_url {
        let duplicate = sqlx::query_scalar::<_, i64>(
            "SELECT id::bigint FROM friends WHERE url = $1 AND id != $2 LIMIT 1",
        )
        .bind(&next_url)
        .bind(friend_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to check friend URL uniqueness: {error}"))
        })?;
        if duplicate.is_some() {
            return Err(AppError::Conflict("Friend URL already exists".to_string()));
        }
    }

    let data = sqlx::query_as::<_, FriendItem>(
        r#"
        UPDATE friends
        SET
            title = $2,
            description = $3,
            category = $4,
            favicon = $5,
            url = $6,
            status = $7
        WHERE id = $1
        RETURNING id::bigint AS id, title, description, category, favicon, url, status, create_time
        "#,
    )
    .bind(friend_id)
    .bind(next_title)
    .bind(next_description)
    .bind(next_category)
    .bind(next_favicon)
    .bind(next_url)
    .bind(next_status)
    .fetch_one(&state.db_pool)
    .await
    .map_err(map_unique_violation("Friend URL already exists"))?;
    invalidate_friends_cache(&state).await;
    sync_api::record_content_change_best_effort(&state, "friend", "update", vec![data.id]).await;

    Ok(Json(AdminFriendDetailResponse { data }))
}

pub async fn admin_delete_friend(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(friend_id): Path<i64>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let result = sqlx::query("DELETE FROM friends WHERE id = $1")
        .bind(friend_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to delete friend: {error}")))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Friend not found"));
    }
    invalidate_friends_cache(&state).await;
    sync_api::record_content_change_best_effort(&state, "friend", "delete", vec![friend_id]).await;

    Ok(Json(AdminActionResponse {
        success: true,
        message: "Friend deleted".to_string(),
    }))
}

pub async fn admin_list_friend_applies(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Query(query): Query<AdminFriendApplyListQuery>,
) -> AppResult<Json<AdminFriendApplyListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let normalized_status = query
        .status_filter
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    if !normalized_status.is_empty() {
        normalize_friend_apply_status(normalized_status.clone())?;
    }

    let keyword = query.keyword.unwrap_or_default().trim().to_string();
    let like_pattern = format!("%{keyword}%");

    let data = if normalized_status.is_empty() && keyword.is_empty() {
        sqlx::query_as::<_, FriendApplyItem>(
            r#"
            SELECT
                id::bigint AS id,
                site_title,
                site_url,
                site_description,
                site_icon,
                contact,
                status::text AS status,
                ip,
                user_agent,
                create_time,
                update_time
            FROM friend_apply
            ORDER BY
                CASE
                    WHEN status = 'pending' THEN 0
                    WHEN status = 'approved' THEN 1
                    WHEN status = 'rejected' THEN 2
                    WHEN status = 'blocked' THEN 3
                    ELSE 4
                END,
                create_time DESC,
                id DESC
            "#,
        )
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list friend applies: {error}")))?
    } else if normalized_status.is_empty() {
        sqlx::query_as::<_, FriendApplyItem>(
            r#"
            SELECT
                id::bigint AS id,
                site_title,
                site_url,
                site_description,
                site_icon,
                contact,
                status::text AS status,
                ip,
                user_agent,
                create_time,
                update_time
            FROM friend_apply
            WHERE
                site_title LIKE $1
                OR site_url LIKE $1
                OR COALESCE(site_description, '') LIKE $1
                OR COALESCE(contact, '') LIKE $1
                OR status::text LIKE $1
                OR COALESCE(ip, '') LIKE $1
                OR CAST(id AS TEXT) LIKE $1
            ORDER BY
                CASE
                    WHEN status = 'pending' THEN 0
                    WHEN status = 'approved' THEN 1
                    WHEN status = 'rejected' THEN 2
                    WHEN status = 'blocked' THEN 3
                    ELSE 4
                END,
                create_time DESC,
                id DESC
            "#,
        )
        .bind(&like_pattern)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list friend applies: {error}")))?
    } else if keyword.is_empty() {
        sqlx::query_as::<_, FriendApplyItem>(
            r#"
            SELECT
                id::bigint AS id,
                site_title,
                site_url,
                site_description,
                site_icon,
                contact,
                status::text AS status,
                ip,
                user_agent,
                create_time,
                update_time
            FROM friend_apply
            WHERE status::text = $1
            ORDER BY
                CASE
                    WHEN status = 'pending' THEN 0
                    WHEN status = 'approved' THEN 1
                    WHEN status = 'rejected' THEN 2
                    WHEN status = 'blocked' THEN 3
                    ELSE 4
                END,
                create_time DESC,
                id DESC
            "#,
        )
        .bind(&normalized_status)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list friend applies: {error}")))?
    } else {
        sqlx::query_as::<_, FriendApplyItem>(
            r#"
            SELECT
                id::bigint AS id,
                site_title,
                site_url,
                site_description,
                site_icon,
                contact,
                status::text AS status,
                ip,
                user_agent,
                create_time,
                update_time
            FROM friend_apply
            WHERE
                status::text = $1
                AND (
                    site_title LIKE $2
                    OR site_url LIKE $2
                    OR COALESCE(site_description, '') LIKE $2
                    OR COALESCE(contact, '') LIKE $2
                    OR status::text LIKE $2
                    OR COALESCE(ip, '') LIKE $2
                    OR CAST(id AS TEXT) LIKE $2
                )
            ORDER BY
                CASE
                    WHEN status = 'pending' THEN 0
                    WHEN status = 'approved' THEN 1
                    WHEN status = 'rejected' THEN 2
                    WHEN status = 'blocked' THEN 3
                    ELSE 4
                END,
                create_time DESC,
                id DESC
            "#,
        )
        .bind(&normalized_status)
        .bind(&like_pattern)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list friend applies: {error}")))?
    };

    Ok(Json(AdminFriendApplyListResponse { data }))
}

pub async fn admin_update_friend_apply_status(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(apply_id): Path<i64>,
    Json(payload): Json<AdminFriendApplyStatusPayload>,
) -> AppResult<Json<AdminFriendApplyDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let status = normalize_friend_apply_status(payload.status)?;
    let create_friend = payload.create_friend.unwrap_or(false);

    let current = sqlx::query_as::<_, FriendApplyItem>(
        r#"
        SELECT
            id::bigint AS id,
            site_title,
            site_url,
            site_description,
            site_icon,
            contact,
            status::text AS status,
            ip,
            user_agent,
            create_time,
            update_time
        FROM friend_apply
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(apply_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to load friend apply: {error}")))?
    .ok_or_else(|| AppError::not_found("Friend application not found"))?;

    if status == "approved" && create_friend {
        let normalized_url = current.site_url.trim().to_string();
        if !normalized_url.is_empty() {
            let existing = sqlx::query_scalar::<_, i64>(
                "SELECT id::bigint FROM friends WHERE url = $1 LIMIT 1",
            )
            .bind(&normalized_url)
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|error| {
                AppError::internal(format!("Failed to inspect friend URL: {error}"))
            })?;

            if existing.is_none() {
                let category = normalize_optional_text(payload.friend_category)
                    .unwrap_or_else(|| "friend_apply".to_string());
                let title = normalize_optional_text(Some(current.site_title.clone()))
                    .unwrap_or_else(|| normalized_url.clone());
                let description = normalize_optional_text(current.site_description.clone());
                let favicon = normalize_optional_text(current.site_icon.clone());

                sqlx::query(
                    r#"
                    INSERT INTO friends (title, description, category, favicon, url, status)
                    VALUES ($1,$2,$3,$4,$5,'ok')
                    "#,
                )
                .bind(truncate_text(title, 255))
                .bind(description.map(|value| truncate_text(value, 500)))
                .bind(truncate_text(category, 100))
                .bind(favicon.map(|value| truncate_text(value, 500)))
                .bind(truncate_text(normalized_url, 500))
                .execute(&state.db_pool)
                .await
                .map_err(map_unique_violation("Friend URL already exists"))?;
            }
        }
    }

    let data = sqlx::query_as::<_, FriendApplyItem>(
        r#"
        UPDATE friend_apply
        SET status = $2::friend_apply_status, update_time = CURRENT_TIMESTAMP
        WHERE id = $1
        RETURNING
            id::bigint AS id,
            site_title,
            site_url,
            site_description,
            site_icon,
            contact,
            status::text AS status,
            ip,
            user_agent,
            create_time,
            update_time
        "#,
    )
    .bind(apply_id)
    .bind(status)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| {
        AppError::internal(format!("Failed to update friend apply status: {error}"))
    })?;
    invalidate_friends_cache(&state).await;
    sync_api::record_content_change_best_effort(&state, "friend", "update", vec![]).await;

    Ok(Json(AdminFriendApplyDetailResponse { data }))
}

fn normalize_required_text(value: String, field_name: &str) -> AppResult<String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Err(AppError::Unprocessable(format!("{field_name} is required")));
    }
    Ok(normalized)
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let normalized = raw.trim().to_string();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

fn normalize_friend_status(value: String) -> AppResult<String> {
    let normalized = value.trim().to_lowercase();
    match normalized.as_str() {
        "ok" | "missing" | "blocked" => Ok(normalized),
        _ => Err(AppError::Unprocessable("Invalid friend status".to_string())),
    }
}

fn normalize_friend_apply_status(value: String) -> AppResult<String> {
    let normalized = value.trim().to_lowercase();
    match normalized.as_str() {
        "pending" | "approved" | "rejected" | "blocked" => Ok(normalized),
        _ => Err(AppError::Unprocessable(
            "Invalid friend apply status".to_string(),
        )),
    }
}

fn normalize_http_url(
    value: &str,
    field_name: &str,
    allow_empty: bool,
) -> AppResult<Option<String>> {
    let normalized = value.trim();
    if normalized.is_empty() {
        if allow_empty {
            return Ok(None);
        }
        return Err(AppError::Unprocessable(format!("{field_name} is required")));
    }

    let mut candidate = normalized.to_string();
    if !candidate.contains("://") {
        candidate = format!("https://{}", candidate.trim_start_matches('/'));
    }

    let parsed = Url::parse(&candidate).map_err(|_| {
        AppError::Unprocessable("URL must start with http:// or https://".to_string())
    })?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(AppError::Unprocessable(
            "URL must start with http:// or https://".to_string(),
        ));
    }

    Ok(Some(candidate))
}

fn map_unique_violation(message: &'static str) -> impl Fn(sqlx::Error) -> AppError {
    move |error| {
        if is_unique_violation(&error) {
            AppError::Conflict(message.to_string())
        } else {
            AppError::internal(format!("Database operation failed: {error}"))
        }
    }
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(db_error) if db_error.code().as_deref() == Some("23505")
    )
}

fn truncate_text(value: String, max_len: usize) -> String {
    let mut buffer = String::new();
    for ch in value.chars() {
        if buffer.chars().count() >= max_len {
            break;
        }
        buffer.push(ch);
    }
    buffer
}

async fn invalidate_friends_cache(state: &AppState) {
    state.runtime_cache.delete(FRIENDS_CACHE_KEY).await;
}

async fn load_friend_exchange_info(state: &AppState) -> AppResult<FriendExchangeInfoItem> {
    let keys = vec![
        FRIEND_EXCHANGE_SITE_TITLE_KEY,
        FRIEND_EXCHANGE_SITE_URL_KEY,
        FRIEND_EXCHANGE_SITE_ICON_KEY,
        FRIEND_EXCHANGE_SITE_DESCRIPTION_KEY,
        SITE_TITLE_KEY,
        SITE_URL_KEY,
        SITE_FAVICON_KEY,
        SITE_DESCRIPTION_KEY,
    ];

    let rows = sqlx::query(
        "SELECT setting_key, setting_content FROM settings WHERE setting_key = ANY($1)",
    )
    .bind(keys)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to load friend exchange settings: {error}")))?;

    let mut settings = HashMap::<String, String>::new();
    for row in rows {
        let key = row.try_get::<String, _>("setting_key").unwrap_or_default();
        let value = row
            .try_get::<Option<String>, _>("setting_content")
            .ok()
            .flatten()
            .unwrap_or_default()
            .trim()
            .to_string();
        settings.insert(key, value);
    }

    let site_title = pick_setting_value(&settings, FRIEND_EXCHANGE_SITE_TITLE_KEY, SITE_TITLE_KEY)
        .unwrap_or_else(|| "NeHex".to_string());
    let site_url = pick_setting_value(&settings, FRIEND_EXCHANGE_SITE_URL_KEY, SITE_URL_KEY)
        .unwrap_or_default();
    let site_icon = pick_setting_value(&settings, FRIEND_EXCHANGE_SITE_ICON_KEY, SITE_FAVICON_KEY)
        .unwrap_or_default();
    let site_description = pick_setting_value(
        &settings,
        FRIEND_EXCHANGE_SITE_DESCRIPTION_KEY,
        SITE_DESCRIPTION_KEY,
    )
    .unwrap_or_default();

    Ok(FriendExchangeInfoItem {
        site_title,
        site_url,
        site_icon,
        site_description,
    })
}

fn pick_setting_value(
    settings: &HashMap<String, String>,
    custom_key: &str,
    fallback_key: &str,
) -> Option<String> {
    let custom_value = settings
        .get(custom_key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if custom_value.is_some() {
        return custom_value;
    }

    settings
        .get(fallback_key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn upsert_setting<'a>(
    tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    key: &str,
    setting_type: &str,
    setting_content: Option<String>,
    description: &str,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO settings (setting_key, setting_type, setting_content, description)
        VALUES ($1, $2, $3, $4)
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
    .bind(setting_content)
    .bind(Some(description.to_string()))
    .execute(&mut **tx)
    .await
    .map_err(|error| AppError::internal(format!("Failed to upsert setting `{key}`: {error}")))?;
    Ok(())
}
