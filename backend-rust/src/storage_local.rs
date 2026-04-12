use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};

use aws_credential_types::Credentials;
use aws_sdk_s3::{Client as S3Client, config::Region, primitives::ByteStream};
use chrono::{Datelike, Timelike, Utc};
use rand::RngCore;
use sqlx::Row;
use url::Url;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

const DEFAULT_LOCAL_ROOT: &str = "storage";
const DEFAULT_LOCAL_PATH_RULE: &str = "/{year}-{month}/{day}/{random_name}.{file_type}";
const DEFAULT_LOCAL_URL_PREFIX: &str = "/storage";
const MAX_IMAGE_SIZE_BYTES: usize = 20 * 1024 * 1024;
const MAX_MEDIA_SIZE_BYTES: usize = 200 * 1024 * 1024;

const STORAGE_PROVIDER_KEY: &str = "object_storage_provider";
const STORAGE_ENABLED_KEY: &str = "object_storage_enabled";
const STORAGE_PUBLIC_BASE_URL_KEY: &str = "object_storage_public_base_url";
const STORAGE_LOCAL_ROOT_KEY: &str = "object_storage_local_root";
const STORAGE_LOCAL_PATH_RULE_KEY: &str = "object_storage_local_path_rule";
const STORAGE_R2_ENDPOINT_KEY: &str = "object_storage_r2_endpoint";
const STORAGE_R2_BUCKET_KEY: &str = "object_storage_r2_bucket";
const STORAGE_R2_ACCESS_KEY_ID_KEY: &str = "object_storage_r2_access_key_id";
const STORAGE_R2_SECRET_ACCESS_KEY_KEY: &str = "object_storage_r2_secret_access_key";
const STORAGE_R2_REGION_KEY: &str = "object_storage_r2_region";
const STORAGE_S3_ENDPOINT_KEY: &str = "object_storage_s3_endpoint";
const STORAGE_S3_BUCKET_KEY: &str = "object_storage_s3_bucket";
const STORAGE_S3_ACCESS_KEY_ID_KEY: &str = "object_storage_s3_access_key_id";
const STORAGE_S3_SECRET_ACCESS_KEY_KEY: &str = "object_storage_s3_secret_access_key";
const STORAGE_S3_REGION_KEY: &str = "object_storage_s3_region";
const STORAGE_HI168_S3_ENDPOINT_KEY: &str = "object_storage_hi168_s3_endpoint";
const STORAGE_HI168_S3_BUCKET_KEY: &str = "object_storage_hi168_s3_bucket";
const STORAGE_HI168_S3_ACCESS_KEY_ID_KEY: &str = "object_storage_hi168_s3_access_key_id";
const STORAGE_HI168_S3_SECRET_ACCESS_KEY_KEY: &str = "object_storage_hi168_s3_secret_access_key";
const STORAGE_HI168_S3_REGION_KEY: &str = "object_storage_hi168_s3_region";
const STORAGE_ALIYUN_OSS_ENDPOINT_KEY: &str = "object_storage_aliyun_oss_endpoint";
const STORAGE_ALIYUN_OSS_BUCKET_KEY: &str = "object_storage_aliyun_oss_bucket";
const STORAGE_ALIYUN_OSS_ACCESS_KEY_ID_KEY: &str = "object_storage_aliyun_oss_access_key_id";
const STORAGE_ALIYUN_OSS_SECRET_ACCESS_KEY_KEY: &str =
    "object_storage_aliyun_oss_secret_access_key";
const STORAGE_ALIYUN_OSS_REGION_KEY: &str = "object_storage_aliyun_oss_region";

const LEGACY_STORAGE_OSS_ENDPOINT_KEY: &str = "object_storage_oss_endpoint";
const LEGACY_STORAGE_OSS_BUCKET_KEY: &str = "object_storage_oss_bucket";
const LEGACY_STORAGE_OSS_ACCESS_KEY_ID_KEY: &str = "object_storage_oss_access_key_id";
const LEGACY_STORAGE_OSS_SECRET_ACCESS_KEY_KEY: &str = "object_storage_oss_secret_access_key";

const BLOCKED_FILE_EXTENSIONS: &[&str] = &[
    "html", "htm", "js", "mjs", "cjs", "ts", "tsx", "jsx", "php", "py", "sh", "bash", "zsh", "bat",
    "cmd", "ps1", "exe", "dll", "msi", "apk", "jar", "war", "com", "scr",
];

const BLOCKED_CONTENT_TYPES: &[&str] = &[
    "text/html",
    "application/xhtml+xml",
    "application/javascript",
    "text/javascript",
    "application/x-javascript",
    "application/x-msdownload",
    "application/x-msdos-program",
    "application/x-sh",
    "application/x-php",
];

pub struct LocalUploadResult {
    pub provider: String,
    pub key: String,
    pub url: String,
}

#[derive(Clone, Copy)]
enum StorageProvider {
    Local,
    R2,
    S3,
    AliyunOss,
    Hi168S3,
}

#[derive(Clone)]
struct StorageConfig {
    enabled: bool,
    provider: StorageProvider,
    public_base_url: String,
    local_root: String,
    local_path_rule: String,
    r2_endpoint: String,
    r2_bucket: String,
    r2_access_key_id: String,
    r2_secret_access_key: String,
    r2_region: String,
    s3_endpoint: String,
    s3_bucket: String,
    s3_access_key_id: String,
    s3_secret_access_key: String,
    s3_region: String,
    hi168_s3_endpoint: String,
    hi168_s3_bucket: String,
    hi168_s3_access_key_id: String,
    hi168_s3_secret_access_key: String,
    hi168_s3_region: String,
    aliyun_oss_endpoint: String,
    aliyun_oss_bucket: String,
    aliyun_oss_access_key_id: String,
    aliyun_oss_secret_access_key: String,
    aliyun_oss_region: String,
}

#[derive(Clone)]
struct S3ProviderConfig {
    provider_name: &'static str,
    endpoint: String,
    bucket: String,
    access_key_id: String,
    secret_access_key: String,
    region: String,
    default_virtual_hosted_url: bool,
    public_base_url: String,
}

pub async fn upload_local_file(
    state: &AppState,
    file_name: &str,
    content_type: &str,
    content: &[u8],
    image_only: bool,
) -> AppResult<LocalUploadResult> {
    if file_name.trim().is_empty() {
        return Err(AppError::Unprocessable("file is required".to_string()));
    }

    if image_only {
        validate_image_file(file_name, content_type, content)?;
    } else {
        validate_media_file(file_name, content_type, content)?;
    }

    let config = load_storage_config(state).await?;
    if !config.enabled {
        return Err(AppError::Unprocessable(
            "Object storage is disabled".to_string(),
        ));
    }

    let extension = guess_file_extension(file_name, content_type);
    let key = build_object_key(&config.local_path_rule, &extension);

    let (provider_name, url) = match config.provider {
        StorageProvider::Local => {
            let local_url = upload_local_to_filesystem(state, &config, &key, content).await?;
            ("local".to_string(), local_url)
        }
        StorageProvider::R2 => {
            let url = upload_r2_file(&config, &key, content_type, content).await?;
            ("r2".to_string(), url)
        }
        StorageProvider::S3 => {
            let url = upload_s3_file(&config, &key, content_type, content).await?;
            ("s3".to_string(), url)
        }
        StorageProvider::Hi168S3 => {
            let url = upload_hi168_s3_file(&config, &key, content_type, content).await?;
            ("hi168_s3".to_string(), url)
        }
        StorageProvider::AliyunOss => {
            let url = upload_aliyun_oss_file(&config, &key, content_type, content).await?;
            ("aliyun_oss".to_string(), url)
        }
    };

    Ok(LocalUploadResult {
        provider: provider_name,
        key,
        url,
    })
}

pub async fn resolve_local_file(state: &AppState, file_path: &str) -> AppResult<Option<PathBuf>> {
    let config = load_storage_config(state).await?;
    let root = resolve_local_root(state, &config.local_root);
    let relative = sanitize_relative_path(file_path);
    let Some(relative) = relative else {
        return Ok(None);
    };

    let target = root.join(relative);
    if !is_path_within_root(&target, &root) {
        return Ok(None);
    }
    let metadata = tokio::fs::metadata(&target).await;
    if let Ok(metadata) = metadata {
        if metadata.is_file() {
            return Ok(Some(target));
        }
    }

    Ok(None)
}

async fn upload_local_to_filesystem(
    state: &AppState,
    config: &StorageConfig,
    key: &str,
    content: &[u8],
) -> AppResult<String> {
    let root = resolve_local_root(state, &config.local_root);
    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|error| AppError::internal(format!("Failed to create storage root: {error}")))?;

    let relative_path = sanitize_relative_path(key)
        .ok_or_else(|| AppError::internal("Failed to normalize upload path".to_string()))?;
    let target = root.join(&relative_path);

    if !is_path_within_root(&target, &root) {
        return Err(AppError::internal(
            "Resolved upload path escapes storage root".to_string(),
        ));
    }

    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|error| {
            AppError::internal(format!("Failed to create upload parent: {error}"))
        })?;
    }
    tokio::fs::write(&target, content)
        .await
        .map_err(|error| AppError::internal(format!("Failed to save upload file: {error}")))?;

    Ok(build_local_public_url(
        &config.public_base_url,
        &relative_path,
    ))
}

async fn upload_r2_file(
    config: &StorageConfig,
    object_key: &str,
    content_type: &str,
    content: &[u8],
) -> AppResult<String> {
    let provider = S3ProviderConfig {
        provider_name: "Cloudflare R2",
        endpoint: config.r2_endpoint.clone(),
        bucket: config.r2_bucket.clone(),
        access_key_id: config.r2_access_key_id.clone(),
        secret_access_key: config.r2_secret_access_key.clone(),
        region: if config.r2_region.trim().is_empty() {
            "auto".to_string()
        } else {
            config.r2_region.clone()
        },
        default_virtual_hosted_url: false,
        public_base_url: config.public_base_url.clone(),
    };

    upload_s3_compatible_file(&provider, object_key, content_type, content, true, false).await
}

async fn upload_s3_file(
    config: &StorageConfig,
    object_key: &str,
    content_type: &str,
    content: &[u8],
) -> AppResult<String> {
    let provider = S3ProviderConfig {
        provider_name: "S3 object storage",
        endpoint: config.s3_endpoint.clone(),
        bucket: config.s3_bucket.clone(),
        access_key_id: config.s3_access_key_id.clone(),
        secret_access_key: config.s3_secret_access_key.clone(),
        region: config.s3_region.clone(),
        default_virtual_hosted_url: false,
        public_base_url: config.public_base_url.clone(),
    };

    upload_s3_compatible_file(&provider, object_key, content_type, content, true, true).await
}

async fn upload_hi168_s3_file(
    config: &StorageConfig,
    object_key: &str,
    content_type: &str,
    content: &[u8],
) -> AppResult<String> {
    let mut regions = Vec::<String>::new();
    let first = config.hi168_s3_region.trim().to_string();
    if !first.is_empty() && !first.eq_ignore_ascii_case("auto") {
        regions.push(first);
    }
    if !regions
        .iter()
        .any(|region| region.eq_ignore_ascii_case("us-east-1"))
    {
        regions.push("us-east-1".to_string());
    }

    let mut last_error = String::new();
    for region in regions {
        let provider = S3ProviderConfig {
            provider_name: "HI168 S3",
            endpoint: config.hi168_s3_endpoint.clone(),
            bucket: config.hi168_s3_bucket.clone(),
            access_key_id: config.hi168_s3_access_key_id.clone(),
            secret_access_key: config.hi168_s3_secret_access_key.clone(),
            region,
            default_virtual_hosted_url: false,
            public_base_url: config.public_base_url.clone(),
        };

        match upload_s3_compatible_file(&provider, object_key, content_type, content, false, true)
            .await
        {
            Ok(url) => return Ok(url),
            Err(error) => last_error = error.to_string(),
        }
    }

    Err(AppError::Unprocessable(if last_error.is_empty() {
        "HI168 S3 upload failed".to_string()
    } else {
        format!("HI168 S3 upload failed: {last_error}")
    }))
}

async fn upload_aliyun_oss_file(
    config: &StorageConfig,
    object_key: &str,
    content_type: &str,
    content: &[u8],
) -> AppResult<String> {
    let provider = S3ProviderConfig {
        provider_name: "Aliyun OSS",
        endpoint: config.aliyun_oss_endpoint.clone(),
        bucket: config.aliyun_oss_bucket.clone(),
        access_key_id: config.aliyun_oss_access_key_id.clone(),
        secret_access_key: config.aliyun_oss_secret_access_key.clone(),
        region: config.aliyun_oss_region.clone(),
        default_virtual_hosted_url: true,
        public_base_url: config.public_base_url.clone(),
    };

    upload_s3_compatible_file(&provider, object_key, content_type, content, true, true).await
}

async fn upload_s3_compatible_file(
    provider: &S3ProviderConfig,
    object_key: &str,
    content_type: &str,
    content: &[u8],
    allow_virtual_host_retry: bool,
    include_us_east_fallback: bool,
) -> AppResult<String> {
    if provider.endpoint.trim().is_empty()
        || provider.bucket.trim().is_empty()
        || provider.access_key_id.trim().is_empty()
        || provider.secret_access_key.trim().is_empty()
    {
        return Err(AppError::Unprocessable(format!(
            "{} config is incomplete",
            provider.provider_name
        )));
    }

    let region_candidates = build_region_candidates(
        &provider.region,
        &provider.endpoint,
        include_us_east_fallback,
    );

    let credentials = Credentials::new(
        provider.access_key_id.clone(),
        provider.secret_access_key.clone(),
        None,
        None,
        "object-storage",
    );

    let mut attempts = vec![true, false];
    if !allow_virtual_host_retry {
        attempts = vec![true];
    }

    let mut last_error = String::new();
    for region_text in region_candidates {
        for force_path_style in attempts.iter().copied() {
            let conf = aws_sdk_s3::config::Builder::new()
                .region(Region::new(region_text.clone()))
                .credentials_provider(credentials.clone())
                .endpoint_url(provider.endpoint.clone())
                .force_path_style(force_path_style)
                .build();
            let client = S3Client::from_conf(conf);

            let mut request = client
                .put_object()
                .bucket(provider.bucket.clone())
                .key(object_key.to_string())
                .body(ByteStream::from(content.to_vec()));

            let normalized_content_type = content_type.trim();
            if !normalized_content_type.is_empty() {
                request = request.content_type(normalized_content_type.to_string());
            }

            match request.send().await {
                Ok(_) => {
                    if !provider.public_base_url.trim().is_empty() {
                        return Ok(join_public_url(&provider.public_base_url, object_key));
                    }
                    return Ok(build_s3_default_public_url(
                        &provider.endpoint,
                        &provider.bucket,
                        object_key,
                        provider.default_virtual_hosted_url && !force_path_style,
                    ));
                }
                Err(error) => {
                    last_error = format!(
                        "region={region_text}, path_style={force_path_style}, error={}",
                        error
                    );
                }
            }
        }
    }

    Err(AppError::Unprocessable(format!(
        "{} upload failed: {}",
        provider.provider_name,
        if last_error.is_empty() {
            "unknown error"
        } else {
            &last_error
        }
    )))
}

fn build_region_candidates(
    configured_region: &str,
    endpoint: &str,
    include_us_east_fallback: bool,
) -> Vec<String> {
    let mut values = Vec::<String>::new();

    let configured = configured_region.trim();
    if !configured.is_empty() && !configured.eq_ignore_ascii_case("auto") {
        values.push(configured.to_string());
    }

    let inferred = infer_region_from_endpoint(endpoint, "");
    if !inferred.is_empty() {
        values.push(inferred);
    }

    if include_us_east_fallback {
        values.push("us-east-1".to_string());
    }

    if values.is_empty() {
        values.push("us-east-1".to_string());
    }

    let mut deduped = Vec::<String>::new();
    for value in values {
        if deduped.iter().any(|item| item.eq_ignore_ascii_case(&value)) {
            continue;
        }
        deduped.push(value);
    }
    deduped
}

fn build_s3_default_public_url(
    endpoint: &str,
    bucket: &str,
    object_key: &str,
    virtual_hosted: bool,
) -> String {
    if let Ok(url) = Url::parse(endpoint) {
        let scheme = url.scheme();
        let host = url.host_str().unwrap_or_default();
        let port = url
            .port()
            .map(|value| format!(":{value}"))
            .unwrap_or_default();
        if !host.is_empty() {
            if virtual_hosted {
                return join_public_url(&format!("{scheme}://{bucket}.{host}{port}"), object_key);
            }
            return join_public_url(&format!("{scheme}://{host}{port}/{bucket}"), object_key);
        }
    }

    join_public_url(
        &format!("{}/{}", endpoint.trim_end_matches('/'), bucket),
        object_key,
    )
}

fn infer_region_from_endpoint(endpoint: &str, fallback: &str) -> String {
    let host = if let Ok(parsed) = Url::parse(endpoint) {
        parsed.host_str().unwrap_or_default().to_lowercase()
    } else {
        endpoint.to_lowercase()
    };

    if let Some(index) = host.find("oss-") {
        let candidate = &host[index + 4..];
        if let Some(end) = candidate.find('.') {
            let value = &candidate[..end];
            if value.contains('-') {
                return value.to_string();
            }
        }
    }

    if let Some(index) = host.find("s3.") {
        let candidate = &host[index + 3..];
        if let Some(end) = candidate.find('.') {
            let value = &candidate[..end];
            if value.contains('-') {
                return value.to_string();
            }
        }
    }

    if let Some(index) = host.find("s3-") {
        let candidate = &host[index + 3..];
        if let Some(end) = candidate.find('.') {
            let value = &candidate[..end];
            if value.contains('-') {
                return value.to_string();
            }
        }
    }

    fallback.to_string()
}

fn resolve_local_root(state: &AppState, local_root: &str) -> PathBuf {
    let candidate = PathBuf::from(local_root.trim());
    if candidate.is_absolute() {
        candidate
    } else {
        state.paths.project_root.join(candidate)
    }
}

async fn load_storage_config(state: &AppState) -> AppResult<StorageConfig> {
    let rows = sqlx::query(
        "SELECT setting_key, setting_content FROM settings WHERE setting_key = ANY($1)",
    )
    .bind(vec![
        STORAGE_PROVIDER_KEY,
        STORAGE_ENABLED_KEY,
        STORAGE_PUBLIC_BASE_URL_KEY,
        STORAGE_LOCAL_ROOT_KEY,
        STORAGE_LOCAL_PATH_RULE_KEY,
        STORAGE_R2_ENDPOINT_KEY,
        STORAGE_R2_BUCKET_KEY,
        STORAGE_R2_ACCESS_KEY_ID_KEY,
        STORAGE_R2_SECRET_ACCESS_KEY_KEY,
        STORAGE_R2_REGION_KEY,
        STORAGE_S3_ENDPOINT_KEY,
        STORAGE_S3_BUCKET_KEY,
        STORAGE_S3_ACCESS_KEY_ID_KEY,
        STORAGE_S3_SECRET_ACCESS_KEY_KEY,
        STORAGE_S3_REGION_KEY,
        STORAGE_HI168_S3_ENDPOINT_KEY,
        STORAGE_HI168_S3_BUCKET_KEY,
        STORAGE_HI168_S3_ACCESS_KEY_ID_KEY,
        STORAGE_HI168_S3_SECRET_ACCESS_KEY_KEY,
        STORAGE_HI168_S3_REGION_KEY,
        STORAGE_ALIYUN_OSS_ENDPOINT_KEY,
        STORAGE_ALIYUN_OSS_BUCKET_KEY,
        STORAGE_ALIYUN_OSS_ACCESS_KEY_ID_KEY,
        STORAGE_ALIYUN_OSS_SECRET_ACCESS_KEY_KEY,
        STORAGE_ALIYUN_OSS_REGION_KEY,
        LEGACY_STORAGE_OSS_ENDPOINT_KEY,
        LEGACY_STORAGE_OSS_BUCKET_KEY,
        LEGACY_STORAGE_OSS_ACCESS_KEY_ID_KEY,
        LEGACY_STORAGE_OSS_SECRET_ACCESS_KEY_KEY,
    ])
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to load storage settings: {error}")))?;

    let mut setting_map = HashMap::<String, String>::new();
    for row in rows {
        let key = row.try_get::<String, _>("setting_key").unwrap_or_default();
        let value = row
            .try_get::<Option<String>, _>("setting_content")
            .ok()
            .flatten()
            .unwrap_or_default()
            .trim()
            .to_string();
        setting_map.insert(key, value);
    }

    let provider = normalize_provider(setting_map.get(STORAGE_PROVIDER_KEY).map(|v| v.as_str()));
    let enabled = parse_boolean(
        setting_map.get(STORAGE_ENABLED_KEY).map(|v| v.as_str()),
        true,
    );

    Ok(StorageConfig {
        provider,
        enabled,
        public_base_url: normalize_base_url(
            setting_map
                .get(STORAGE_PUBLIC_BASE_URL_KEY)
                .map(|v| v.as_str())
                .unwrap_or(""),
        ),
        local_root: normalize_text(setting_map.get(STORAGE_LOCAL_ROOT_KEY).map(|v| v.as_str()))
            .unwrap_or_else(|| DEFAULT_LOCAL_ROOT.to_string()),
        local_path_rule: normalize_text(
            setting_map
                .get(STORAGE_LOCAL_PATH_RULE_KEY)
                .map(|v| v.as_str()),
        )
        .unwrap_or_else(|| DEFAULT_LOCAL_PATH_RULE.to_string()),
        r2_endpoint: normalize_endpoint(
            setting_map
                .get(STORAGE_R2_ENDPOINT_KEY)
                .map(|v| v.as_str())
                .unwrap_or(""),
        ),
        r2_bucket: normalize_text(setting_map.get(STORAGE_R2_BUCKET_KEY).map(|v| v.as_str()))
            .unwrap_or_default(),
        r2_access_key_id: normalize_text(
            setting_map
                .get(STORAGE_R2_ACCESS_KEY_ID_KEY)
                .map(|v| v.as_str()),
        )
        .unwrap_or_default(),
        r2_secret_access_key: normalize_text(
            setting_map
                .get(STORAGE_R2_SECRET_ACCESS_KEY_KEY)
                .map(|v| v.as_str()),
        )
        .unwrap_or_default(),
        r2_region: normalize_text(setting_map.get(STORAGE_R2_REGION_KEY).map(|v| v.as_str()))
            .unwrap_or_else(|| "auto".to_string()),
        s3_endpoint: normalize_endpoint(
            read_setting_with_fallback(
                &setting_map,
                STORAGE_S3_ENDPOINT_KEY,
                &[LEGACY_STORAGE_OSS_ENDPOINT_KEY],
            )
            .as_deref()
            .unwrap_or(""),
        ),
        s3_bucket: read_setting_with_fallback(
            &setting_map,
            STORAGE_S3_BUCKET_KEY,
            &[LEGACY_STORAGE_OSS_BUCKET_KEY],
        )
        .unwrap_or_default(),
        s3_access_key_id: read_setting_with_fallback(
            &setting_map,
            STORAGE_S3_ACCESS_KEY_ID_KEY,
            &[LEGACY_STORAGE_OSS_ACCESS_KEY_ID_KEY],
        )
        .unwrap_or_default(),
        s3_secret_access_key: read_setting_with_fallback(
            &setting_map,
            STORAGE_S3_SECRET_ACCESS_KEY_KEY,
            &[LEGACY_STORAGE_OSS_SECRET_ACCESS_KEY_KEY],
        )
        .unwrap_or_default(),
        s3_region: normalize_text(setting_map.get(STORAGE_S3_REGION_KEY).map(|v| v.as_str()))
            .unwrap_or_default(),
        hi168_s3_endpoint: normalize_endpoint(
            setting_map
                .get(STORAGE_HI168_S3_ENDPOINT_KEY)
                .map(|v| v.as_str())
                .unwrap_or(""),
        ),
        hi168_s3_bucket: normalize_text(
            setting_map
                .get(STORAGE_HI168_S3_BUCKET_KEY)
                .map(|v| v.as_str()),
        )
        .unwrap_or_default(),
        hi168_s3_access_key_id: normalize_text(
            setting_map
                .get(STORAGE_HI168_S3_ACCESS_KEY_ID_KEY)
                .map(|v| v.as_str()),
        )
        .unwrap_or_default(),
        hi168_s3_secret_access_key: normalize_text(
            setting_map
                .get(STORAGE_HI168_S3_SECRET_ACCESS_KEY_KEY)
                .map(|v| v.as_str()),
        )
        .unwrap_or_default(),
        hi168_s3_region: normalize_text(
            setting_map
                .get(STORAGE_HI168_S3_REGION_KEY)
                .map(|v| v.as_str()),
        )
        .unwrap_or_default(),
        aliyun_oss_endpoint: normalize_endpoint(
            setting_map
                .get(STORAGE_ALIYUN_OSS_ENDPOINT_KEY)
                .map(|v| v.as_str())
                .unwrap_or(""),
        ),
        aliyun_oss_bucket: normalize_text(
            setting_map
                .get(STORAGE_ALIYUN_OSS_BUCKET_KEY)
                .map(|v| v.as_str()),
        )
        .unwrap_or_default(),
        aliyun_oss_access_key_id: normalize_text(
            setting_map
                .get(STORAGE_ALIYUN_OSS_ACCESS_KEY_ID_KEY)
                .map(|v| v.as_str()),
        )
        .unwrap_or_default(),
        aliyun_oss_secret_access_key: normalize_text(
            setting_map
                .get(STORAGE_ALIYUN_OSS_SECRET_ACCESS_KEY_KEY)
                .map(|v| v.as_str()),
        )
        .unwrap_or_default(),
        aliyun_oss_region: normalize_text(
            setting_map
                .get(STORAGE_ALIYUN_OSS_REGION_KEY)
                .map(|v| v.as_str()),
        )
        .unwrap_or_default(),
    })
}

fn normalize_provider(value: Option<&str>) -> StorageProvider {
    let text = value.unwrap_or_default().trim().to_lowercase();
    match text.as_str() {
        "r2" => StorageProvider::R2,
        "s3" => StorageProvider::S3,
        "aliyun_oss" => StorageProvider::AliyunOss,
        "hi168_s3" => StorageProvider::Hi168S3,
        "oss" => StorageProvider::S3,
        _ => StorageProvider::Local,
    }
}

fn parse_boolean(value: Option<&str>, default: bool) -> bool {
    let Some(value) = value else {
        return default;
    };
    let normalized = value.trim().to_lowercase();
    if normalized.is_empty() {
        return default;
    }
    matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
}

fn normalize_text(value: Option<&str>) -> Option<String> {
    let normalized = value.unwrap_or_default().trim().to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_endpoint(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.contains("://") {
        trimmed.trim_end_matches('/').to_string()
    } else {
        format!("https://{}", trimmed.trim_end_matches('/'))
    }
}

fn normalize_base_url(value: &str) -> String {
    let text = value.trim();
    if text.is_empty() {
        return String::new();
    }
    if let Ok(parsed) = Url::parse(text) {
        let scheme = parsed.scheme().to_lowercase();
        if scheme == "http" || scheme == "https" {
            return text.trim_end_matches('/').to_string();
        }
    }
    String::new()
}

fn read_setting_with_fallback(
    setting_map: &HashMap<String, String>,
    primary: &str,
    fallback_keys: &[&str],
) -> Option<String> {
    if let Some(value) = normalize_text(setting_map.get(primary).map(|v| v.as_str())) {
        return Some(value);
    }
    for key in fallback_keys {
        if let Some(value) = normalize_text(setting_map.get(*key).map(|v| v.as_str())) {
            return Some(value);
        }
    }
    None
}

fn validate_image_file(file_name: &str, content_type: &str, content: &[u8]) -> AppResult<()> {
    if content.is_empty() {
        return Err(AppError::Unprocessable("Upload file is empty".to_string()));
    }
    if content.len() > MAX_IMAGE_SIZE_BYTES {
        return Err(AppError::Unprocessable(
            "Image size cannot exceed 20MB".to_string(),
        ));
    }

    let normalized_content_type = content_type.trim().to_lowercase();
    if normalized_content_type.starts_with("image/") {
        return Ok(());
    }

    let extension = Path::new(file_name)
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if matches!(
        extension.as_str(),
        "jpeg" | "jpg" | "png" | "webp" | "gif" | "svg" | "bmp" | "avif"
    ) {
        Ok(())
    } else {
        Err(AppError::Unprocessable(
            "Only image file uploads are allowed".to_string(),
        ))
    }
}

fn validate_media_file(file_name: &str, content_type: &str, content: &[u8]) -> AppResult<()> {
    if content.is_empty() {
        return Err(AppError::Unprocessable("Upload file is empty".to_string()));
    }
    if content.len() > MAX_MEDIA_SIZE_BYTES {
        return Err(AppError::Unprocessable(
            "Upload file exceeds 200 MB".to_string(),
        ));
    }

    let normalized_content_type = content_type.trim().to_lowercase();
    if BLOCKED_CONTENT_TYPES.contains(&normalized_content_type.as_str()) {
        return Err(AppError::Unprocessable(
            "Unsupported upload content_type".to_string(),
        ));
    }

    let extension = Path::new(file_name)
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let normalized_ext = normalize_extension(&extension);
    if BLOCKED_FILE_EXTENSIONS.contains(&normalized_ext.as_str()) {
        return Err(AppError::Unprocessable(
            "Unsupported upload file extension".to_string(),
        ));
    }

    Ok(())
}

fn build_local_public_url(public_base_url: &str, key: &str) -> String {
    let base = build_local_public_base_url(public_base_url);
    join_public_url(&base, key)
}

fn build_local_public_base_url(public_base_url: &str) -> String {
    let normalized = normalize_base_url(public_base_url);
    if normalized.is_empty() {
        return DEFAULT_LOCAL_URL_PREFIX.to_string();
    }

    if let Ok(parsed) = Url::parse(&normalized) {
        let path = parsed.path().trim_end_matches('/').to_string();
        if path.ends_with(DEFAULT_LOCAL_URL_PREFIX) {
            return normalized;
        }
    }

    format!("{normalized}{DEFAULT_LOCAL_URL_PREFIX}")
}

fn join_public_url(base_url: &str, key: &str) -> String {
    let encoded_key = encode_storage_key(key);
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        encoded_key.trim_start_matches('/')
    )
}

fn encode_storage_key(key: &str) -> String {
    key.split('/')
        .map(encode_path_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn encode_path_segment(segment: &str) -> String {
    let mut encoded = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        let ch = byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            encoded.push(ch);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn sanitize_relative_path(value: &str) -> Option<String> {
    let normalized = value.replace('\\', "/").trim().to_string();
    if normalized.is_empty() {
        return None;
    }

    let without_prefix = normalized.trim_start_matches('/').to_string();
    let path = Path::new(&without_prefix);
    let mut safe = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) | Component::ParentDir => return None,
        }
    }

    if safe.as_os_str().is_empty() {
        None
    } else {
        Some(safe.to_string_lossy().replace('\\', "/"))
    }
}

fn is_path_within_root(target: &Path, root: &Path) -> bool {
    target.starts_with(root)
}

fn random_name(bytes: usize) -> String {
    let mut raw = vec![0_u8; bytes.max(8)];
    rand::thread_rng().fill_bytes(&mut raw);
    hex::encode(raw)
}

fn build_object_key(path_rule: &str, extension: &str) -> String {
    let now = Utc::now();
    let random = random_name(8);
    let fallback = format!(
        "{:04}{:02}{:02}/{}.{}",
        now.year(),
        now.month(),
        now.day(),
        random,
        extension
    );

    let mut template = path_rule.trim().to_string();
    if template.is_empty() {
        template = DEFAULT_LOCAL_PATH_RULE.to_string();
    }
    if !template.starts_with('/') {
        template.insert(0, '/');
    }

    let timestamp = now.timestamp().to_string();
    let pairs = [
        ("{year}", format!("{:04}", now.year())),
        ("{month}", format!("{:02}", now.month())),
        ("{day}", format!("{:02}", now.day())),
        ("{hour}", format!("{:02}", now.hour())),
        ("{minute}", format!("{:02}", now.minute())),
        ("{second}", format!("{:02}", now.second())),
        ("{timestamp}", timestamp),
        ("{random_name}", random),
        ("{file_type}", extension.to_string()),
    ];

    let mut rendered = template;
    for (key, value) in pairs {
        rendered = rendered.replace(key, &value);
    }

    let object_key = sanitize_relative_path(&rendered).unwrap_or(fallback);
    let file_name = Path::new(&object_key)
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_default();
    if file_name.contains('.') {
        object_key
    } else {
        format!("{object_key}.{extension}")
    }
}

fn guess_file_extension(file_name: &str, content_type: &str) -> String {
    let ext = Path::new(file_name)
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let normalized = normalize_extension(&ext);
    if !normalized.is_empty() {
        return normalized;
    }

    let mapped = match content_type.trim().to_lowercase().as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/svg+xml" => "svg",
        "image/bmp" => "bmp",
        "image/avif" => "avif",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/ogg" => "ogv",
        "video/quicktime" => "mov",
        "video/x-matroska" => "mkv",
        "audio/mpeg" => "mp3",
        "audio/wav" => "wav",
        "audio/ogg" => "ogg",
        "audio/flac" => "flac",
        "audio/aac" => "aac",
        "application/pdf" => "pdf",
        "text/plain" => "txt",
        "text/markdown" => "md",
        "text/csv" => "csv",
        "application/json" => "json",
        "application/zip" | "application/x-zip-compressed" => "zip",
        "application/x-rar-compressed" | "application/vnd.rar" => "rar",
        "application/x-7z-compressed" => "7z",
        "application/msword" => "doc",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.ms-excel" => "xls",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "application/vnd.ms-powerpoint" => "ppt",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",
        _ => "bin",
    };

    mapped.to_string()
}

fn normalize_extension(ext: &str) -> String {
    let normalized = ext.trim().trim_start_matches('.').to_lowercase();
    if normalized.is_empty() {
        return String::new();
    }
    if normalized.len() > 16 || !normalized.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return String::new();
    }
    if normalized == "jpeg" {
        "jpg".to_string()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        DEFAULT_LOCAL_URL_PREFIX, StorageProvider, build_local_public_base_url, build_object_key,
        build_region_candidates, infer_region_from_endpoint, join_public_url, normalize_provider,
        read_setting_with_fallback, sanitize_relative_path, validate_media_file,
    };
    use crate::error::AppError;

    #[test]
    fn provider_normalization_supports_legacy_oss() {
        assert!(matches!(
            normalize_provider(Some("oss")),
            StorageProvider::S3
        ));
        assert!(matches!(
            normalize_provider(Some("aliyun_oss")),
            StorageProvider::AliyunOss
        ));
        assert!(matches!(
            normalize_provider(Some("unknown")),
            StorageProvider::Local
        ));
    }

    #[test]
    fn setting_fallback_prefers_primary_then_legacy() {
        let mut map = HashMap::<String, String>::new();
        map.insert("legacy_key".to_string(), "legacy-value".to_string());
        assert_eq!(
            read_setting_with_fallback(&map, "primary_key", &["legacy_key"]),
            Some("legacy-value".to_string())
        );

        map.insert("primary_key".to_string(), "primary-value".to_string());
        assert_eq!(
            read_setting_with_fallback(&map, "primary_key", &["legacy_key"]),
            Some("primary-value".to_string())
        );
    }

    #[test]
    fn local_public_base_url_behavior_matches_python() {
        assert_eq!(
            build_local_public_base_url(""),
            DEFAULT_LOCAL_URL_PREFIX.to_string()
        );
        assert_eq!(
            build_local_public_base_url("https://cdn.example.com"),
            "https://cdn.example.com/storage".to_string()
        );
        assert_eq!(
            build_local_public_base_url("https://cdn.example.com/storage"),
            "https://cdn.example.com/storage".to_string()
        );
    }

    #[test]
    fn join_public_url_encodes_object_key_segments() {
        let url = join_public_url("https://cdn.example.com/storage", "2026/04/my image.png");
        assert_eq!(
            url,
            "https://cdn.example.com/storage/2026/04/my%20image.png"
        );
    }

    #[test]
    fn object_key_appends_extension_when_missing() {
        let key = build_object_key("/{year}/{month}/{random_name}", "jpg");
        assert!(key.ends_with(".jpg"));
    }

    #[test]
    fn sanitize_path_blocks_traversal() {
        assert!(sanitize_relative_path("../passwd").is_none());
        assert!(sanitize_relative_path("a/../../b").is_none());
        assert_eq!(
            sanitize_relative_path("/safe/path/file.txt"),
            Some("safe/path/file.txt".to_string())
        );
    }

    #[test]
    fn media_validation_blocks_dangerous_types() {
        let blocked_by_content_type =
            validate_media_file("a.txt", "text/html", b"<html>evil</html>");
        assert!(matches!(
            blocked_by_content_type,
            Err(AppError::Unprocessable(_))
        ));

        let blocked_by_extension = validate_media_file("shell.sh", "text/plain", b"echo test");
        assert!(matches!(
            blocked_by_extension,
            Err(AppError::Unprocessable(_))
        ));

        let allowed = validate_media_file("doc.pdf", "application/pdf", b"pdf-content");
        assert!(allowed.is_ok());
    }

    #[test]
    fn region_inference_from_common_endpoints() {
        assert_eq!(
            infer_region_from_endpoint("https://oss-cn-hangzhou.aliyuncs.com", "us-east-1"),
            "cn-hangzhou".to_string()
        );
        assert_eq!(
            infer_region_from_endpoint("https://s3.ap-shanghai.myqcloud.com", "us-east-1"),
            "ap-shanghai".to_string()
        );
    }

    #[test]
    fn region_candidates_dedup_and_fallback() {
        let values =
            build_region_candidates("ap-shanghai", "https://s3.ap-shanghai.myqcloud.com", true);
        assert_eq!(
            values,
            vec!["ap-shanghai".to_string(), "us-east-1".to_string()]
        );

        let values = build_region_candidates("", "https://custom-endpoint.local", false);
        assert_eq!(values, vec!["us-east-1".to_string()]);
    }
}
