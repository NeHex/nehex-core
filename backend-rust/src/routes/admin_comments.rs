use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, Method},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

use super::{admin_auth, admin_mail};

#[derive(Serialize)]
pub struct AdminActionResponse {
    success: bool,
    message: String,
}

#[derive(Serialize)]
struct AdminPagination {
    page: i64,
    size: i64,
    total: i64,
    total_pages: i64,
}

#[derive(sqlx::FromRow, Serialize, Clone)]
struct CommentItem {
    id: i64,
    parent_id: i64,
    target_type: String,
    target_id: i64,
    content: String,
    nickname: String,
    email: Option<String>,
    website: Option<String>,
    like_count: i64,
    status: i64,
    ip: Option<String>,
    create_time: NaiveDateTime,
    update_time: NaiveDateTime,
    #[sqlx(skip)]
    replies: Vec<CommentItem>,
}

#[derive(Serialize)]
pub struct AdminCommentListResponse {
    data: Vec<CommentItem>,
    pagination: AdminPagination,
}

#[derive(Serialize)]
pub struct AdminCommentDetailResponse {
    data: CommentItem,
}

#[derive(Deserialize)]
pub struct AdminCommentQuery {
    keyword: Option<String>,
    page: Option<i64>,
    size: Option<i64>,
}

#[derive(Deserialize)]
pub struct AdminCommentCreatePayload {
    parent_id: i64,
    target_type: String,
    target_id: i64,
    content: String,
    nickname: String,
    email: Option<String>,
    website: Option<String>,
    status: i64,
}

#[derive(Deserialize)]
pub struct AdminCommentUpdatePayload {
    parent_id: Option<i64>,
    content: Option<String>,
    nickname: Option<String>,
    email: Option<String>,
    website: Option<String>,
    status: Option<i64>,
}

pub async fn admin_create_comment(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<AdminCommentCreatePayload>,
) -> AppResult<Json<AdminCommentDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let parent_id = payload.parent_id.max(0);
    let target_type = normalize_required_text(payload.target_type, "target_type")?.to_lowercase();
    let target_id = payload.target_id.max(1);
    let content = normalize_required_text(payload.content, "content")?;
    let nickname = normalize_required_text(payload.nickname, "nickname")?;

    let item = sqlx::query_as::<_, CommentItem>(
        r#"
        INSERT INTO comment (
            parent_id,
            target_type,
            target_id,
            content,
            nickname,
            email,
            website,
            status,
            is_admin
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,1)
        RETURNING
            id::bigint AS id,
            parent_id::bigint AS parent_id,
            target_type,
            target_id::bigint AS target_id,
            content,
            nickname,
            email,
            website,
            like_count::bigint AS like_count,
            status::bigint AS status,
            ip,
            create_time,
            update_time
        "#,
    )
    .bind(parent_id)
    .bind(target_type)
    .bind(target_id)
    .bind(content)
    .bind(nickname)
    .bind(normalize_optional_text(payload.email))
    .bind(normalize_optional_text(payload.website))
    .bind(if payload.status > 0 { 1 } else { 0 })
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to create comment: {error}")))?;

    let notice_comment = admin_mail::MailNoticeCommentContext {
        id: item.id,
        parent_id: item.parent_id,
        target_type: item.target_type.clone(),
        target_id: item.target_id,
        nickname: item.nickname.clone(),
        email: item.email.clone(),
        content: item.content.clone(),
        create_time: item.create_time,
    };
    admin_mail::spawn_comment_notification_mails(state.clone(), notice_comment);

    invalidate_comment_cache_for_target(&state, &item.target_type, item.target_id).await;

    Ok(Json(AdminCommentDetailResponse {
        data: with_empty_replies(item),
    }))
}

pub async fn admin_list_comments(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Query(query): Query<AdminCommentQuery>,
) -> AppResult<Json<AdminCommentListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let page = query.page.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * size;

    let keyword = query.keyword.unwrap_or_default().trim().to_string();
    let like_pattern = format!("%{keyword}%");

    let total = if keyword.is_empty() {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM comment")
            .fetch_one(&state.db_pool)
            .await
            .map_err(|error| AppError::internal(format!("Failed to count comments: {error}")))?
    } else {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::bigint
            FROM comment
            WHERE
                content LIKE $1
                OR nickname LIKE $1
                OR target_type LIKE $1
                OR CAST(target_id AS TEXT) LIKE $1
            "#,
        )
        .bind(&like_pattern)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to count comments: {error}")))?
    }
    .max(0);

    let total_pages = if total <= 0 {
        0
    } else {
        (total + size - 1) / size
    };

    let rows = if total <= 0 {
        vec![]
    } else if keyword.is_empty() {
        sqlx::query_as::<_, CommentItem>(
            r#"
            SELECT
                id::bigint AS id,
                parent_id::bigint AS parent_id,
                target_type,
                target_id::bigint AS target_id,
                content,
                nickname,
                email,
                website,
                like_count::bigint AS like_count,
                status::bigint AS status,
                ip,
                create_time,
                update_time
            FROM comment
            ORDER BY create_time DESC, id DESC
            OFFSET $1 LIMIT $2
            "#,
        )
        .bind(offset)
        .bind(size)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list comments: {error}")))?
    } else {
        sqlx::query_as::<_, CommentItem>(
            r#"
            SELECT
                id::bigint AS id,
                parent_id::bigint AS parent_id,
                target_type,
                target_id::bigint AS target_id,
                content,
                nickname,
                email,
                website,
                like_count::bigint AS like_count,
                status::bigint AS status,
                ip,
                create_time,
                update_time
            FROM comment
            WHERE
                content LIKE $1
                OR nickname LIKE $1
                OR target_type LIKE $1
                OR CAST(target_id AS TEXT) LIKE $1
            ORDER BY create_time DESC, id DESC
            OFFSET $2 LIMIT $3
            "#,
        )
        .bind(&like_pattern)
        .bind(offset)
        .bind(size)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list comments: {error}")))?
    };

    let data = rows.into_iter().map(with_empty_replies).collect::<Vec<_>>();

    Ok(Json(AdminCommentListResponse {
        data,
        pagination: AdminPagination {
            page,
            size,
            total,
            total_pages,
        },
    }))
}

pub async fn admin_update_comment(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(comment_id): Path<i64>,
    Json(payload): Json<AdminCommentUpdatePayload>,
) -> AppResult<Json<AdminCommentDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let current = sqlx::query_as::<_, CommentItem>(
        r#"
        SELECT
            id::bigint AS id,
            parent_id::bigint AS parent_id,
            target_type,
            target_id::bigint AS target_id,
            content,
            nickname,
            email,
            website,
            like_count::bigint AS like_count,
            status::bigint AS status,
            ip,
            create_time,
            update_time
        FROM comment
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(comment_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to load comment: {error}")))?
    .ok_or_else(|| AppError::not_found("Comment not found"))?;

    let updated = sqlx::query_as::<_, CommentItem>(
        r#"
        UPDATE comment
        SET
            parent_id = $2,
            content = $3,
            nickname = $4,
            email = $5,
            website = $6,
            status = $7,
            update_time = CURRENT_TIMESTAMP
        WHERE id = $1
        RETURNING
            id::bigint AS id,
            parent_id::bigint AS parent_id,
            target_type,
            target_id::bigint AS target_id,
            content,
            nickname,
            email,
            website,
            like_count::bigint AS like_count,
            status::bigint AS status,
            ip,
            create_time,
            update_time
        "#,
    )
    .bind(comment_id)
    .bind(payload.parent_id.unwrap_or(current.parent_id).max(0))
    .bind(
        payload
            .content
            .map(|value| normalize_required_text(value, "content"))
            .transpose()?
            .unwrap_or(current.content),
    )
    .bind(
        payload
            .nickname
            .map(|value| normalize_required_text(value, "nickname"))
            .transpose()?
            .unwrap_or(current.nickname),
    )
    .bind(normalize_update_optional_text(payload.email, current.email))
    .bind(normalize_update_optional_text(
        payload.website,
        current.website,
    ))
    .bind(
        payload
            .status
            .map(|value| if value > 0 { 1 } else { 0 })
            .unwrap_or(current.status),
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to update comment: {error}")))?;
    invalidate_comment_cache_for_target(&state, &current.target_type, current.target_id).await;

    Ok(Json(AdminCommentDetailResponse {
        data: with_empty_replies(updated),
    }))
}

pub async fn admin_delete_comment(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(comment_id): Path<i64>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let target = sqlx::query_as::<_, (String, i64)>(
        "SELECT target_type, target_id::bigint AS target_id FROM comment WHERE id = $1 LIMIT 1",
    )
    .bind(comment_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to load comment: {error}")))?;

    let Some((target_type, target_id)) = target else {
        return Err(AppError::not_found("Comment not found"));
    };

    sqlx::query(
        r#"
        WITH RECURSIVE descendants(id) AS (
            SELECT id
            FROM comment
            WHERE id = $1
            UNION
            SELECT c.id
            FROM comment c
            INNER JOIN descendants d ON c.parent_id = d.id
        )
        DELETE FROM comment
        WHERE id IN (SELECT id FROM descendants)
        "#,
    )
    .bind(comment_id)
    .execute(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to delete comment: {error}")))?;
    invalidate_comment_cache_for_target(&state, &target_type, target_id).await;

    Ok(Json(AdminActionResponse {
        success: true,
        message: "Comment deleted".to_string(),
    }))
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

fn normalize_update_optional_text(next: Option<String>, current: Option<String>) -> Option<String> {
    match next {
        Some(value) => normalize_optional_text(Some(value)),
        None => current,
    }
}

fn with_empty_replies(mut item: CommentItem) -> CommentItem {
    item.replies = Vec::new();
    item
}

async fn invalidate_comment_cache_for_target(state: &AppState, target_type: &str, target_id: i64) {
    let prefix = format!("comments:list:{target_type}:{target_id}:");
    state.runtime_cache.delete_prefix(&prefix).await;
}
