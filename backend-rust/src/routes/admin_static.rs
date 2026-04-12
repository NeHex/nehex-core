use std::{path::PathBuf, time::Duration};

use axum::{
    body::Body,
    extract::{OriginalUri, State},
    http::{HeaderValue, Method, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use mime_guess::from_path;
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::Row;
use tokio::fs;
use tokio::io::ErrorKind;
use tokio_util::io::ReaderStream;
use tracing::warn;

use crate::{
    config::normalize_admin_manager_web_path,
    error::{AppError, AppResult},
    state::{AdminIndexTemplateCache, AdminPathCache, AppState},
};

const ADMIN_BASE_PLACEHOLDER: &str = "__ADMIN_MANAGER_WEB__";
const ADMIN_PATH_CACHE_TTL_SECONDS: u64 = 10;

static MDI_PRELOAD_LINK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\s*<link[^>]*rel=\"preload\"[^>]*href=\"assets/materialdesignicons-webfont[^\"]+\"[^>]*>\s*"#)
        .expect("mdi preload regex must compile")
});

pub async fn fallback_handler(
    method: Method,
    uri: OriginalUri,
    State(state): State<AppState>,
) -> AppResult<Response> {
    if method != Method::GET && method != Method::HEAD {
        return Err(AppError::not_found("Not Found"));
    }

    if !state.paths.admin_index_file.exists() {
        return Err(AppError::not_found(
            "Admin manager frontend not found. Build app/nehex-admin first.",
        ));
    }

    let request_path = uri.path().to_string();
    let admin_base_path = get_admin_manager_web_path(&state).await;
    let relative_path = extract_admin_relative_path(&request_path, &admin_base_path);

    let Some(relative_path) = relative_path else {
        return Err(AppError::not_found("Not Found"));
    };

    if relative_path.is_empty() && request_path != format!("{}/", admin_base_path) {
        let redirect = Redirect::permanent(&format!("{}/", admin_base_path));
        return Ok(redirect.into_response());
    }

    if let Some(file_path) = resolve_admin_file(&state, &relative_path).await {
        return serve_file(file_path, method).await;
    }

    if !relative_path.is_empty() && std::path::Path::new(&relative_path).extension().is_some() {
        return Err(AppError::not_found("Admin asset not found."));
    }

    let html = render_admin_index(&state, &admin_base_path).await?;
    if method == Method::HEAD {
        return Ok((
            StatusCode::OK,
            [content_type_header("text/html; charset=utf-8")],
        )
            .into_response());
    }

    Ok(Html(html).into_response())
}

async fn serve_file(file_path: PathBuf, method: Method) -> AppResult<Response> {
    let mime = from_path(&file_path).first_or_octet_stream().to_string();
    let content_type = HeaderValue::from_str(&mime)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    let metadata = fs::metadata(&file_path)
        .await
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => AppError::not_found("Admin asset not found."),
            _ => AppError::internal(format!("Failed to read admin asset metadata: {error}")),
        })?;

    if method == Method::HEAD {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::OK;
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, content_type.clone());
        response.headers_mut().insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&metadata.len().to_string())
                .unwrap_or_else(|_| HeaderValue::from_static("0")),
        );
        return Ok(response);
    }

    let file = fs::File::open(&file_path)
        .await
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => AppError::not_found("Admin asset not found."),
            _ => AppError::internal(format!("Failed to open admin asset: {error}")),
        })?;
    let stream = ReaderStream::new(file);
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&metadata.len().to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    Ok(response)
}

fn content_type_header(value: &str) -> (header::HeaderName, HeaderValue) {
    (
        header::CONTENT_TYPE,
        HeaderValue::from_str(value)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    )
}

async fn get_admin_manager_web_path(state: &AppState) -> String {
    let now = std::time::Instant::now();

    {
        let cache_guard = state.admin_path_cache.read().await;
        if let Some(cache) = cache_guard.as_ref() {
            if cache.expires_at > now {
                return cache.value.clone();
            }
        }
    }

    let mut resolved = state.settings.admin_manager_web.clone();

    let query_result =
        sqlx::query("SELECT setting_content FROM settings WHERE setting_key = $1 LIMIT 1")
            .bind("admin_manager_web")
            .fetch_optional(&state.db_pool)
            .await;

    match query_result {
        Ok(Some(row)) => {
            let value = row.try_get::<Option<String>, _>(0).ok().flatten();
            resolved = normalize_admin_manager_web_path(
                value.as_deref(),
                &state.settings.admin_manager_web,
            );
        }
        Ok(None) => {}
        Err(error) => {
            warn!("[admin-static] failed to load admin path from settings table: {error}");
        }
    }

    {
        let mut cache_guard = state.admin_path_cache.write().await;
        *cache_guard = Some(AdminPathCache {
            expires_at: now + Duration::from_secs(ADMIN_PATH_CACHE_TTL_SECONDS),
            value: resolved.clone(),
        });
    }

    resolved
}

async fn read_admin_index_template(state: &AppState) -> AppResult<String> {
    let metadata = fs::metadata(&state.paths.admin_index_file).await?;
    let mtime = metadata.modified().ok();

    {
        let cache_guard = state.admin_index_cache.read().await;
        if let Some(cache) = cache_guard.as_ref() {
            if cache.mtime == mtime {
                return Ok(cache.content.clone());
            }
        }
    }

    let content = fs::read_to_string(&state.paths.admin_index_file).await?;
    {
        let mut cache_guard = state.admin_index_cache.write().await;
        *cache_guard = Some(AdminIndexTemplateCache {
            mtime,
            content: content.clone(),
        });
    }

    Ok(content)
}

async fn render_admin_index(state: &AppState, admin_base_path: &str) -> AppResult<String> {
    let index_html = read_admin_index_template(state).await?;
    let admin_base = admin_base_with_slash(admin_base_path);

    let mut rendered = index_html.replace(ADMIN_BASE_PLACEHOLDER, &admin_base);
    rendered = MDI_PRELOAD_LINK_RE.replace_all(&rendered, "\n").to_string();

    let base_tag = format!("<base href=\"{admin_base}\">");
    if !rendered.contains("<base ") {
        rendered = rendered.replacen("<head>", &format!("<head>\n    {base_tag}"), 1);
    }

    Ok(rendered)
}

fn admin_base_with_slash(path: &str) -> String {
    format!("{}/", path.trim_end_matches('/'))
}

async fn resolve_admin_file(state: &AppState, full_path: &str) -> Option<PathBuf> {
    if full_path.is_empty() {
        return None;
    }

    let candidate = state.paths.admin_dist_dir.join(full_path);
    let resolved = fs::canonicalize(candidate).await.ok()?;
    if !resolved.starts_with(&state.paths.admin_dist_dir_resolved) {
        return None;
    }

    let metadata = fs::metadata(&resolved).await.ok()?;
    if metadata.is_file() {
        Some(resolved)
    } else {
        None
    }
}

fn extract_admin_relative_path(request_path: &str, admin_base_path: &str) -> Option<String> {
    let normalized_request = if request_path.starts_with('/') {
        request_path.to_string()
    } else {
        format!("/{request_path}")
    };

    let mut normalized_admin_base = admin_base_path.trim_end_matches('/').to_string();
    if !normalized_admin_base.starts_with('/') {
        normalized_admin_base.insert(0, '/');
    }

    if normalized_request == normalized_admin_base {
        return Some(String::new());
    }

    let base_prefix = format!("{normalized_admin_base}/");
    if normalized_request == base_prefix {
        return Some(String::new());
    }

    if !normalized_request.starts_with(&base_prefix) {
        return None;
    }

    Some(normalized_request[base_prefix.len()..].to_string())
}
