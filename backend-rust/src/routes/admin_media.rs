use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::{HeaderMap, Method},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    storage_local,
};

use super::admin_auth;

#[derive(Serialize)]
pub struct AdminActionResponse {
    success: bool,
    message: String,
}

#[derive(sqlx::FromRow, Serialize)]
struct MediaFolderItem {
    id: i64,
    name: String,
    image_count: i64,
    create_time: NaiveDateTime,
    update_time: NaiveDateTime,
}

#[derive(sqlx::FromRow, Serialize)]
struct MediaImageItem {
    id: i64,
    folder_id: Option<i64>,
    media_type: String,
    provider: String,
    key: String,
    url: String,
    file_name: Option<String>,
    content_type: Option<String>,
    size_bytes: i64,
    create_time: NaiveDateTime,
}

#[derive(Serialize)]
struct AdminMediaLibraryData {
    folders: Vec<MediaFolderItem>,
    uncategorized: Vec<MediaImageItem>,
}

#[derive(Serialize)]
pub struct AdminMediaLibraryResponse {
    data: AdminMediaLibraryData,
}

#[derive(Serialize)]
pub struct AdminMediaFolderDetailResponse {
    data: MediaFolderItem,
}

#[derive(Serialize)]
pub struct AdminMediaImageDetailResponse {
    data: MediaImageItem,
}

#[derive(Serialize)]
pub struct AdminMediaImageListResponse {
    data: Vec<MediaImageItem>,
}

#[derive(Serialize)]
pub struct AdminStorageUploadResponse {
    data: AdminStorageUploadData,
}

#[derive(Serialize)]
struct AdminStorageUploadData {
    provider: String,
    key: String,
    url: String,
}

#[derive(Deserialize)]
pub struct MediaFolderNamePayload {
    name: String,
}

#[derive(Deserialize)]
pub struct MediaImageMovePayload {
    ids: Vec<i64>,
    folder_id: Option<i64>,
}

pub async fn admin_get_media_library(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> AppResult<Json<AdminMediaLibraryResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let folders = sqlx::query_as::<_, MediaFolderItem>(
        r#"
        SELECT
            f.id::bigint AS id,
            f.name,
            COALESCE(c.image_count, 0)::bigint AS image_count,
            f.create_time,
            f.update_time
        FROM media_folder f
        LEFT JOIN (
            SELECT folder_id, COUNT(*)::bigint AS image_count
            FROM media_image
            WHERE folder_id IS NOT NULL
            GROUP BY folder_id
        ) c ON c.folder_id = f.id
        ORDER BY f.name ASC, f.id ASC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list media folders: {error}")))?;

    let uncategorized = query_media_images(&state, "WHERE folder_id IS NULL").await?;

    Ok(Json(AdminMediaLibraryResponse {
        data: AdminMediaLibraryData {
            folders,
            uncategorized,
        },
    }))
}

pub async fn admin_get_media_folder_images(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(folder_id): Path<i64>,
) -> AppResult<Json<AdminMediaImageListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    if folder_id <= 0 {
        return Err(AppError::not_found("Media folder not found"));
    }

    let exists =
        sqlx::query_scalar::<_, i64>("SELECT id::bigint FROM media_folder WHERE id = $1 LIMIT 1")
            .bind(folder_id)
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|error| {
                AppError::internal(format!("Failed to inspect media folder: {error}"))
            })?;
    if exists.is_none() {
        return Err(AppError::not_found("Media folder not found"));
    }

    let rows = sqlx::query(
        r#"
        SELECT
            id::bigint AS id,
            folder_id::bigint AS folder_id,
            provider,
            storage_key,
            url,
            file_name,
            content_type,
            size_bytes::bigint AS size_bytes,
            create_time
        FROM media_image
        WHERE folder_id = $1
        ORDER BY create_time DESC, id DESC
        "#,
    )
    .bind(folder_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list media images: {error}")))?;

    let images = rows
        .into_iter()
        .map(map_media_image_item)
        .collect::<Vec<_>>();
    Ok(Json(AdminMediaImageListResponse { data: images }))
}

pub async fn admin_create_media_folder(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<MediaFolderNamePayload>,
) -> AppResult<Json<AdminMediaFolderDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let name = normalize_required_text(payload.name, "name")?;

    let data = sqlx::query_as::<_, MediaFolderItem>(
        r#"
        INSERT INTO media_folder (name)
        VALUES ($1)
        RETURNING id::bigint AS id, name, 0::bigint AS image_count, create_time, update_time
        "#,
    )
    .bind(name)
    .fetch_one(&state.db_pool)
    .await
    .map_err(map_unique_violation("Folder name already exists"))?;

    Ok(Json(AdminMediaFolderDetailResponse { data }))
}

pub async fn admin_rename_media_folder(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(folder_id): Path<i64>,
    Json(payload): Json<MediaFolderNamePayload>,
) -> AppResult<Json<AdminMediaFolderDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let name = normalize_required_text(payload.name, "name")?;

    let data = sqlx::query_as::<_, MediaFolderItem>(
        r#"
        UPDATE media_folder
        SET name = $2, update_time = CURRENT_TIMESTAMP
        WHERE id = $1
        RETURNING
            id::bigint AS id,
            name,
            (
                SELECT COUNT(*)::bigint FROM media_image WHERE folder_id = media_folder.id
            ) AS image_count,
            create_time,
            update_time
        "#,
    )
    .bind(folder_id)
    .bind(name)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(map_unique_violation("Folder name already exists"))?
    .ok_or_else(|| AppError::not_found("Media folder not found"))?;

    Ok(Json(AdminMediaFolderDetailResponse { data }))
}

pub async fn admin_delete_media_folder(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(folder_id): Path<i64>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let mut tx =
        state.db_pool.begin().await.map_err(|error| {
            AppError::internal(format!("Failed to begin media folder tx: {error}"))
        })?;

    let exists =
        sqlx::query_scalar::<_, i64>("SELECT id::bigint FROM media_folder WHERE id = $1 LIMIT 1")
            .bind(folder_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| {
                AppError::internal(format!("Failed to inspect media folder: {error}"))
            })?;
    if exists.is_none() {
        return Err(AppError::not_found("Media folder not found"));
    }

    let moved = sqlx::query("UPDATE media_image SET folder_id = NULL WHERE folder_id = $1")
        .bind(folder_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| AppError::internal(format!("Failed to move folder images: {error}")))?
        .rows_affected();

    sqlx::query("DELETE FROM media_folder WHERE id = $1")
        .bind(folder_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| AppError::internal(format!("Failed to delete media folder: {error}")))?;

    tx.commit().await.map_err(|error| {
        AppError::internal(format!("Failed to commit media folder tx: {error}"))
    })?;

    Ok(Json(AdminActionResponse {
        success: true,
        message: format!("Folder deleted and moved {moved} asset(s) to uncategorized"),
    }))
}

pub async fn admin_upload_media_image(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    multipart: Multipart,
) -> AppResult<Json<AdminMediaImageDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let (file_name, content_type, bytes) = read_multipart_file(multipart).await?;

    let upload =
        storage_local::upload_local_file(&state, &file_name, &content_type, &bytes, false).await?;

    let row = sqlx::query(
        r#"
        INSERT INTO media_image (folder_id, provider, storage_key, url, file_name, content_type, size_bytes)
        VALUES (NULL, $1, $2, $3, $4, $5, $6)
        RETURNING
            id::bigint AS id,
            folder_id::bigint AS folder_id,
            provider,
            storage_key,
            url,
            file_name,
            content_type,
            size_bytes::bigint AS size_bytes,
            create_time
        "#,
    )
    .bind(upload.provider.clone())
    .bind(upload.key.clone())
    .bind(upload.url.clone())
    .bind(file_name)
    .bind(content_type)
    .bind(bytes.len() as i64)
    .fetch_one(&state.db_pool)
    .await
    .map_err(map_unique_violation("Image record already exists"))?;

    Ok(Json(AdminMediaImageDetailResponse {
        data: map_media_image_item(row),
    }))
}

pub async fn admin_move_media_images(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<MediaImageMovePayload>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let mut ids = payload
        .ids
        .into_iter()
        .filter(|value| *value > 0)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();

    if ids.is_empty() {
        return Ok(Json(AdminActionResponse {
            success: true,
            message: "Moved 0 asset(s)".to_string(),
        }));
    }

    if let Some(folder_id) = payload.folder_id {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT id::bigint FROM media_folder WHERE id = $1 LIMIT 1",
        )
        .bind(folder_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to inspect media folder: {error}")))?;
        if exists.is_none() {
            return Err(AppError::not_found("Media folder not found"));
        }
    }

    let changed = sqlx::query(
        "UPDATE media_image SET folder_id = $2 WHERE id = ANY($1) AND folder_id IS DISTINCT FROM $2",
    )
    .bind(ids)
    .bind(payload.folder_id)
    .execute(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to move media images: {error}")))?
    .rows_affected();

    Ok(Json(AdminActionResponse {
        success: true,
        message: format!("Moved {changed} asset(s)"),
    }))
}

pub async fn admin_delete_media_image(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(image_id): Path<i64>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let deleted = sqlx::query("DELETE FROM media_image WHERE id = $1")
        .bind(image_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to delete media image: {error}")))?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::not_found("Media image not found"));
    }

    Ok(Json(AdminActionResponse {
        success: true,
        message: "Media image deleted".to_string(),
    }))
}

pub async fn admin_upload_storage_image(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    multipart: Multipart,
) -> AppResult<Json<AdminStorageUploadResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let (file_name, content_type, bytes) = read_multipart_file(multipart).await?;

    let upload =
        storage_local::upload_local_file(&state, &file_name, &content_type, &bytes, true).await?;

    let row = sqlx::query(
        r#"
        INSERT INTO media_image (folder_id, provider, storage_key, url, file_name, content_type, size_bytes)
        VALUES (NULL, $1, $2, $3, $4, $5, $6)
        RETURNING provider, storage_key, url
        "#,
    )
    .bind(upload.provider.clone())
    .bind(upload.key.clone())
    .bind(upload.url.clone())
    .bind(file_name)
    .bind(content_type)
    .bind(bytes.len() as i64)
    .fetch_one(&state.db_pool)
    .await
    .map_err(map_unique_violation("Image record already exists"))?;

    Ok(Json(AdminStorageUploadResponse {
        data: AdminStorageUploadData {
            provider: row
                .try_get::<String, _>("provider")
                .unwrap_or(upload.provider),
            key: row
                .try_get::<String, _>("storage_key")
                .unwrap_or(upload.key),
            url: row.try_get::<String, _>("url").unwrap_or(upload.url),
        },
    }))
}

async fn read_multipart_file(mut multipart: Multipart) -> AppResult<(String, String, Vec<u8>)> {
    loop {
        let field = multipart.next_field().await.map_err(|error| {
            AppError::Unprocessable(format!("Invalid multipart payload: {error}"))
        })?;
        let Some(field) = field else {
            break;
        };

        if field.name() != Some("file") {
            continue;
        }

        let file_name = field
            .file_name()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "upload.bin".to_string());
        let content_type = field
            .content_type()
            .map(|value| value.trim().to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let bytes = field
            .bytes()
            .await
            .map_err(|error| AppError::Unprocessable(format!("Invalid upload file: {error}")))?;

        return Ok((file_name, content_type, bytes.to_vec()));
    }

    Err(AppError::Unprocessable(
        "Missing upload file field `file`".to_string(),
    ))
}

async fn query_media_images(
    state: &AppState,
    where_clause: &str,
) -> AppResult<Vec<MediaImageItem>> {
    let sql = format!(
        r#"
        SELECT
            id::bigint AS id,
            folder_id::bigint AS folder_id,
            provider,
            storage_key,
            url,
            file_name,
            content_type,
            size_bytes::bigint AS size_bytes,
            create_time
        FROM media_image
        {where_clause}
        ORDER BY create_time DESC, id DESC
        "#
    );
    let rows = sqlx::query(&sql)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list media images: {error}")))?;

    Ok(rows.into_iter().map(map_media_image_item).collect())
}

fn map_media_image_item(row: sqlx::postgres::PgRow) -> MediaImageItem {
    let file_name = row.try_get::<Option<String>, _>("file_name").ok().flatten();
    let content_type = row
        .try_get::<Option<String>, _>("content_type")
        .ok()
        .flatten();
    let provider = row
        .try_get::<String, _>("provider")
        .unwrap_or_else(|_| "local".to_string());
    let key = row.try_get::<String, _>("storage_key").unwrap_or_default();
    let raw_url = row.try_get::<String, _>("url").unwrap_or_default();
    let resolved_url = if provider == "hi168_s3" && !key.trim().is_empty() {
        storage_local::build_object_proxy_path(&key)
    } else {
        raw_url
    };

    MediaImageItem {
        id: row.try_get::<i64, _>("id").unwrap_or_default(),
        folder_id: row.try_get::<Option<i64>, _>("folder_id").ok().flatten(),
        media_type: detect_media_type(content_type.as_deref(), file_name.as_deref()),
        provider,
        key,
        url: resolved_url,
        file_name,
        content_type,
        size_bytes: row
            .try_get::<i64, _>("size_bytes")
            .unwrap_or_default()
            .max(0),
        create_time: row
            .try_get::<NaiveDateTime, _>("create_time")
            .unwrap_or_else(|_| chrono::Utc::now().naive_utc()),
    }
}

fn detect_media_type(content_type: Option<&str>, file_name: Option<&str>) -> String {
    let content_type = content_type.unwrap_or_default().trim().to_lowercase();
    if content_type.starts_with("image/") {
        return "image".to_string();
    }
    if content_type.starts_with("video/") {
        return "video".to_string();
    }
    if content_type.starts_with("audio/") {
        return "audio".to_string();
    }

    let lower_file = file_name.unwrap_or_default().trim().to_lowercase();
    if lower_file.ends_with(".jpg")
        || lower_file.ends_with(".jpeg")
        || lower_file.ends_with(".png")
        || lower_file.ends_with(".webp")
        || lower_file.ends_with(".gif")
        || lower_file.ends_with(".bmp")
        || lower_file.ends_with(".svg")
        || lower_file.ends_with(".avif")
    {
        return "image".to_string();
    }
    if lower_file.ends_with(".mp4")
        || lower_file.ends_with(".webm")
        || lower_file.ends_with(".mov")
        || lower_file.ends_with(".mkv")
        || lower_file.ends_with(".avi")
        || lower_file.ends_with(".ogv")
    {
        return "video".to_string();
    }
    if lower_file.ends_with(".mp3")
        || lower_file.ends_with(".wav")
        || lower_file.ends_with(".ogg")
        || lower_file.ends_with(".flac")
        || lower_file.ends_with(".aac")
        || lower_file.ends_with(".m4a")
    {
        return "audio".to_string();
    }

    "file".to_string()
}

fn normalize_required_text(value: String, field_name: &str) -> AppResult<String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Err(AppError::Unprocessable(format!("{field_name} is required")));
    }
    Ok(normalized)
}

fn map_unique_violation(message: &'static str) -> impl Fn(sqlx::Error) -> AppError {
    move |error| {
        if matches!(
            error,
            sqlx::Error::Database(ref db_error) if db_error.code().as_deref() == Some("23505")
        ) {
            AppError::Conflict(message.to_string())
        } else {
            AppError::internal(format!("Database operation failed: {error}"))
        }
    }
}
