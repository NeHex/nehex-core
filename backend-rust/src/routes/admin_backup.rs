use std::{
    collections::{HashMap, HashSet},
    io,
    path::{Component, Path as StdPath, PathBuf},
    time::SystemTime,
};

use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::Response,
};
use chrono::{DateTime, Local, NaiveDateTime, Utc};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use once_cell::sync::Lazy;
use rand::RngCore;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tar::Archive;
use tokio_util::io::ReaderStream;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

use super::admin_auth;

const BACKUP_ROOT_DIR: &str = "backups";
const BACKUP_TMP_DIR: &str = ".tmp";
const MAX_BACKUP_UPLOAD_BYTES: usize = 512 * 1024 * 1024;
const MAX_BACKUP_EXTRACT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const DEFAULT_LOCAL_STORAGE_ROOT: &str = "storage";

static BACKUP_FILENAME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^nehex-backup-\d{8}-\d{6}(?:-[a-z0-9]{6})?\.tar\.gz$")
        .expect("backup filename regex must compile")
});

const APP_TABLE_ORDER: &[&str] = &[
    "settings",
    "article",
    "singlepage",
    "daily",
    "project",
    "album",
    "friends",
    "friend_apply",
    "comment",
    "media_folder",
    "media_image",
    "kuma_movie",
    "mail_log",
];

#[derive(Serialize)]
pub struct AdminActionResponse {
    success: bool,
    message: String,
}

#[derive(Serialize)]
pub struct AdminBackupItem {
    filename: String,
    size_bytes: i64,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct AdminBackupListResponse {
    data: Vec<AdminBackupItem>,
}

#[derive(Serialize)]
pub struct AdminBackupDetailResponse {
    data: AdminBackupItem,
}

#[derive(Deserialize)]
pub struct AdminBackupRestoreRequest {
    #[serde(default)]
    confirm_overwrite: bool,
}

#[derive(Serialize)]
struct DatabaseSnapshotPayload {
    created_at: String,
    dialect: String,
    tables: Vec<DatabaseTableSnapshot>,
}

#[derive(Serialize, Deserialize)]
struct DatabaseTableSnapshot {
    name: String,
    #[serde(default)]
    columns: Vec<String>,
    #[serde(default)]
    rows: Value,
}

#[derive(Serialize)]
struct SnapshotMetaPayload {
    created_at: String,
    project_root: String,
    storage: SnapshotStorageMeta,
}

#[derive(Serialize)]
struct SnapshotStorageMeta {
    local_storage_root: String,
    copied: bool,
    file_count: u64,
}

#[derive(Deserialize)]
struct DatabaseSnapshotRestorePayload {
    #[serde(default)]
    tables: Vec<DatabaseTableSnapshot>,
}

pub async fn admin_list_backups(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> AppResult<Json<AdminBackupListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let data = list_admin_backups(&state).await?;
    Ok(Json(AdminBackupListResponse { data }))
}

pub async fn admin_create_backup(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> AppResult<Json<AdminBackupDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let data = create_admin_backup(&state).await?;
    Ok(Json(AdminBackupDetailResponse { data }))
}

pub async fn admin_download_backup(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> AppResult<Response> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let backup_file = get_admin_backup_file_path(&state, &filename).await?;
    let backup_name = backup_file
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("backup.tar.gz")
        .to_string();

    let file = tokio::fs::File::open(&backup_file)
        .await
        .map_err(|error| AppError::internal(format!("Failed to open backup file: {error}")))?;
    let stream = ReaderStream::new(file);

    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/gzip"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{backup_name}\""))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );

    Ok(response)
}

pub async fn admin_delete_backup(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let deleted = delete_admin_backup(&state, &filename).await?;
    Ok(Json(AdminActionResponse {
        success: true,
        message: format!("备份已删除：{}", deleted.filename),
    }))
}

pub async fn admin_restore_backup(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(filename): Path<String>,
    Json(payload): Json<AdminBackupRestoreRequest>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    restore_admin_backup(&state, &filename, payload.confirm_overwrite).await?;
    Ok(Json(AdminActionResponse {
        success: true,
        message: "备份恢复完成，现有数据已覆盖".to_string(),
    }))
}

pub async fn admin_upload_and_restore_backup(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    multipart: Multipart,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let upload = read_upload_restore_payload(multipart).await?;
    let item = save_uploaded_backup(&state, &upload.file_name, &upload.file_bytes).await?;
    restore_admin_backup(&state, &item.filename, upload.confirm_overwrite).await?;

    Ok(Json(AdminActionResponse {
        success: true,
        message: format!("上传并恢复成功：{}", item.filename),
    }))
}

#[derive(Default)]
struct UploadRestorePayload {
    file_name: String,
    file_bytes: Vec<u8>,
    confirm_overwrite: bool,
    has_file_field: bool,
}

async fn read_upload_restore_payload(mut multipart: Multipart) -> AppResult<UploadRestorePayload> {
    let mut payload = UploadRestorePayload::default();

    loop {
        let field = multipart.next_field().await.map_err(|error| {
            AppError::Unprocessable(format!("Invalid multipart payload: {error}"))
        })?;
        let Some(field) = field else {
            break;
        };

        let name = field.name().unwrap_or_default().trim().to_string();
        if name == "confirm_overwrite" {
            let text = field.text().await.map_err(|error| {
                AppError::Unprocessable(format!("Invalid confirm_overwrite field: {error}"))
            })?;
            payload.confirm_overwrite = parse_boolean_text(&text);
            continue;
        }

        if name != "file" {
            continue;
        }
        payload.has_file_field = true;

        payload.file_name = field
            .file_name()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "backup.tar.gz".to_string());

        let bytes = field
            .bytes()
            .await
            .map_err(|error| AppError::Unprocessable(format!("Invalid upload file: {error}")))?;
        payload.file_bytes = bytes.to_vec();
    }

    if !payload.has_file_field {
        return Err(AppError::Unprocessable(
            "Missing upload file field `file`".to_string(),
        ));
    }
    if payload.file_bytes.is_empty() {
        return Err(AppError::Unprocessable("上传备份文件为空".to_string()));
    }

    Ok(payload)
}

async fn list_admin_backups(state: &AppState) -> AppResult<Vec<AdminBackupItem>> {
    ensure_backup_directories(state).await?;

    let root = backup_root(state);
    let mut reader = tokio::fs::read_dir(&root)
        .await
        .map_err(|error| AppError::internal(format!("Failed to read backup root: {error}")))?;

    let mut items = Vec::new();
    loop {
        let entry = reader
            .next_entry()
            .await
            .map_err(|error| AppError::internal(format!("Failed to read backup entry: {error}")))?;
        let Some(entry) = entry else {
            break;
        };

        let file_type = entry.file_type().await.map_err(|error| {
            AppError::internal(format!("Failed to inspect backup file type: {error}"))
        })?;
        if !file_type.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        if !is_valid_backup_filename(&file_name) {
            continue;
        }

        let item = to_backup_item(entry.path()).await?;
        items.push(item);
    }

    items.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then(right.filename.cmp(&left.filename))
    });

    Ok(items)
}

async fn create_admin_backup(state: &AppState) -> AppResult<AdminBackupItem> {
    ensure_backup_directories(state).await?;

    let filename = build_backup_filename();
    let backup_path = backup_root(state).join(&filename);
    let tmp_root = backup_tmp_root(state).join(format!("backup-{}", random_hex(8)));
    let snapshot_root = tmp_root.join("snapshot");

    tokio::fs::create_dir_all(&snapshot_root)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to create backup temp root: {error}"))
        })?;

    let result = async {
        snapshot_database(state, &snapshot_root).await?;
        let storage_meta = snapshot_storage(state, &snapshot_root).await?;
        write_snapshot_meta(state, &snapshot_root, &storage_meta).await?;
        let archive_source = snapshot_root.clone();
        let archive_target = backup_path.clone();
        tokio::task::spawn_blocking(move || {
            create_backup_archive(&archive_source, &archive_target)
        })
        .await
        .map_err(|error| {
            AppError::internal(format!("Backup archive task join failed: {error}"))
        })??;
        Ok::<(), AppError>(())
    }
    .await;

    let _ = tokio::fs::remove_dir_all(&tmp_root).await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(&backup_path).await;
    }
    result?;

    to_backup_item(backup_path).await
}

async fn save_uploaded_backup(
    state: &AppState,
    file_name: &str,
    content: &[u8],
) -> AppResult<AdminBackupItem> {
    validate_uploaded_backup_file_name(file_name)?;
    ensure_backup_directories(state).await?;

    if content.is_empty() {
        return Err(AppError::Unprocessable("上传备份文件为空".to_string()));
    }
    if content.len() > MAX_BACKUP_UPLOAD_BYTES {
        return Err(AppError::Unprocessable(
            "备份文件不能超过 512MB".to_string(),
        ));
    }

    let filename = build_backup_filename();
    let path = backup_root(state).join(filename);
    tokio::fs::write(&path, content)
        .await
        .map_err(|error| AppError::internal(format!("Failed to save uploaded backup: {error}")))?;

    to_backup_item(path).await
}

async fn delete_admin_backup(state: &AppState, filename: &str) -> AppResult<AdminBackupItem> {
    let target = get_admin_backup_file_path(state, filename).await?;
    let item = to_backup_item(target.clone()).await?;

    tokio::fs::remove_file(&target)
        .await
        .map_err(|error| AppError::internal(format!("Failed to delete backup file: {error}")))?;

    Ok(item)
}

async fn restore_admin_backup(
    state: &AppState,
    filename: &str,
    confirm_overwrite: bool,
) -> AppResult<()> {
    if !confirm_overwrite {
        return Err(AppError::Unprocessable(
            "恢复操作需要确认覆盖现有数据".to_string(),
        ));
    }

    let backup_path = get_admin_backup_file_path(state, filename).await?;
    let tmp_root = backup_tmp_root(state).join(format!("restore-{}", random_hex(8)));

    tokio::fs::create_dir_all(&tmp_root)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to create restore temp root: {error}"))
        })?;

    let result = async {
        let archive_source = backup_path.clone();
        let archive_target = tmp_root.clone();
        tokio::task::spawn_blocking(move || {
            extract_backup_archive_safely(&archive_source, &archive_target)
        })
        .await
        .map_err(|error| {
            AppError::internal(format!("Backup extract task join failed: {error}"))
        })??;

        let snapshot_root = tmp_root.join("snapshot");
        let snapshot_meta = tokio::fs::metadata(&snapshot_root)
            .await
            .map_err(|_| AppError::Unprocessable("备份包缺少 snapshot 目录".to_string()))?;
        if !snapshot_meta.is_dir() {
            return Err(AppError::Unprocessable(
                "备份包缺少 snapshot 目录".to_string(),
            ));
        }

        restore_database_snapshot(state, &snapshot_root).await?;
        restore_storage_snapshot(state, &snapshot_root).await?;
        state.runtime_cache.clear().await;

        Ok::<(), AppError>(())
    }
    .await;

    let _ = tokio::fs::remove_dir_all(&tmp_root).await;
    result
}

async fn get_admin_backup_file_path(state: &AppState, filename: &str) -> AppResult<PathBuf> {
    let normalized = normalize_backup_filename(filename)?;
    let path = backup_root(state).join(normalized);

    let meta = tokio::fs::metadata(&path).await;
    match meta {
        Ok(metadata) if metadata.is_file() => Ok(path),
        Ok(_) => Err(AppError::not_found("备份文件不存在")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(AppError::not_found("备份文件不存在"))
        }
        Err(error) => Err(AppError::internal(format!(
            "Failed to inspect backup file: {error}"
        ))),
    }
}

async fn to_backup_item(path: PathBuf) -> AppResult<AdminBackupItem> {
    let metadata = tokio::fs::metadata(&path).await.map_err(|error| {
        AppError::internal(format!("Failed to inspect backup file metadata: {error}"))
    })?;

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();

    let modified_time = metadata.modified().unwrap_or_else(|_| SystemTime::now());
    let created_time = metadata.created().unwrap_or(modified_time);

    Ok(AdminBackupItem {
        filename: file_name,
        size_bytes: metadata.len().min(i64::MAX as u64) as i64,
        created_at: system_time_to_naive_datetime(created_time),
        updated_at: system_time_to_naive_datetime(modified_time),
    })
}

async fn snapshot_database(state: &AppState, snapshot_root: &StdPath) -> AppResult<()> {
    let tables = list_existing_app_tables(state).await?;

    let mut payload_tables = Vec::with_capacity(tables.len());
    for table_name in tables {
        let columns = sqlx::query_scalar::<_, String>(
            r#"
            SELECT column_name
            FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = $1
            ORDER BY ordinal_position
            "#,
        )
        .bind(&table_name)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| {
            AppError::internal(format!(
                "Failed to read columns for table `{table_name}`: {error}"
            ))
        })?;

        let quoted_table = quote_ident(&table_name);
        let rows_query = format!(
            "SELECT COALESCE(json_agg(to_jsonb(t)), '[]'::json)::text FROM (SELECT * FROM {quoted_table}) t"
        );
        let rows_text = sqlx::query_scalar::<_, Option<String>>(&rows_query)
            .fetch_one(&state.db_pool)
            .await
            .map_err(|error| {
                AppError::internal(format!(
                    "Failed to snapshot rows for table `{table_name}`: {error}"
                ))
            })?
            .unwrap_or_else(|| "[]".to_string());

        let rows = serde_json::from_str::<Value>(&rows_text).map_err(|error| {
            AppError::internal(format!(
                "Failed to decode table snapshot for `{table_name}`: {error}"
            ))
        })?;

        payload_tables.push(DatabaseTableSnapshot {
            name: table_name.to_string(),
            columns,
            rows,
        });
    }

    let payload = DatabaseSnapshotPayload {
        created_at: Utc::now().to_rfc3339(),
        dialect: "postgresql".to_string(),
        tables: payload_tables,
    };

    let db_dir = snapshot_root.join("database");
    tokio::fs::create_dir_all(&db_dir).await.map_err(|error| {
        AppError::internal(format!("Failed to create database snapshot dir: {error}"))
    })?;

    let db_file = db_dir.join("data.json");
    let content = serde_json::to_string_pretty(&payload).map_err(|error| {
        AppError::internal(format!("Failed to encode database snapshot: {error}"))
    })?;

    tokio::fs::write(db_file, content).await.map_err(|error| {
        AppError::internal(format!("Failed to write database snapshot: {error}"))
    })?;

    Ok(())
}

async fn restore_database_snapshot(state: &AppState, snapshot_root: &StdPath) -> AppResult<()> {
    let db_file = snapshot_root.join("database").join("data.json");
    let raw = tokio::fs::read_to_string(&db_file)
        .await
        .map_err(|_| AppError::Unprocessable("备份包中缺少数据库快照".to_string()))?;

    let payload: DatabaseSnapshotRestorePayload = serde_json::from_str(&raw)
        .map_err(|error| AppError::Unprocessable(format!("数据库快照格式不正确: {error}")))?;

    let existing_tables = list_existing_app_tables(state).await?;
    if existing_tables.is_empty() {
        return Ok(());
    }

    let mut table_rows_map: HashMap<String, Value> = HashMap::new();
    for table in payload.tables {
        if !existing_tables.iter().any(|name| *name == table.name) {
            continue;
        }
        if !table.rows.is_array() {
            continue;
        }
        table_rows_map.insert(table.name, table.rows);
    }

    let mut tx = state.db_pool.begin().await.map_err(|error| {
        AppError::internal(format!("Failed to start restore transaction: {error}"))
    })?;

    let truncate_sql = format!(
        "TRUNCATE TABLE {} RESTART IDENTITY CASCADE",
        existing_tables
            .iter()
            .map(|name| quote_ident(name))
            .collect::<Vec<_>>()
            .join(", ")
    );

    sqlx::query(&truncate_sql)
        .execute(&mut *tx)
        .await
        .map_err(|error| AppError::Unprocessable(format!("数据库清空失败: {error}")))?;

    for table_name in &existing_tables {
        let rows = table_rows_map.remove(table_name);
        let Some(rows) = rows else {
            continue;
        };

        let Some(items) = rows.as_array() else {
            continue;
        };
        if items.is_empty() {
            continue;
        }

        let rows_json = serde_json::to_string(items)
            .map_err(|error| AppError::Unprocessable(format!("数据库恢复数据编码失败: {error}")))?;

        let quoted_table = quote_ident(table_name);
        let insert_sql = format!(
            "INSERT INTO {quoted_table} SELECT * FROM json_populate_recordset(NULL::{quoted_table}, $1::json)"
        );

        sqlx::query(&insert_sql)
            .bind(rows_json)
            .execute(&mut *tx)
            .await
            .map_err(|error| {
                AppError::Unprocessable(format!("恢复表 `{table_name}` 失败: {error}"))
            })?;
    }

    tx.commit().await.map_err(|error| {
        AppError::internal(format!("Failed to commit restore transaction: {error}"))
    })?;

    Ok(())
}

async fn snapshot_storage(
    state: &AppState,
    snapshot_root: &StdPath,
) -> AppResult<SnapshotStorageMeta> {
    let local_storage_root = resolve_local_storage_root(state).await?;
    let storage_target = snapshot_root.join("storage");

    let source_meta = tokio::fs::metadata(&local_storage_root).await;
    let mut copied = false;
    let mut file_count = 0_u64;

    if let Ok(source_meta) = source_meta {
        if source_meta.is_dir() {
            let src = local_storage_root.clone();
            let dst = storage_target.clone();
            file_count = tokio::task::spawn_blocking(move || copy_dir_recursive(&src, &dst))
                .await
                .map_err(|error| {
                    AppError::internal(format!("Backup storage task join failed: {error}"))
                })??;
            copied = true;
        }
    }

    Ok(SnapshotStorageMeta {
        local_storage_root: local_storage_root.to_string_lossy().to_string(),
        copied,
        file_count,
    })
}

async fn write_snapshot_meta(
    state: &AppState,
    snapshot_root: &StdPath,
    storage_meta: &SnapshotStorageMeta,
) -> AppResult<()> {
    let payload = SnapshotMetaPayload {
        created_at: Utc::now().to_rfc3339(),
        project_root: state.paths.project_root.to_string_lossy().to_string(),
        storage: SnapshotStorageMeta {
            local_storage_root: storage_meta.local_storage_root.clone(),
            copied: storage_meta.copied,
            file_count: storage_meta.file_count,
        },
    };

    let content = serde_json::to_string_pretty(&payload)
        .map_err(|error| AppError::internal(format!("Failed to encode snapshot meta: {error}")))?;

    let meta_path = snapshot_root.join("meta.json");
    tokio::fs::write(meta_path, content)
        .await
        .map_err(|error| AppError::internal(format!("Failed to write snapshot meta: {error}")))?;

    Ok(())
}

async fn restore_storage_snapshot(state: &AppState, snapshot_root: &StdPath) -> AppResult<()> {
    let storage_src = snapshot_root.join("storage");
    let storage_meta = tokio::fs::metadata(&storage_src).await;
    match storage_meta {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(AppError::internal(format!(
                "Failed to inspect snapshot storage directory: {error}"
            )));
        }
    }

    let target_root = resolve_local_storage_root(state).await?;
    if is_unsafe_storage_target(&target_root, state) {
        return Err(AppError::Unprocessable(
            "存储目录配置过于宽泛，拒绝覆盖".to_string(),
        ));
    }

    let clear_target = target_root.clone();
    tokio::task::spawn_blocking(move || clear_directory(&clear_target))
        .await
        .map_err(|error| {
            AppError::internal(format!("Storage clear task join failed: {error}"))
        })??;

    let src = storage_src.clone();
    let dst = target_root.clone();
    tokio::task::spawn_blocking(move || copy_dir_recursive(&src, &dst))
        .await
        .map_err(|error| {
            AppError::internal(format!("Storage restore task join failed: {error}"))
        })??;

    Ok(())
}

fn create_backup_archive(snapshot_root: &StdPath, backup_path: &StdPath) -> AppResult<()> {
    let file = std::fs::File::create(backup_path)
        .map_err(|error| AppError::internal(format!("Failed to create backup archive: {error}")))?;

    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);

    builder
        .append_dir_all("snapshot", snapshot_root)
        .map_err(|error| AppError::internal(format!("Failed to append snapshot files: {error}")))?;

    let encoder = builder
        .into_inner()
        .map_err(|error| AppError::internal(format!("Failed to finalize tar builder: {error}")))?;

    encoder
        .finish()
        .map_err(|error| AppError::internal(format!("Failed to finalize gzip archive: {error}")))?;

    Ok(())
}

fn extract_backup_archive_safely(archive_path: &StdPath, dest_root: &StdPath) -> AppResult<()> {
    let file = std::fs::File::open(archive_path).map_err(|error| {
        AppError::Unprocessable(format!("备份文件格式错误，必须是 tar.gz 压缩包: {error}"))
    })?;

    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    let mut total_extract_bytes: u64 = 0;
    let entries = archive.entries().map_err(|error| {
        AppError::Unprocessable(format!("备份文件格式错误，必须是 tar.gz 压缩包: {error}"))
    })?;

    for item in entries {
        let mut entry = item.map_err(|error| {
            AppError::Unprocessable(format!("备份文件格式错误，必须是 tar.gz 压缩包: {error}"))
        })?;

        let raw_path = entry
            .path()
            .map_err(|error| AppError::Unprocessable(format!("备份压缩包路径无效: {error}")))?;
        let relative = sanitize_archive_path(&raw_path)?;

        let target = dest_root.join(&relative);
        if !is_path_within_root(&target, dest_root) {
            return Err(AppError::Unprocessable(
                "备份压缩包包含非法路径".to_string(),
            ));
        }

        let entry_type = entry.header().entry_type();
        if entry_type.is_dir() {
            std::fs::create_dir_all(&target)
                .map_err(|error| AppError::Unprocessable(format!("创建目录失败: {error}")))?;
            continue;
        }

        if !entry_type.is_file() {
            return Err(AppError::Unprocessable(
                "备份压缩包包含不安全条目".to_string(),
            ));
        }

        let file_size = entry
            .header()
            .size()
            .map_err(|error| AppError::Unprocessable(format!("备份压缩包条目尺寸无效: {error}")))?;
        total_extract_bytes = total_extract_bytes.saturating_add(file_size);
        if total_extract_bytes > MAX_BACKUP_EXTRACT_BYTES {
            return Err(AppError::Unprocessable(
                "备份文件解压后体积超过限制".to_string(),
            ));
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| AppError::Unprocessable(format!("创建目录失败: {error}")))?;
        }

        let mut output = std::fs::File::create(&target)
            .map_err(|error| AppError::Unprocessable(format!("创建文件失败: {error}")))?;
        io::copy(&mut entry, &mut output)
            .map_err(|error| AppError::Unprocessable(format!("写入文件失败: {error}")))?;
    }

    Ok(())
}

async fn resolve_local_storage_root(state: &AppState) -> AppResult<PathBuf> {
    let local_root = sqlx::query_scalar::<_, Option<String>>(
        "SELECT setting_content FROM settings WHERE setting_key = 'object_storage_local_root' LIMIT 1",
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to read storage root setting: {error}")))?
    .flatten()
    .unwrap_or_else(|| DEFAULT_LOCAL_STORAGE_ROOT.to_string());

    let candidate = PathBuf::from(local_root.trim());
    if candidate.is_absolute() {
        Ok(candidate)
    } else {
        Ok(state.paths.project_root.join(candidate))
    }
}

async fn list_existing_app_tables(state: &AppState) -> AppResult<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name = ANY($1)",
    )
    .bind(APP_TABLE_ORDER.to_vec())
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list app tables: {error}")))?;

    let existing = rows.into_iter().collect::<HashSet<_>>();
    let ordered = APP_TABLE_ORDER
        .iter()
        .filter(|name| existing.contains(**name))
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();

    Ok(ordered)
}

fn copy_dir_recursive(source: &StdPath, target: &StdPath) -> AppResult<u64> {
    if !source.exists() {
        return Ok(0);
    }

    std::fs::create_dir_all(target)
        .map_err(|error| AppError::internal(format!("Failed to create directory: {error}")))?;

    let mut file_count = 0_u64;
    for entry in std::fs::read_dir(source)
        .map_err(|error| AppError::internal(format!("Failed to read directory: {error}")))?
    {
        let entry = entry.map_err(|error| {
            AppError::internal(format!("Failed to read directory entry: {error}"))
        })?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| AppError::internal(format!("Failed to inspect file type: {error}")))?;

        if file_type.is_symlink() {
            return Err(AppError::Unprocessable(
                "存储目录包含符号链接，不允许备份".to_string(),
            ));
        }

        if file_type.is_dir() {
            file_count = file_count.saturating_add(copy_dir_recursive(&source_path, &target_path)?);
            continue;
        }

        if file_type.is_file() {
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    AppError::internal(format!("Failed to create target parent: {error}"))
                })?;
            }

            std::fs::copy(&source_path, &target_path)
                .map_err(|error| AppError::internal(format!("Failed to copy file: {error}")))?;
            file_count = file_count.saturating_add(1);
        }
    }

    Ok(file_count)
}

fn clear_directory(target: &StdPath) -> AppResult<()> {
    if !target.exists() {
        std::fs::create_dir_all(target).map_err(|error| {
            AppError::internal(format!("Failed to create storage directory: {error}"))
        })?;
        return Ok(());
    }

    for entry in std::fs::read_dir(target).map_err(|error| {
        AppError::internal(format!("Failed to list directory for clear: {error}"))
    })? {
        let entry = entry
            .map_err(|error| AppError::internal(format!("Failed to read clear entry: {error}")))?;
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path).map_err(|error| {
                AppError::internal(format!("Failed to remove directory: {error}"))
            })?;
        } else {
            std::fs::remove_file(&path)
                .map_err(|error| AppError::internal(format!("Failed to remove file: {error}")))?;
        }
    }

    Ok(())
}

fn is_unsafe_storage_target(target_root: &StdPath, state: &AppState) -> bool {
    let normalized = normalize_path_for_compare(target_root);
    let project_root = normalize_path_for_compare(&state.paths.project_root);

    if normalized == PathBuf::from("/") || normalized == project_root {
        return true;
    }

    if let Some(home) = std::env::var_os("HOME") {
        let home_path = normalize_path_for_compare(&PathBuf::from(home));
        if normalized == home_path {
            return true;
        }
    }

    false
}

fn normalize_path_for_compare(path: &StdPath) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn sanitize_archive_path(path: &StdPath) -> AppResult<PathBuf> {
    let mut safe = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) | Component::ParentDir => {
                return Err(AppError::Unprocessable(
                    "备份压缩包包含非法路径".to_string(),
                ));
            }
        }
    }

    if safe.as_os_str().is_empty() {
        return Err(AppError::Unprocessable(
            "备份压缩包包含非法路径".to_string(),
        ));
    }

    Ok(safe)
}

fn is_path_within_root(target: &StdPath, root: &StdPath) -> bool {
    target.starts_with(root)
}

async fn ensure_backup_directories(state: &AppState) -> AppResult<()> {
    let root = backup_root(state);
    let tmp = backup_tmp_root(state);

    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|error| AppError::internal(format!("Failed to create backup root: {error}")))?;
    tokio::fs::create_dir_all(&tmp).await.map_err(|error| {
        AppError::internal(format!("Failed to create backup temp root: {error}"))
    })?;

    Ok(())
}

fn backup_root(state: &AppState) -> PathBuf {
    state.paths.project_root.join(BACKUP_ROOT_DIR)
}

fn backup_tmp_root(state: &AppState) -> PathBuf {
    backup_root(state).join(BACKUP_TMP_DIR)
}

fn parse_boolean_text(value: &str) -> bool {
    matches!(
        value.trim().to_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn build_backup_filename() -> String {
    let now = Local::now().format("%Y%m%d-%H%M%S").to_string();
    format!("nehex-backup-{now}-{}.tar.gz", random_hex(3))
}

fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0_u8; bytes.max(1)];
    rand::thread_rng().fill_bytes(&mut buffer);
    hex::encode(buffer)
}

fn is_valid_backup_filename(value: &str) -> bool {
    BACKUP_FILENAME_RE.is_match(value.trim())
}

fn normalize_backup_filename(filename: &str) -> AppResult<String> {
    let normalized = filename.trim().to_string();
    if !is_valid_backup_filename(&normalized) {
        return Err(AppError::Unprocessable("无效的备份文件名".to_string()));
    }
    Ok(normalized)
}

fn validate_uploaded_backup_file_name(file_name: &str) -> AppResult<()> {
    if !file_name.trim().to_lowercase().ends_with(".tar.gz") {
        return Err(AppError::Unprocessable(
            "仅支持 .tar.gz 备份文件".to_string(),
        ));
    }
    Ok(())
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn system_time_to_naive_datetime(value: SystemTime) -> NaiveDateTime {
    let date_time: DateTime<Utc> = value.into();
    date_time.naive_utc()
}
