use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use mime_guess::from_path;
use tokio::io::ErrorKind;
use tokio_util::io::ReaderStream;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    storage_local,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/storage/object/{*object_key}", get(get_storage_object_redirect))
        .route("/storage/{*file_path}", get(get_local_storage_file))
}

async fn get_storage_object_redirect(
    method: Method,
    State(state): State<AppState>,
    Path(object_key): Path<String>,
) -> AppResult<impl IntoResponse> {
    if method != Method::GET && method != Method::HEAD {
        return Err(AppError::not_found("Not Found"));
    }

    let signed_url = storage_local::resolve_object_access_url(&state, &object_key).await?;
    Ok(Redirect::temporary(&signed_url))
}

async fn get_local_storage_file(
    method: Method,
    State(state): State<AppState>,
    Path(file_path): Path<String>,
) -> AppResult<impl IntoResponse> {
    if method != Method::GET && method != Method::HEAD {
        return Err(AppError::not_found("Not Found"));
    }

    let target = storage_local::resolve_local_file(&state, &file_path).await?;
    let Some(target) = target else {
        return Err(AppError::not_found("Storage file not found"));
    };

    let mime = from_path(&target).first_or_octet_stream().to_string();
    let content_type = HeaderValue::from_str(&mime)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    let metadata = tokio::fs::metadata(&target)
        .await
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => AppError::not_found("Storage file not found"),
            _ => AppError::internal(format!("Failed to read storage file metadata: {error}")),
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

    let file = tokio::fs::File::open(&target)
        .await
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => AppError::not_found("Storage file not found"),
            _ => AppError::internal(format!("Failed to open storage file: {error}")),
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
