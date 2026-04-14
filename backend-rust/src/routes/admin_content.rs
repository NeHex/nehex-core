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

use super::{admin_auth, sync_api};

const ARTICLES_CACHE_KEY: &str = "articles:list";
const DAILIES_CACHE_KEY: &str = "dailies:list";
const ALBUMS_CACHE_KEY: &str = "albums:list";
const PAGES_CACHE_KEY: &str = "pages:list";
const PROJECTS_CACHE_KEY: &str = "projects:list";

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

#[derive(sqlx::FromRow, Serialize)]
struct ArticleItem {
    id: i64,
    title: String,
    #[serde(rename = "articleTopImage")]
    article_top_image: Option<String>,
    #[serde(rename = "class")]
    class_name: String,
    read: i64,
    like_count: i64,
    #[serde(rename = "lastEditTime")]
    last_edit_time: NaiveDateTime,
    tag: Option<String>,
    top: i64,
    status: i64,
    content: Option<String>,
}

#[derive(Serialize)]
pub struct AdminArticleListResponse {
    data: Vec<ArticleItem>,
    pagination: AdminPagination,
}

#[derive(Serialize)]
pub struct AdminArticleDetailResponse {
    data: ArticleItem,
}

#[derive(sqlx::FromRow, Serialize)]
struct DailyItem {
    id: i64,
    title: String,
    content: Option<String>,
    create_time: NaiveDateTime,
    weather: Option<String>,
}

#[derive(Serialize)]
pub struct AdminDailyDetailResponse {
    data: DailyItem,
}

#[derive(sqlx::FromRow, Serialize)]
struct AlbumItem {
    id: i64,
    title: String,
    cover: Option<String>,
    #[serde(rename = "class")]
    class_name: String,
    like_count: i64,
    img_urls: Option<String>,
    create_time: NaiveDateTime,
    update_time: NaiveDateTime,
}

#[derive(Serialize)]
pub struct AdminAlbumDetailResponse {
    data: AlbumItem,
}

#[derive(sqlx::FromRow, Serialize)]
struct PageItem {
    id: i64,
    page_key: String,
    title: String,
    cover_image: Option<String>,
    content: Option<String>,
    sort: i64,
    status: i64,
    create_time: NaiveDateTime,
    update_time: NaiveDateTime,
}

#[derive(Serialize)]
pub struct AdminPageListResponse {
    data: Vec<PageItem>,
}

#[derive(Serialize)]
pub struct AdminPageDetailResponse {
    data: PageItem,
}

#[derive(sqlx::FromRow, Serialize)]
struct ProjectItem {
    id: i64,
    title: String,
    cover: Option<String>,
    category: Option<String>,
    description: Option<String>,
    content: Option<String>,
    tech_stack: Option<String>,
    project_url: Option<String>,
    github_url: Option<String>,
    sort: i64,
    status: i64,
    create_time: NaiveDateTime,
    update_time: NaiveDateTime,
}

#[derive(Serialize)]
pub struct AdminProjectListResponse {
    data: Vec<ProjectItem>,
}

#[derive(Serialize)]
pub struct AdminProjectDetailResponse {
    data: ProjectItem,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    page: Option<i64>,
    size: Option<i64>,
}

#[derive(Deserialize)]
pub struct ArticlePayload {
    title: String,
    #[serde(rename = "articleTopImage")]
    article_top_image: Option<String>,
    #[serde(rename = "class")]
    class_name: String,
    read: i64,
    like_count: Option<i64>,
    tag: Option<String>,
    top: i64,
    status: i64,
    content: Option<String>,
}

#[derive(Deserialize)]
pub struct DailyPayload {
    title: String,
    content: Option<String>,
    weather: Option<String>,
}

#[derive(Deserialize)]
pub struct AlbumPayload {
    title: String,
    cover: Option<String>,
    #[serde(rename = "class")]
    class_name: String,
    like_count: i64,
    img_urls: Option<String>,
}

#[derive(Deserialize)]
pub struct PagePayload {
    page_key: String,
    title: String,
    cover_image: Option<String>,
    content: Option<String>,
    sort: i64,
    status: i64,
}

#[derive(Deserialize)]
pub struct ProjectPayload {
    title: String,
    cover: Option<String>,
    category: Option<String>,
    description: Option<String>,
    content: Option<String>,
    tech_stack: Option<String>,
    project_url: Option<String>,
    github_url: Option<String>,
    sort: i64,
    status: i64,
}

pub async fn admin_list_articles(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<AdminArticleListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let page = query.page.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(24).clamp(1, 100);
    let offset = (page - 1) * size;

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM article")
        .fetch_one(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to count articles: {error}")))?
        .max(0);

    let total_pages = if total <= 0 {
        0
    } else {
        (total + size - 1) / size
    };

    let data = if total <= 0 {
        vec![]
    } else {
        sqlx::query_as::<_, ArticleItem>(
            r#"
            SELECT
                id::bigint AS id,
                title,
                "articleTopImage" AS article_top_image,
                class AS class_name,
                read::bigint AS read,
                like_count::bigint AS like_count,
                "lastEditTime" AS last_edit_time,
                tag,
                top::bigint AS top,
                status::bigint AS status,
                content
            FROM article
            ORDER BY top DESC, "lastEditTime" DESC, id DESC
            OFFSET $1 LIMIT $2
            "#,
        )
        .bind(offset)
        .bind(size)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list articles: {error}")))?
    };

    Ok(Json(AdminArticleListResponse {
        data,
        pagination: AdminPagination {
            page,
            size,
            total,
            total_pages,
        },
    }))
}

pub async fn admin_get_article(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(article_id): Path<i64>,
) -> AppResult<Json<AdminArticleDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let item = sqlx::query_as::<_, ArticleItem>(
        r#"
        SELECT
            id::bigint AS id,
            title,
            "articleTopImage" AS article_top_image,
            class AS class_name,
            read::bigint AS read,
            like_count::bigint AS like_count,
            "lastEditTime" AS last_edit_time,
            tag,
            top::bigint AS top,
            status::bigint AS status,
            content
        FROM article
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(article_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to get article: {error}")))?
    .ok_or_else(|| AppError::not_found("Article not found"))?;

    Ok(Json(AdminArticleDetailResponse { data: item }))
}

pub async fn admin_create_article(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<ArticlePayload>,
) -> AppResult<Json<AdminArticleDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let normalized = normalize_article_payload(payload)?;

    let item = sqlx::query_as::<_, ArticleItem>(
        r#"
        INSERT INTO article (
            title,
            "articleTopImage",
            class,
            read,
            like_count,
            tag,
            top,
            status,
            content
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING
            id::bigint AS id,
            title,
            "articleTopImage" AS article_top_image,
            class AS class_name,
            read::bigint AS read,
            like_count::bigint AS like_count,
            "lastEditTime" AS last_edit_time,
            tag,
            top::bigint AS top,
            status::bigint AS status,
            content
        "#,
    )
    .bind(normalized.title)
    .bind(normalized.article_top_image)
    .bind(normalized.class_name)
    .bind(normalized.read)
    .bind(normalized.like_count)
    .bind(normalized.tag)
    .bind(normalized.top)
    .bind(normalized.status)
    .bind(normalized.content)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to create article: {error}")))?;
    invalidate_cache_key(&state, ARTICLES_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "article", "create", vec![item.id]).await;

    Ok(Json(AdminArticleDetailResponse { data: item }))
}

pub async fn admin_update_article(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(article_id): Path<i64>,
    Json(payload): Json<ArticlePayload>,
) -> AppResult<Json<AdminArticleDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let normalized = normalize_article_payload(payload)?;

    let item = sqlx::query_as::<_, ArticleItem>(
        r#"
        UPDATE article
        SET
            title = $2,
            "articleTopImage" = $3,
            class = $4,
            read = $5,
            like_count = $6,
            tag = $7,
            top = $8,
            status = $9,
            content = $10,
            "lastEditTime" = CURRENT_TIMESTAMP
        WHERE id = $1
        RETURNING
            id::bigint AS id,
            title,
            "articleTopImage" AS article_top_image,
            class AS class_name,
            read::bigint AS read,
            like_count::bigint AS like_count,
            "lastEditTime" AS last_edit_time,
            tag,
            top::bigint AS top,
            status::bigint AS status,
            content
        "#,
    )
    .bind(article_id)
    .bind(normalized.title)
    .bind(normalized.article_top_image)
    .bind(normalized.class_name)
    .bind(normalized.read)
    .bind(normalized.like_count)
    .bind(normalized.tag)
    .bind(normalized.top)
    .bind(normalized.status)
    .bind(normalized.content)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to update article: {error}")))?
    .ok_or_else(|| AppError::not_found("Article not found"))?;
    invalidate_cache_key(&state, ARTICLES_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "article", "update", vec![item.id]).await;

    Ok(Json(AdminArticleDetailResponse { data: item }))
}

pub async fn admin_delete_article(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(article_id): Path<i64>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let result = sqlx::query("DELETE FROM article WHERE id = $1")
        .bind(article_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to delete article: {error}")))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Article not found"));
    }
    invalidate_cache_key(&state, ARTICLES_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "article", "delete", vec![article_id])
        .await;

    Ok(Json(AdminActionResponse {
        success: true,
        message: "Article deleted".to_string(),
    }))
}

pub async fn admin_create_daily(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<DailyPayload>,
) -> AppResult<Json<AdminDailyDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let title = normalize_required_text(payload.title, "title")?;

    let item = sqlx::query_as::<_, DailyItem>(
        r#"
        INSERT INTO daily (title, content, weather)
        VALUES ($1, $2, $3)
        RETURNING id::bigint AS id, title, content, create_time, weather
        "#,
    )
    .bind(title)
    .bind(normalize_optional_text(payload.content))
    .bind(normalize_optional_text(payload.weather))
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to create daily: {error}")))?;
    invalidate_cache_key(&state, DAILIES_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "daily", "create", vec![item.id]).await;

    Ok(Json(AdminDailyDetailResponse { data: item }))
}

pub async fn admin_update_daily(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(daily_id): Path<i64>,
    Json(payload): Json<DailyPayload>,
) -> AppResult<Json<AdminDailyDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let title = normalize_required_text(payload.title, "title")?;

    let item = sqlx::query_as::<_, DailyItem>(
        r#"
        UPDATE daily
        SET title = $2, content = $3, weather = $4
        WHERE id = $1
        RETURNING id::bigint AS id, title, content, create_time, weather
        "#,
    )
    .bind(daily_id)
    .bind(title)
    .bind(normalize_optional_text(payload.content))
    .bind(normalize_optional_text(payload.weather))
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to update daily: {error}")))?
    .ok_or_else(|| AppError::not_found("Daily not found"))?;
    invalidate_cache_key(&state, DAILIES_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "daily", "update", vec![item.id]).await;

    Ok(Json(AdminDailyDetailResponse { data: item }))
}

pub async fn admin_get_daily(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(daily_id): Path<i64>,
) -> AppResult<Json<AdminDailyDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let item = sqlx::query_as::<_, DailyItem>(
        "SELECT id::bigint AS id, title, content, create_time, weather FROM daily WHERE id = $1 LIMIT 1",
    )
    .bind(daily_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to get daily: {error}")))?
    .ok_or_else(|| AppError::not_found("Daily not found"))?;

    Ok(Json(AdminDailyDetailResponse { data: item }))
}

pub async fn admin_delete_daily(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(daily_id): Path<i64>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let result = sqlx::query("DELETE FROM daily WHERE id = $1")
        .bind(daily_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to delete daily: {error}")))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Daily not found"));
    }
    invalidate_cache_key(&state, DAILIES_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "daily", "delete", vec![daily_id]).await;

    Ok(Json(AdminActionResponse {
        success: true,
        message: "Daily deleted".to_string(),
    }))
}

pub async fn admin_create_album(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<AlbumPayload>,
) -> AppResult<Json<AdminAlbumDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let title = normalize_required_text(payload.title, "title")?;
    let class_name = normalize_required_text(payload.class_name, "class")?;

    let item = sqlx::query_as::<_, AlbumItem>(
        r#"
        INSERT INTO album (title, cover, class, like_count, img_urls)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING
            id::bigint AS id,
            title,
            cover,
            class AS class_name,
            like_count::bigint AS like_count,
            img_urls,
            create_time,
            update_time
        "#,
    )
    .bind(title)
    .bind(normalize_optional_text(payload.cover))
    .bind(class_name)
    .bind(payload.like_count.max(0))
    .bind(normalize_optional_text(payload.img_urls))
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to create album: {error}")))?;
    invalidate_cache_key(&state, ALBUMS_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "album", "create", vec![item.id]).await;

    Ok(Json(AdminAlbumDetailResponse { data: item }))
}

pub async fn admin_update_album(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(album_id): Path<i64>,
    Json(payload): Json<AlbumPayload>,
) -> AppResult<Json<AdminAlbumDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let title = normalize_required_text(payload.title, "title")?;
    let class_name = normalize_required_text(payload.class_name, "class")?;

    let item = sqlx::query_as::<_, AlbumItem>(
        r#"
        UPDATE album
        SET title = $2, cover = $3, class = $4, like_count = $5, img_urls = $6
        WHERE id = $1
        RETURNING
            id::bigint AS id,
            title,
            cover,
            class AS class_name,
            like_count::bigint AS like_count,
            img_urls,
            create_time,
            update_time
        "#,
    )
    .bind(album_id)
    .bind(title)
    .bind(normalize_optional_text(payload.cover))
    .bind(class_name)
    .bind(payload.like_count.max(0))
    .bind(normalize_optional_text(payload.img_urls))
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to update album: {error}")))?
    .ok_or_else(|| AppError::not_found("Album not found"))?;
    invalidate_cache_key(&state, ALBUMS_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "album", "update", vec![item.id]).await;

    Ok(Json(AdminAlbumDetailResponse { data: item }))
}

pub async fn admin_delete_album(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(album_id): Path<i64>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let result = sqlx::query("DELETE FROM album WHERE id = $1")
        .bind(album_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to delete album: {error}")))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Album not found"));
    }
    invalidate_cache_key(&state, ALBUMS_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "album", "delete", vec![album_id]).await;

    Ok(Json(AdminActionResponse {
        success: true,
        message: "Album deleted".to_string(),
    }))
}

pub async fn admin_list_pages(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> AppResult<Json<AdminPageListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let data = sqlx::query_as::<_, PageItem>(
        r#"
        SELECT
            id::bigint AS id,
            page_key,
            title,
            cover_image,
            content,
            sort::bigint AS sort,
            status::bigint AS status,
            create_time,
            update_time
        FROM singlepage
        ORDER BY sort ASC, update_time DESC, id DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list pages: {error}")))?;

    Ok(Json(AdminPageListResponse { data }))
}

pub async fn admin_create_page(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<PagePayload>,
) -> AppResult<Json<AdminPageDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let normalized = normalize_page_payload(payload)?;

    let item = sqlx::query_as::<_, PageItem>(
        r#"
        INSERT INTO singlepage (page_key, title, cover_image, content, sort, status)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
            id::bigint AS id,
            page_key,
            title,
            cover_image,
            content,
            sort::bigint AS sort,
            status::bigint AS status,
            create_time,
            update_time
        "#,
    )
    .bind(normalized.page_key)
    .bind(normalized.title)
    .bind(normalized.cover_image)
    .bind(normalized.content)
    .bind(normalized.sort)
    .bind(normalized.status)
    .fetch_one(&state.db_pool)
    .await
    .map_err(map_unique_violation("Page key already exists"))?;
    invalidate_cache_key(&state, PAGES_CACHE_KEY).await;

    Ok(Json(AdminPageDetailResponse { data: item }))
}

pub async fn admin_get_page(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(page_id): Path<i64>,
) -> AppResult<Json<AdminPageDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let item = sqlx::query_as::<_, PageItem>(
        r#"
        SELECT
            id::bigint AS id,
            page_key,
            title,
            cover_image,
            content,
            sort::bigint AS sort,
            status::bigint AS status,
            create_time,
            update_time
        FROM singlepage
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(page_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to get page: {error}")))?
    .ok_or_else(|| AppError::not_found("Standalone page not found"))?;

    Ok(Json(AdminPageDetailResponse { data: item }))
}

pub async fn admin_update_page(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(page_id): Path<i64>,
    Json(payload): Json<PagePayload>,
) -> AppResult<Json<AdminPageDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let normalized = normalize_page_payload(payload)?;

    let item = sqlx::query_as::<_, PageItem>(
        r#"
        UPDATE singlepage
        SET
            page_key = $2,
            title = $3,
            cover_image = $4,
            content = $5,
            sort = $6,
            status = $7,
            update_time = CURRENT_TIMESTAMP
        WHERE id = $1
        RETURNING
            id::bigint AS id,
            page_key,
            title,
            cover_image,
            content,
            sort::bigint AS sort,
            status::bigint AS status,
            create_time,
            update_time
        "#,
    )
    .bind(page_id)
    .bind(normalized.page_key)
    .bind(normalized.title)
    .bind(normalized.cover_image)
    .bind(normalized.content)
    .bind(normalized.sort)
    .bind(normalized.status)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(map_unique_violation("Page key already exists"))?
    .ok_or_else(|| AppError::not_found("Standalone page not found"))?;
    invalidate_cache_key(&state, PAGES_CACHE_KEY).await;

    Ok(Json(AdminPageDetailResponse { data: item }))
}

pub async fn admin_delete_page(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(page_id): Path<i64>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let result = sqlx::query("DELETE FROM singlepage WHERE id = $1")
        .bind(page_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to delete page: {error}")))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Standalone page not found"));
    }
    invalidate_cache_key(&state, PAGES_CACHE_KEY).await;

    Ok(Json(AdminActionResponse {
        success: true,
        message: "Standalone page deleted".to_string(),
    }))
}

pub async fn admin_list_projects(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> AppResult<Json<AdminProjectListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let data = sqlx::query_as::<_, ProjectItem>(
        r#"
        SELECT
            id::bigint AS id,
            title,
            cover,
            category,
            description,
            content,
            tech_stack,
            project_url,
            github_url,
            sort::bigint AS sort,
            status::bigint AS status,
            create_time,
            update_time
        FROM project
        ORDER BY sort ASC, update_time DESC, id DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list projects: {error}")))?;

    Ok(Json(AdminProjectListResponse { data }))
}

pub async fn admin_create_project(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<ProjectPayload>,
) -> AppResult<Json<AdminProjectDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let normalized = normalize_project_payload(payload)?;

    let item = sqlx::query_as::<_, ProjectItem>(
        r#"
        INSERT INTO project (
            title,
            cover,
            category,
            description,
            content,
            tech_stack,
            project_url,
            github_url,
            sort,
            status
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
        RETURNING
            id::bigint AS id,
            title,
            cover,
            category,
            description,
            content,
            tech_stack,
            project_url,
            github_url,
            sort::bigint AS sort,
            status::bigint AS status,
            create_time,
            update_time
        "#,
    )
    .bind(normalized.title)
    .bind(normalized.cover)
    .bind(normalized.category)
    .bind(normalized.description)
    .bind(normalized.content)
    .bind(normalized.tech_stack)
    .bind(normalized.project_url)
    .bind(normalized.github_url)
    .bind(normalized.sort)
    .bind(normalized.status)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to create project: {error}")))?;
    invalidate_cache_key(&state, PROJECTS_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "project", "create", vec![item.id]).await;

    Ok(Json(AdminProjectDetailResponse { data: item }))
}

pub async fn admin_update_project(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(project_id): Path<i64>,
    Json(payload): Json<ProjectPayload>,
) -> AppResult<Json<AdminProjectDetailResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let normalized = normalize_project_payload(payload)?;

    let item = sqlx::query_as::<_, ProjectItem>(
        r#"
        UPDATE project
        SET
            title = $2,
            cover = $3,
            category = $4,
            description = $5,
            content = $6,
            tech_stack = $7,
            project_url = $8,
            github_url = $9,
            sort = $10,
            status = $11,
            update_time = CURRENT_TIMESTAMP
        WHERE id = $1
        RETURNING
            id::bigint AS id,
            title,
            cover,
            category,
            description,
            content,
            tech_stack,
            project_url,
            github_url,
            sort::bigint AS sort,
            status::bigint AS status,
            create_time,
            update_time
        "#,
    )
    .bind(project_id)
    .bind(normalized.title)
    .bind(normalized.cover)
    .bind(normalized.category)
    .bind(normalized.description)
    .bind(normalized.content)
    .bind(normalized.tech_stack)
    .bind(normalized.project_url)
    .bind(normalized.github_url)
    .bind(normalized.sort)
    .bind(normalized.status)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to update project: {error}")))?
    .ok_or_else(|| AppError::not_found("Project not found"))?;
    invalidate_cache_key(&state, PROJECTS_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "project", "update", vec![item.id]).await;

    Ok(Json(AdminProjectDetailResponse { data: item }))
}

pub async fn admin_delete_project(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(project_id): Path<i64>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let result = sqlx::query("DELETE FROM project WHERE id = $1")
        .bind(project_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to delete project: {error}")))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Project not found"));
    }
    invalidate_cache_key(&state, PROJECTS_CACHE_KEY).await;
    sync_api::record_content_change_best_effort(&state, "project", "delete", vec![project_id])
        .await;

    Ok(Json(AdminActionResponse {
        success: true,
        message: "Project deleted".to_string(),
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

fn normalize_status(status: i64) -> i64 {
    if status > 0 { 1 } else { 0 }
}

fn normalize_article_payload(payload: ArticlePayload) -> AppResult<ArticlePayload> {
    Ok(ArticlePayload {
        title: normalize_required_text(payload.title, "title")?,
        article_top_image: normalize_optional_text(payload.article_top_image),
        class_name: normalize_required_text(payload.class_name, "class")?,
        read: payload.read.max(0),
        like_count: Some(payload.like_count.unwrap_or(0).max(0)),
        tag: normalize_optional_text(payload.tag),
        top: payload.top.max(0),
        status: normalize_status(payload.status),
        content: normalize_optional_text(payload.content),
    })
}

fn normalize_page_payload(payload: PagePayload) -> AppResult<PagePayload> {
    let page_key = payload.page_key.trim().trim_matches('/').to_string();
    if page_key.is_empty() {
        return Err(AppError::Unprocessable("page_key is required".to_string()));
    }

    Ok(PagePayload {
        page_key,
        title: normalize_required_text(payload.title, "title")?,
        cover_image: normalize_optional_text(payload.cover_image),
        content: normalize_optional_text(payload.content),
        sort: payload.sort,
        status: normalize_status(payload.status),
    })
}

fn normalize_project_payload(payload: ProjectPayload) -> AppResult<ProjectPayload> {
    Ok(ProjectPayload {
        title: normalize_required_text(payload.title, "title")?,
        cover: normalize_optional_text(payload.cover),
        category: normalize_optional_text(payload.category),
        description: normalize_optional_text(payload.description),
        content: normalize_optional_text(payload.content),
        tech_stack: normalize_optional_text(payload.tech_stack),
        project_url: normalize_optional_text(payload.project_url),
        github_url: normalize_optional_text(payload.github_url),
        sort: payload.sort,
        status: normalize_status(payload.status),
    })
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

async fn invalidate_cache_key(state: &AppState, key: &str) {
    state.runtime_cache.delete(key).await;
}
