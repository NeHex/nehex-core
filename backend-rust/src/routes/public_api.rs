use std::collections::{HashMap, HashSet};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{NaiveDateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

use crate::{
    error::{AppError, AppResult},
    routes::{admin_auth, admin_mail},
    state::AppState,
};

const DEFAULT_ARTICLE_PAGE_SIZE: i64 = 20;
const MAX_ARTICLE_PAGE_SIZE: i64 = 100;
const LIKE_COOKIE_MAX_ITEMS: usize = 400;
const LIKE_COOKIE_MAX_AGE_SECONDS: i64 = 60 * 60 * 24 * 365;
const ARTICLE_LIKE_COOKIE_KEY: &str = "article_liked_ids";
const COMMENT_LIKE_COOKIE_KEY: &str = "comment_liked_ids";

const COMMENT_CACHE_STATUS_PUBLISHED: i64 = 1;
const COMMENT_RATE_LIMIT_WINDOW_SECONDS: i64 = 45;
const COMMENT_RATE_LIMIT_MAX_PER_IP: i64 = 5;
const COMMENT_DUPLICATE_WINDOW_SECONDS: i64 = 600;
const COMMENT_MAX_LINKS: usize = 8;
const FRIEND_APPLY_RATE_LIMIT_SECONDS: i64 = 300;
const FRIEND_APPLY_RATE_LIMIT_PER_IP: usize = 5;
const FRIEND_APPLY_DUPLICATE_WINDOW_DAYS: i64 = 14;

const SUPPORTED_COMMENT_TARGET_TYPES: &[&str] = &["article", "album", "singlepage", "friend_page"];

const PUBLIC_VISIBLE_SETTING_KEYS: &[&str] = &[
    "site_title",
    "site_sub_title",
    "site_api_base",
    "site_description",
    "site_keywords",
    "site_icp",
    "site_notice",
    "site_url",
    "site_desc",
    "site_favicon",
    "theme_background",
    "theme_primary",
    "theme_banner",
    "theme_card_style",
    "theme_nav",
    "nehex_article_class",
    "user_social_link",
    "user_headpic",
    "admin_login_background",
];

const THEME_DETAIL_SETTING_KEYS: &[&str] = &["theme_active_profile", "theme_profiles"];
const DAILIES_CACHE_KEY: &str = "dailies:list";
const DAILIES_CACHE_TTL_SECONDS: u64 = 20;
const PROJECTS_CACHE_KEY: &str = "projects:list";
const PROJECTS_CACHE_TTL_SECONDS: u64 = 20;
const ALBUMS_CACHE_KEY: &str = "albums:list";
const ALBUMS_CACHE_TTL_SECONDS: u64 = 20;
const PAGES_CACHE_KEY: &str = "pages:list";
const PAGES_CACHE_TTL_SECONDS: u64 = 20;
const FRIENDS_CACHE_KEY: &str = "friends:list";
const FRIENDS_CACHE_TTL_SECONDS: u64 = 30;
const KUMA_MOVIE_CACHE_KEY: &str = "kuma:movie:list";
const KUMA_MOVIE_CACHE_TTL_SECONDS: u64 = 20;
const COMMENT_CACHE_TTL_SECONDS: u64 = 8;
const SETTINGS_CACHE_KEY: &str = "settings:list";
const SETTINGS_WITH_THEME_DETAILS_CACHE_KEY: &str = "settings:list:with-theme-details";
const SETTINGS_CACHE_TTL_SECONDS: u64 = 60;

static WHITESPACE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s+").expect("whitespace regex should compile"));
static LINK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)https?://|www\.").expect("link regex should compile"));

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/daily", get(get_dailies))
        .route("/project", get(get_projects))
        .route("/project/{project_id}", get(get_project_detail))
        .route("/comment", get(get_comments).post(post_comment))
        .route("/comment/{comment_id}/like", post(post_comment_like))
        .route("/setting", get(get_settings))
        .route("/setting/theme", get(get_theme_settings))
        .route("/setting/site-owner", get(get_site_owner_profile))
        .route("/site-owner", get(get_site_owner_profile))
        .route("/album", get(get_albums))
        .route("/album/{album_id}", get(get_album_detail))
        .route("/article", get(get_articles))
        .route("/article/{article_id}", get(get_article_detail))
        .route("/article/{article_id}/read", post(post_article_read))
        .route("/article/{article_id}/like", post(post_article_like))
        .route("/page", get(get_pages))
        .route("/page/{page_key}", get(get_page_detail))
        .route("/friend", get(get_friends))
        .route("/friend/apply", post(post_friend_apply))
        .route("/friend-apply", post(post_friend_apply))
        .route("/kuma/movie", get(get_kuma_movies))
}

#[derive(sqlx::FromRow)]
struct ArticleRow {
    id: i64,
    title: String,
    article_top_image: Option<String>,
    class_name: String,
    read_count: i64,
    like_count: i64,
    last_edit_time: NaiveDateTime,
    tag: Option<String>,
    top: i64,
    status: i64,
    content: Option<String>,
}

#[derive(Serialize)]
struct ArticleItem {
    id: i64,
    title: String,
    #[serde(rename = "articleTopImage")]
    article_top_image: Option<String>,
    #[serde(rename = "class")]
    class_name: String,
    #[serde(rename = "read")]
    read_count: i64,
    like_count: i64,
    #[serde(rename = "lastEditTime")]
    last_edit_time: NaiveDateTime,
    tag: Option<String>,
    top: i64,
    status: i64,
    content: Option<String>,
}

#[derive(Serialize)]
struct ArticlePagination {
    page: i64,
    size: i64,
    total: i64,
    total_pages: i64,
}

#[derive(Serialize)]
struct ArticleListResponse {
    data: Vec<ArticleItem>,
    pagination: ArticlePagination,
}

#[derive(Serialize)]
struct ArticleDetailResponse {
    data: ArticleItem,
}

#[derive(Deserialize)]
struct ArticleListQuery {
    page: Option<i64>,
    size: Option<i64>,
}

async fn get_articles(
    State(state): State<AppState>,
    Query(query): Query<ArticleListQuery>,
) -> AppResult<Json<ArticleListResponse>> {
    let page = query.page.unwrap_or(1).max(1);
    let size = query
        .size
        .unwrap_or(DEFAULT_ARTICLE_PAGE_SIZE)
        .clamp(1, MAX_ARTICLE_PAGE_SIZE);
    let offset = (page - 1) * size;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(id) FROM article WHERE status = 1")
        .fetch_one(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to count articles: {error}")))?;

    if total <= 0 {
        return Ok(Json(ArticleListResponse {
            data: Vec::new(),
            pagination: ArticlePagination {
                page,
                size,
                total: 0,
                total_pages: 0,
            },
        }));
    }

    let rows = sqlx::query_as::<_, ArticleRow>(
        r#"
        SELECT
            id::bigint AS id,
            title,
            "articleTopImage" AS article_top_image,
            class AS class_name,
            read::bigint AS read_count,
            like_count::bigint AS like_count,
            "lastEditTime" AS last_edit_time,
            tag,
            top::bigint AS top,
            status::bigint AS status,
            content
        FROM article
        WHERE status = 1
        ORDER BY top DESC, "lastEditTime" DESC, id DESC
        OFFSET $1
        LIMIT $2
        "#,
    )
    .bind(offset)
    .bind(size)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list articles: {error}")))?;

    let data = rows.into_iter().map(map_article_item).collect::<Vec<_>>();
    let total_pages = (total + size - 1) / size;

    Ok(Json(ArticleListResponse {
        data,
        pagination: ArticlePagination {
            page,
            size,
            total,
            total_pages,
        },
    }))
}

async fn get_article_detail(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> AppResult<Json<ArticleDetailResponse>> {
    let row = fetch_published_article(&state, article_id).await?;
    Ok(Json(ArticleDetailResponse {
        data: map_article_item(row),
    }))
}

async fn post_article_read(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> AppResult<Json<ArticleDetailResponse>> {
    if article_id <= 0 {
        return Err(AppError::BadRequest("Invalid article id".to_string()));
    }

    let updated = sqlx::query("UPDATE article SET read = read + 1 WHERE id = $1 AND status = 1")
        .bind(article_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to update article read count: {error}"))
        })?;

    if updated.rows_affected() == 0 {
        return Err(AppError::not_found("Article not found"));
    }

    let row = fetch_published_article(&state, article_id).await?;
    Ok(Json(ArticleDetailResponse {
        data: map_article_item(row),
    }))
}

async fn post_article_like(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(article_id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    if article_id <= 0 {
        return Err(AppError::Unprocessable("Invalid article id".to_string()));
    }

    let mut liked_ids = parse_liked_cookie(headers.get(header::COOKIE), ARTICLE_LIKE_COOKIE_KEY);
    if liked_ids.contains(&article_id) {
        return Err(AppError::Conflict("Already liked".to_string()));
    }

    let updated =
        sqlx::query("UPDATE article SET like_count = like_count + 1 WHERE id = $1 AND status = 1")
            .bind(article_id)
            .execute(&state.db_pool)
            .await
            .map_err(|error| {
                AppError::internal(format!("Failed to update article like count: {error}"))
            })?;

    if updated.rows_affected() == 0 {
        return Err(AppError::not_found("Article not found"));
    }

    liked_ids.push(article_id);
    if liked_ids.len() > LIKE_COOKIE_MAX_ITEMS {
        let drain_until = liked_ids.len().saturating_sub(LIKE_COOKIE_MAX_ITEMS);
        liked_ids.drain(0..drain_until);
    }

    let row = fetch_published_article(&state, article_id).await?;
    let cookie_value = format_liked_cookie_value(ARTICLE_LIKE_COOKIE_KEY, &liked_ids);
    Ok((
        [(header::SET_COOKIE, cookie_value)],
        Json(ArticleDetailResponse {
            data: map_article_item(row),
        }),
    ))
}

async fn fetch_published_article(state: &AppState, article_id: i64) -> AppResult<ArticleRow> {
    let row = sqlx::query_as::<_, ArticleRow>(
        r#"
        SELECT
            id::bigint AS id,
            title,
            "articleTopImage" AS article_top_image,
            class AS class_name,
            read::bigint AS read_count,
            like_count::bigint AS like_count,
            "lastEditTime" AS last_edit_time,
            tag,
            top::bigint AS top,
            status::bigint AS status,
            content
        FROM article
        WHERE id = $1 AND status = 1
        LIMIT 1
        "#,
    )
    .bind(article_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to query article detail: {error}")))?;

    row.ok_or_else(|| AppError::not_found("Article not found"))
}

fn map_article_item(row: ArticleRow) -> ArticleItem {
    ArticleItem {
        id: row.id,
        title: row.title,
        article_top_image: row.article_top_image,
        class_name: row.class_name,
        read_count: row.read_count,
        like_count: row.like_count,
        last_edit_time: row.last_edit_time,
        tag: row.tag,
        top: row.top,
        status: if row.status > 0 { 1 } else { 0 },
        content: row.content,
    }
}

#[derive(sqlx::FromRow)]
struct ProjectRow {
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

#[derive(Serialize, Deserialize, Clone)]
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
struct ProjectListResponse {
    data: Vec<ProjectItem>,
}

#[derive(Serialize)]
struct ProjectDetailResponse {
    data: ProjectItem,
}

async fn get_projects(State(state): State<AppState>) -> AppResult<Json<ProjectListResponse>> {
    if let Some(cached) = state
        .runtime_cache
        .get::<Vec<ProjectItem>>(PROJECTS_CACHE_KEY)
        .await
    {
        return Ok(Json(ProjectListResponse { data: cached }));
    }

    let rows = sqlx::query_as::<_, ProjectRow>(
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
        WHERE status = 1
        ORDER BY sort ASC, update_time DESC, id DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list projects: {error}")))?;

    let data = rows.into_iter().map(map_project_item).collect::<Vec<_>>();
    state
        .runtime_cache
        .set(PROJECTS_CACHE_KEY, data.clone(), PROJECTS_CACHE_TTL_SECONDS)
        .await;
    Ok(Json(ProjectListResponse { data }))
}

async fn get_project_detail(
    State(state): State<AppState>,
    Path(project_id): Path<i64>,
) -> AppResult<Json<ProjectDetailResponse>> {
    let row = sqlx::query_as::<_, ProjectRow>(
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
        WHERE id = $1 AND status = 1
        LIMIT 1
        "#,
    )
    .bind(project_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to query project detail: {error}")))?;

    let Some(row) = row else {
        return Err(AppError::not_found("Project not found"));
    };

    Ok(Json(ProjectDetailResponse {
        data: map_project_item(row),
    }))
}

fn map_project_item(row: ProjectRow) -> ProjectItem {
    ProjectItem {
        id: row.id,
        title: row.title,
        cover: row.cover,
        category: row.category,
        description: row.description,
        content: row.content,
        tech_stack: row.tech_stack,
        project_url: row.project_url,
        github_url: row.github_url,
        sort: row.sort,
        status: row.status,
        create_time: row.create_time,
        update_time: row.update_time,
    }
}

#[derive(sqlx::FromRow)]
struct KumaMovieRow {
    id: i64,
    provider: String,
    movie_id: String,
    watch_status: String,
    cover: Option<String>,
    title: String,
    years: Option<String>,
    score: Option<String>,
    desc: Option<String>,
    url: Option<String>,
    create_time: NaiveDateTime,
    update_time: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Clone)]
struct KumaMovieItem {
    id: i64,
    provider: String,
    movie_id: String,
    watch_status: String,
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
struct KumaMovieListResponse {
    data: Vec<KumaMovieItem>,
}

async fn get_kuma_movies(State(state): State<AppState>) -> AppResult<Json<KumaMovieListResponse>> {
    if let Some(cached) = state
        .runtime_cache
        .get::<Vec<KumaMovieItem>>(KUMA_MOVIE_CACHE_KEY)
        .await
    {
        return Ok(Json(KumaMovieListResponse { data: cached }));
    }

    let rows = sqlx::query_as::<_, KumaMovieRow>(
        r#"
        SELECT
            id::bigint AS id,
            provider,
            movie_id,
            COALESCE(watch_status, 'want') AS watch_status,
            cover,
            title,
            years,
            score,
            description AS desc,
            source_url AS url,
            create_time,
            update_time
        FROM kuma_movie
        ORDER BY d.create_time DESC, d.id DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list kuma movies: {error}")))?;

    let data = rows
        .into_iter()
        .map(|row| KumaMovieItem {
            id: row.id,
            provider: row.provider,
            movie_id: row.movie_id,
            watch_status: row.watch_status,
            cover: row.cover.unwrap_or_default(),
            title: row.title,
            years: row.years.unwrap_or_default(),
            score: row.score,
            desc: row.desc.unwrap_or_default(),
            url: row.url.unwrap_or_default(),
            create_time: row.create_time,
            update_time: row.update_time,
        })
        .collect::<Vec<_>>();

    state
        .runtime_cache
        .set(
            KUMA_MOVIE_CACHE_KEY,
            data.clone(),
            KUMA_MOVIE_CACHE_TTL_SECONDS,
        )
        .await;

    Ok(Json(KumaMovieListResponse { data }))
}

#[derive(sqlx::FromRow)]
struct DailyRow {
    id: i64,
    title: String,
    content: Option<String>,
    create_time: NaiveDateTime,
    weather: Option<String>,
    daily_type: String,
    kuma_movie_id: Option<i64>,
    movie_provider: Option<String>,
    movie_movie_id: Option<String>,
    movie_watch_status: Option<String>,
    movie_cover: Option<String>,
    movie_title: Option<String>,
    movie_years: Option<String>,
    movie_score: Option<String>,
    movie_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct DailyMovieItem {
    id: i64,
    provider: String,
    movie_id: String,
    watch_status: String,
    cover: String,
    title: String,
    years: String,
    score: Option<String>,
    url: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct DailyItem {
    id: i64,
    title: String,
    content: Option<String>,
    create_time: NaiveDateTime,
    weather: Option<String>,
    daily_type: String,
    kuma_movie_id: Option<i64>,
    movie: Option<DailyMovieItem>,
}

#[derive(Serialize)]
struct DailyListResponse {
    data: Vec<DailyItem>,
}

async fn get_dailies(State(state): State<AppState>) -> AppResult<Json<DailyListResponse>> {
    if let Some(cached) = state
        .runtime_cache
        .get::<Vec<DailyItem>>(DAILIES_CACHE_KEY)
        .await
    {
        return Ok(Json(DailyListResponse { data: cached }));
    }

    let rows = sqlx::query_as::<_, DailyRow>(
        r#"
        SELECT
            d.id::bigint AS id,
            d.title,
            d.content,
            d.create_time,
            d.weather,
            COALESCE(d.daily_type, 'note') AS daily_type,
            d.kuma_movie_id::bigint AS kuma_movie_id,
            km.provider AS movie_provider,
            km.movie_id AS movie_movie_id,
            COALESCE(km.watch_status, 'want') AS movie_watch_status,
            km.cover AS movie_cover,
            km.title AS movie_title,
            km.years AS movie_years,
            km.score AS movie_score,
            km.source_url AS movie_url
        FROM daily d
        LEFT JOIN kuma_movie km ON km.id = d.kuma_movie_id
        ORDER BY create_time DESC, id DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list dailies: {error}")))?;

    let data = rows
        .into_iter()
        .map(|row| DailyItem {
            id: row.id,
            title: row.title,
            content: row.content,
            create_time: row.create_time,
            weather: row.weather,
            daily_type: row.daily_type,
            kuma_movie_id: row.kuma_movie_id,
            movie: row.kuma_movie_id.map(|movie_id| DailyMovieItem {
                id: movie_id,
                provider: row.movie_provider.unwrap_or_default(),
                movie_id: row.movie_movie_id.unwrap_or_default(),
                watch_status: row.movie_watch_status.unwrap_or_else(|| "want".to_string()),
                cover: row.movie_cover.unwrap_or_default(),
                title: row.movie_title.unwrap_or_default(),
                years: row.movie_years.unwrap_or_default(),
                score: row.movie_score,
                url: row.movie_url.unwrap_or_default(),
            }),
        })
        .collect::<Vec<_>>();

    state
        .runtime_cache
        .set(DAILIES_CACHE_KEY, data.clone(), DAILIES_CACHE_TTL_SECONDS)
        .await;
    Ok(Json(DailyListResponse { data }))
}

#[derive(sqlx::FromRow)]
struct AlbumRow {
    id: i64,
    title: String,
    cover: Option<String>,
    class_name: String,
    like_count: i64,
    img_urls: Option<String>,
    create_time: NaiveDateTime,
    update_time: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Clone)]
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
struct AlbumListResponse {
    data: Vec<AlbumItem>,
}

#[derive(Serialize)]
struct AlbumDetailResponse {
    data: AlbumItem,
}

async fn get_albums(State(state): State<AppState>) -> AppResult<Json<AlbumListResponse>> {
    if let Some(cached) = state
        .runtime_cache
        .get::<Vec<AlbumItem>>(ALBUMS_CACHE_KEY)
        .await
    {
        return Ok(Json(AlbumListResponse { data: cached }));
    }

    let rows = sqlx::query_as::<_, AlbumRow>(
        r#"
        SELECT
            id::bigint AS id,
            title,
            cover,
            class AS class_name,
            like_count::bigint AS like_count,
            img_urls,
            create_time,
            update_time
        FROM album
        ORDER BY update_time DESC, id DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list albums: {error}")))?;

    let data = rows.into_iter().map(map_album_item).collect::<Vec<_>>();
    state
        .runtime_cache
        .set(ALBUMS_CACHE_KEY, data.clone(), ALBUMS_CACHE_TTL_SECONDS)
        .await;
    Ok(Json(AlbumListResponse { data }))
}

async fn get_album_detail(
    State(state): State<AppState>,
    Path(album_id): Path<i64>,
) -> AppResult<Json<AlbumDetailResponse>> {
    let row = sqlx::query_as::<_, AlbumRow>(
        r#"
        SELECT
            id::bigint AS id,
            title,
            cover,
            class AS class_name,
            like_count::bigint AS like_count,
            img_urls,
            create_time,
            update_time
        FROM album
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(album_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to query album detail: {error}")))?;

    let Some(row) = row else {
        return Err(AppError::not_found("Album not found"));
    };

    Ok(Json(AlbumDetailResponse {
        data: map_album_item(row),
    }))
}

fn map_album_item(row: AlbumRow) -> AlbumItem {
    AlbumItem {
        id: row.id,
        title: row.title,
        cover: row.cover,
        class_name: row.class_name,
        like_count: row.like_count,
        img_urls: row.img_urls,
        create_time: row.create_time,
        update_time: row.update_time,
    }
}

#[derive(sqlx::FromRow)]
struct PageRow {
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

#[derive(Serialize, Deserialize, Clone)]
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
struct PageListResponse {
    data: Vec<PageItem>,
}

#[derive(Serialize)]
struct PageDetailResponse {
    data: PageItem,
}

async fn get_pages(State(state): State<AppState>) -> AppResult<Json<PageListResponse>> {
    if let Some(cached) = state
        .runtime_cache
        .get::<Vec<PageItem>>(PAGES_CACHE_KEY)
        .await
    {
        return Ok(Json(PageListResponse { data: cached }));
    }

    let rows = sqlx::query_as::<_, PageRow>(
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
        WHERE status = 1
        ORDER BY sort ASC, update_time DESC, id DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list pages: {error}")))?;

    let data = rows.into_iter().map(map_page_item).collect::<Vec<_>>();
    state
        .runtime_cache
        .set(PAGES_CACHE_KEY, data.clone(), PAGES_CACHE_TTL_SECONDS)
        .await;
    Ok(Json(PageListResponse { data }))
}

async fn get_page_detail(
    State(state): State<AppState>,
    Path(page_key): Path<String>,
) -> AppResult<Json<PageDetailResponse>> {
    let normalized_key = page_key.trim().trim_matches('/').to_string();
    if normalized_key.is_empty() {
        return Err(AppError::not_found("Page not found"));
    }

    let row = sqlx::query_as::<_, PageRow>(
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
        WHERE page_key = $1 AND status = 1
        LIMIT 1
        "#,
    )
    .bind(normalized_key)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to query page detail: {error}")))?;

    let Some(row) = row else {
        return Err(AppError::not_found("Page not found"));
    };

    Ok(Json(PageDetailResponse {
        data: map_page_item(row),
    }))
}

fn map_page_item(row: PageRow) -> PageItem {
    PageItem {
        id: row.id,
        page_key: row.page_key,
        title: row.title,
        cover_image: row.cover_image,
        content: row.content,
        sort: row.sort,
        status: row.status,
        create_time: row.create_time,
        update_time: row.update_time,
    }
}

#[derive(sqlx::FromRow)]
struct SettingRow {
    setting_key: String,
    setting_type: String,
    setting_content: Option<String>,
    description: Option<String>,
    updated_at: NaiveDateTime,
    created_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Clone)]
struct SettingItem {
    setting_key: String,
    setting_type: String,
    setting_content: Value,
    description: Option<String>,
    updated_at: NaiveDateTime,
    created_at: NaiveDateTime,
}

#[derive(Serialize)]
struct SettingListResponse {
    data: Vec<SettingItem>,
}

#[derive(Serialize, Clone)]
struct ThemeSettingData {
    active_profile: String,
    profiles: HashMap<String, Value>,
    current: Value,
}

#[derive(Serialize)]
struct ThemeSettingResponse {
    data: ThemeSettingData,
}

#[derive(Serialize, Clone)]
struct SiteOwnerProfileData {
    avatar: String,
    nickname: String,
    homepage: String,
    email: String,
    bio: String,
}

#[derive(Serialize)]
struct SiteOwnerProfileResponse {
    data: SiteOwnerProfileData,
}

async fn get_settings(State(state): State<AppState>) -> AppResult<Json<SettingListResponse>> {
    let data = list_settings(&state, false).await;
    Ok(Json(SettingListResponse { data }))
}

async fn get_theme_settings(
    State(state): State<AppState>,
) -> AppResult<Json<ThemeSettingResponse>> {
    let setting_items = list_settings(&state, true).await;
    let setting_map = setting_items
        .iter()
        .map(|item| (item.setting_key.clone(), item.clone()))
        .collect::<HashMap<_, _>>();

    let default_rei = default_rei_theme_profile();

    let raw_profiles = setting_map
        .get("theme_profiles")
        .map(|item| item.setting_content.clone())
        .unwrap_or_else(|| json!({}));

    let mut profiles = parse_theme_profiles(raw_profiles);
    if let Some(profile) = profiles.get_mut("rei.json") {
        merge_profile_defaults(profile, &default_rei);
    } else {
        profiles.insert("rei.json".to_string(), default_rei.clone());
    }

    let active_profile = setting_map
        .get("theme_active_profile")
        .and_then(|item| {
            item.setting_content
                .as_str()
                .map(|value| normalize_theme_file_name(value))
        })
        .filter(|value| profiles.contains_key(value))
        .unwrap_or_else(|| {
            if profiles.contains_key("rei.json") {
                "rei.json".to_string()
            } else {
                profiles
                    .keys()
                    .next()
                    .cloned()
                    .unwrap_or_else(|| "rei.json".to_string())
            }
        });

    let current = profiles
        .get(&active_profile)
        .cloned()
        .unwrap_or_else(|| default_rei.clone());

    Ok(Json(ThemeSettingResponse {
        data: ThemeSettingData {
            active_profile,
            profiles,
            current,
        },
    }))
}

fn default_rei_theme_profile() -> Value {
    serde_json::from_str(include_str!("../defaults/rei_theme.json")).unwrap_or_else(|_| {
        json!({
            "head_pic": "/images/head.jpg",
            "background_images": "/images/background-2k.png",
            "headmsg": "hi"
        })
    })
}

fn merge_profile_defaults(target: &mut Value, defaults: &Value) {
    let Some(target_obj) = target.as_object_mut() else {
        return;
    };
    let Some(default_obj) = defaults.as_object() else {
        return;
    };

    for (key, default_value) in default_obj {
        if let Some(target_value) = target_obj.get_mut(key) {
            merge_profile_defaults(target_value, default_value);
        } else {
            target_obj.insert(key.clone(), default_value.clone());
        }
    }
}

fn parse_theme_profiles(raw: Value) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    let source = if let Some(themes) = raw.get("themes") {
        themes
    } else {
        &raw
    };

    if let Some(map) = source.as_object() {
        for (key, value) in map {
            let normalized = normalize_theme_file_name(key);
            if normalized.is_empty() || !value.is_object() {
                continue;
            }
            result.insert(normalized, value.clone());
        }
    }

    result
}

fn normalize_theme_file_name(value: &str) -> String {
    let text = value.trim();
    if text.is_empty() || text.contains('/') || text.contains('\\') {
        return String::new();
    }
    if text.contains('.') {
        text.to_string()
    } else {
        format!("{text}.json")
    }
}

async fn get_site_owner_profile(
    State(state): State<AppState>,
) -> AppResult<Json<SiteOwnerProfileResponse>> {
    let rows = sqlx::query_as::<_, SettingRow>(
        r#"
        SELECT
            setting_key,
            setting_type::text AS setting_type,
            setting_content,
            description,
            updated_at,
            created_at
        FROM settings
        WHERE setting_key = ANY($1)
        "#,
    )
    .bind(vec![
        "site_owner_avatar",
        "site_owner_nickname",
        "site_owner_homepage",
        "site_owner_email",
        "site_owner_bio",
        "site_url",
    ])
    .fetch_all(&state.db_pool)
    .await
    .unwrap_or_default();

    let mut map = HashMap::<String, SettingItem>::new();
    for row in rows {
        let item = map_setting_item(row);
        map.insert(item.setting_key.clone(), item);
    }

    let avatar = read_setting_text(&map, "site_owner_avatar", "/images/head.jpg");
    let nickname = read_setting_text(&map, "site_owner_nickname", "站长");
    let mut homepage = read_setting_text(&map, "site_owner_homepage", "");
    if homepage.is_empty() {
        homepage = read_setting_text(&map, "site_url", "");
    }

    let email = read_setting_text(&map, "site_owner_email", "");
    let bio = read_setting_text(&map, "site_owner_bio", "");

    Ok(Json(SiteOwnerProfileResponse {
        data: SiteOwnerProfileData {
            avatar,
            nickname,
            homepage,
            email,
            bio,
        },
    }))
}

fn read_setting_text(map: &HashMap<String, SettingItem>, key: &str, fallback: &str) -> String {
    map.get(key)
        .and_then(|item| match &item.setting_content {
            Value::String(value) => Some(value.trim().to_string()),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

async fn list_settings(state: &AppState, include_theme_details: bool) -> Vec<SettingItem> {
    let cache_key = if include_theme_details {
        SETTINGS_WITH_THEME_DETAILS_CACHE_KEY
    } else {
        SETTINGS_CACHE_KEY
    };

    if let Some(cached) = state.runtime_cache.get::<Vec<SettingItem>>(cache_key).await {
        return cached;
    }

    let key_list = build_setting_key_list(include_theme_details);
    let rows = sqlx::query_as::<_, SettingRow>(
        r#"
        SELECT
            setting_key,
            setting_type::text AS setting_type,
            setting_content,
            description,
            updated_at,
            created_at
        FROM settings
        WHERE setting_key = ANY($1)
        ORDER BY setting_key ASC
        "#,
    )
    .bind(key_list)
    .fetch_all(&state.db_pool)
    .await;

    let now = Utc::now().naive_utc();
    let mut map = HashMap::<String, SettingItem>::new();

    if let Ok(rows) = rows {
        for row in rows {
            let item = map_setting_item(row);
            map.insert(item.setting_key.clone(), item);
        }
    }

    merge_compat_defaults(&mut map, now);

    let mut items = map.into_values().collect::<Vec<_>>();
    items.sort_by(|left, right| left.setting_key.cmp(&right.setting_key));
    state
        .runtime_cache
        .set(cache_key, items.clone(), SETTINGS_CACHE_TTL_SECONDS)
        .await;
    items
}

fn build_setting_key_filter(include_theme_details: bool) -> HashSet<&'static str> {
    let mut keys = PUBLIC_VISIBLE_SETTING_KEYS
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    if include_theme_details {
        keys.extend(THEME_DETAIL_SETTING_KEYS.iter().copied());
    }
    keys
}

fn build_setting_key_list(include_theme_details: bool) -> Vec<&'static str> {
    let mut keys = build_setting_key_filter(include_theme_details)
        .into_iter()
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys
}

fn merge_compat_defaults(map: &mut HashMap<String, SettingItem>, now: NaiveDateTime) {
    let defaults = vec![
        ("site_title", "string", Value::String(String::new())),
        (
            "site_desc",
            "string",
            map.get("site_description")
                .map(|item| item.setting_content.clone())
                .unwrap_or_else(|| Value::String(String::new())),
        ),
        (
            "site_favicon",
            "string",
            Value::String("/favicon.ico".to_string()),
        ),
        ("site_url", "string", Value::String(String::new())),
        ("theme_background", "string", Value::String(String::new())),
        ("theme_nav", "json", json!({})),
        ("user_social_link", "json", json!([])),
        (
            "user_headpic",
            "string",
            Value::String("/images/head.jpg".to_string()),
        ),
        (
            "admin_login_background",
            "string",
            Value::String("/images/background-2k.png".to_string()),
        ),
    ];

    for (setting_key, setting_type, setting_content) in defaults {
        map.entry(setting_key.to_string())
            .or_insert_with(|| SettingItem {
                setting_key: setting_key.to_string(),
                setting_type: setting_type.to_string(),
                setting_content,
                description: Some("compat default".to_string()),
                updated_at: now,
                created_at: now,
            });
    }
}

fn map_setting_item(row: SettingRow) -> SettingItem {
    SettingItem {
        setting_key: row.setting_key,
        setting_type: row.setting_type.clone(),
        setting_content: parse_setting_content(&row.setting_type, row.setting_content.as_deref()),
        description: row.description,
        updated_at: row.updated_at,
        created_at: row.created_at,
    }
}

fn parse_setting_content(setting_type: &str, raw_content: Option<&str>) -> Value {
    let Some(raw_content) = raw_content else {
        return Value::Null;
    };

    match setting_type {
        "string" => Value::String(raw_content.to_string()),
        "int" => raw_content
            .trim()
            .parse::<i64>()
            .map(|value| Value::Number(value.into()))
            .unwrap_or_else(|_| Value::String(raw_content.to_string())),
        "float" => raw_content
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or_else(|| Value::String(raw_content.to_string())),
        "boolean" => Value::Bool(matches!(
            raw_content.trim().to_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )),
        "json" => serde_json::from_str(raw_content)
            .unwrap_or_else(|_| Value::String(raw_content.to_string())),
        _ => Value::String(raw_content.to_string()),
    }
}

#[derive(Deserialize)]
struct CommentListQuery {
    target_type: String,
    target_id: i64,
    status: Option<i64>,
}

#[derive(Deserialize)]
struct CommentCreateRequest {
    parent_id: Option<i64>,
    target_type: String,
    target_id: i64,
    content: String,
    nickname: String,
    email: Option<String>,
    website: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CommentRow {
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
    is_admin: i64,
    ip: Option<String>,
    create_time: NaiveDateTime,
    update_time: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Clone)]
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
    is_admin: bool,
    ip: Option<String>,
    create_time: NaiveDateTime,
    update_time: NaiveDateTime,
    replies: Vec<CommentItem>,
}

#[derive(Serialize)]
struct CommentListResponse {
    data: Vec<CommentItem>,
}

#[derive(Serialize)]
struct CommentDetailResponse {
    data: CommentItem,
}

async fn get_comments(
    State(state): State<AppState>,
    Query(query): Query<CommentListQuery>,
) -> AppResult<Json<CommentListResponse>> {
    let target_type = query.target_type.trim().to_lowercase();
    if !SUPPORTED_COMMENT_TARGET_TYPES.contains(&target_type.as_str()) {
        return Err(AppError::Unprocessable(
            "Unsupported target_type".to_string(),
        ));
    }
    if query.target_id <= 0 {
        return Err(AppError::Unprocessable("Invalid target_id".to_string()));
    }

    let status = query.status.unwrap_or(COMMENT_CACHE_STATUS_PUBLISHED);
    if !(0..=1).contains(&status) {
        return Err(AppError::Unprocessable("Invalid status".to_string()));
    }
    let cache_key = format!("comments:list:{target_type}:{}:{status}", query.target_id);
    if let Some(cached) = state
        .runtime_cache
        .get::<Vec<CommentItem>>(&cache_key)
        .await
    {
        return Ok(Json(CommentListResponse { data: cached }));
    }

    let rows = sqlx::query_as::<_, CommentRow>(
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
            is_admin::bigint AS is_admin,
            ip,
            create_time,
            update_time
        FROM comment
        WHERE target_type = $1 AND target_id = $2 AND status = $3
        ORDER BY create_time ASC, id ASC
        "#,
    )
    .bind(target_type)
    .bind(query.target_id)
    .bind(status)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list comments: {error}")))?;

    let items = rows.into_iter().map(map_comment_item).collect::<Vec<_>>();
    let tree = build_comment_tree(items);
    state
        .runtime_cache
        .set(cache_key, tree.clone(), COMMENT_CACHE_TTL_SECONDS)
        .await;
    Ok(Json(CommentListResponse { data: tree }))
}

async fn post_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CommentCreateRequest>,
) -> AppResult<Json<CommentDetailResponse>> {
    let ip_address = resolve_client_ip(&headers);
    let marker_header = headers
        .get("x-nehex-admin-marker")
        .and_then(|value| value.to_str().ok());
    let is_admin = admin_auth::resolve_admin_identity_from_headers(&state, &headers, marker_header);

    let normalized = normalize_comment_create_payload(payload)?;
    validate_comment_parent(&state, &normalized).await?;
    validate_comment_payload(&state, &normalized, ip_address.as_deref()).await?;
    let comment_cache_prefix = format!(
        "comments:list:{}:{}:",
        normalized.target_type, normalized.target_id
    );

    let row = sqlx::query_as::<_, CommentRow>(
        r#"
        INSERT INTO comment (
            parent_id,
            target_type,
            target_id,
            content,
            nickname,
            email,
            website,
            is_admin,
            ip
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
            is_admin::bigint AS is_admin,
            ip,
            create_time,
            update_time
        "#,
    )
    .bind(normalized.parent_id)
    .bind(normalized.target_type)
    .bind(normalized.target_id)
    .bind(normalized.content)
    .bind(normalized.nickname)
    .bind(normalized.email)
    .bind(normalized.website)
    .bind(if is_admin { 1_i64 } else { 0_i64 })
    .bind(ip_address.map(|value| truncate_text(value, 50)))
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to create comment: {error}")))?;

    let notice_comment = admin_mail::MailNoticeCommentContext {
        id: row.id,
        parent_id: row.parent_id,
        target_type: row.target_type.clone(),
        target_id: row.target_id,
        nickname: row.nickname.clone(),
        email: row.email.clone(),
        content: row.content.clone(),
        create_time: row.create_time,
    };
    admin_mail::spawn_comment_notification_mails(state.clone(), notice_comment);

    state
        .runtime_cache
        .delete_prefix(&comment_cache_prefix)
        .await;

    Ok(Json(CommentDetailResponse {
        data: map_comment_item(row),
    }))
}

async fn post_comment_like(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(comment_id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    if comment_id <= 0 {
        return Err(AppError::Unprocessable("Invalid comment id".to_string()));
    }

    let mut liked_ids = parse_liked_cookie(headers.get(header::COOKIE), COMMENT_LIKE_COOKIE_KEY);
    if liked_ids.contains(&comment_id) {
        return Err(AppError::Conflict("Already liked".to_string()));
    }

    let updated =
        sqlx::query("UPDATE comment SET like_count = like_count + 1 WHERE id = $1 AND status = 1")
            .bind(comment_id)
            .execute(&state.db_pool)
            .await
            .map_err(|error| AppError::internal(format!("Failed to like comment: {error}")))?;

    if updated.rows_affected() == 0 {
        return Err(AppError::not_found("Comment not found"));
    }

    let row = sqlx::query_as::<_, CommentRow>(
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
            is_admin::bigint AS is_admin,
            ip,
            create_time,
            update_time
        FROM comment
        WHERE id = $1 AND status = 1
        LIMIT 1
        "#,
    )
    .bind(comment_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to query liked comment: {error}")))?;

    let Some(row) = row else {
        return Err(AppError::not_found("Comment not found"));
    };

    liked_ids.push(comment_id);
    if liked_ids.len() > LIKE_COOKIE_MAX_ITEMS {
        let drain_until = liked_ids.len().saturating_sub(LIKE_COOKIE_MAX_ITEMS);
        liked_ids.drain(0..drain_until);
    }

    let cookie_value = format_liked_cookie_value(COMMENT_LIKE_COOKIE_KEY, &liked_ids);
    Ok((
        [(header::SET_COOKIE, cookie_value)],
        Json(CommentDetailResponse {
            data: map_comment_item(row),
        }),
    ))
}

fn map_comment_item(row: CommentRow) -> CommentItem {
    CommentItem {
        id: row.id,
        parent_id: row.parent_id,
        target_type: row.target_type,
        target_id: row.target_id,
        content: row.content,
        nickname: row.nickname,
        email: row.email,
        website: row.website,
        like_count: row.like_count,
        status: row.status,
        is_admin: row.is_admin > 0,
        ip: row.ip,
        create_time: row.create_time,
        update_time: row.update_time,
        replies: Vec::new(),
    }
}

fn build_comment_tree(items: Vec<CommentItem>) -> Vec<CommentItem> {
    let mut by_id = HashMap::<i64, CommentItem>::new();
    let mut ordered_ids = Vec::<i64>::new();
    for item in items {
        ordered_ids.push(item.id);
        by_id.insert(item.id, item);
    }

    let mut children = HashMap::<i64, Vec<i64>>::new();
    let mut roots = Vec::<i64>::new();
    for id in &ordered_ids {
        let parent_id = by_id.get(id).map(|item| item.parent_id).unwrap_or(0);
        if parent_id == 0 || !by_id.contains_key(&parent_id) {
            roots.push(*id);
        } else {
            children.entry(parent_id).or_default().push(*id);
        }
    }

    fn build_subtree(
        id: i64,
        by_id: &HashMap<i64, CommentItem>,
        children: &HashMap<i64, Vec<i64>>,
    ) -> Option<CommentItem> {
        let mut item = by_id.get(&id)?.clone();
        let child_ids = children.get(&id).cloned().unwrap_or_default();
        item.replies = child_ids
            .into_iter()
            .filter_map(|child_id| build_subtree(child_id, by_id, children))
            .collect::<Vec<_>>();
        Some(item)
    }

    roots
        .into_iter()
        .filter_map(|id| build_subtree(id, &by_id, &children))
        .collect::<Vec<_>>()
}

struct NormalizedCommentCreatePayload {
    parent_id: i64,
    target_type: String,
    target_id: i64,
    content: String,
    nickname: String,
    email: Option<String>,
    website: Option<String>,
}

fn normalize_comment_create_payload(
    payload: CommentCreateRequest,
) -> Result<NormalizedCommentCreatePayload, AppError> {
    let parent_id = payload.parent_id.unwrap_or(0);
    if parent_id < 0 {
        return Err(AppError::Unprocessable(
            "parent_id must be >= 0".to_string(),
        ));
    }

    let target_type = payload.target_type.trim().to_lowercase();
    if target_type.is_empty() || target_type.len() > 20 {
        return Err(AppError::Unprocessable(
            "Unsupported target_type".to_string(),
        ));
    }
    if !SUPPORTED_COMMENT_TARGET_TYPES.contains(&target_type.as_str()) {
        return Err(AppError::Unprocessable(
            "Unsupported target_type".to_string(),
        ));
    }

    if payload.target_id <= 0 {
        return Err(AppError::Unprocessable(
            "target_id must be >= 1".to_string(),
        ));
    }

    let content = normalize_comment_content(&payload.content);
    if content.is_empty() || content.chars().count() > 1200 {
        return Err(AppError::Unprocessable("content is required".to_string()));
    }

    let nickname = payload.nickname.trim().to_string();
    if nickname.is_empty() || nickname.chars().count() > 100 {
        return Err(AppError::Unprocessable("nickname is required".to_string()));
    }

    let email = normalize_optional_text(payload.email, 255)?;
    let website = normalize_optional_text(payload.website, 255)?;

    Ok(NormalizedCommentCreatePayload {
        parent_id,
        target_type,
        target_id: payload.target_id,
        content,
        nickname,
        email,
        website,
    })
}

fn normalize_comment_content(value: &str) -> String {
    let value = value.replace("\r\n", "\n");
    WHITESPACE_RE.replace_all(&value, " ").trim().to_string()
}

fn normalize_optional_text(
    value: Option<String>,
    max_len: usize,
) -> Result<Option<String>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Ok(None);
    }
    if normalized.chars().count() > max_len {
        return Err(AppError::Unprocessable(format!(
            "Field length must be <= {max_len}"
        )));
    }
    Ok(Some(normalized))
}

async fn validate_comment_parent(
    state: &AppState,
    payload: &NormalizedCommentCreatePayload,
) -> AppResult<()> {
    if payload.parent_id <= 0 {
        return Ok(());
    }

    let parent_exists = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT id::bigint
        FROM comment
        WHERE id = $1 AND target_type = $2 AND target_id = $3
        LIMIT 1
        "#,
    )
    .bind(payload.parent_id)
    .bind(&payload.target_type)
    .bind(payload.target_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to validate parent comment: {error}")))?;

    if parent_exists.is_none() {
        return Err(AppError::Unprocessable(
            "Invalid parent_id for this target".to_string(),
        ));
    }
    Ok(())
}

async fn validate_comment_payload(
    state: &AppState,
    payload: &NormalizedCommentCreatePayload,
    ip_address: Option<&str>,
) -> AppResult<()> {
    let lowered = format!(" {} ", payload.content.to_lowercase());
    for pattern in [
        "<script",
        "</script",
        "javascript:",
        "onerror=",
        "onload=",
        " union select ",
        " drop table ",
        " delete from ",
    ] {
        if lowered.contains(pattern) {
            return Err(AppError::BadRequest(
                "Comment contains blocked content".to_string(),
            ));
        }
    }

    if looks_like_spam(&payload.content) {
        return Err(AppError::BadRequest("Comment looks like spam".to_string()));
    }

    let now = Utc::now().naive_utc();
    if let Some(ip_address) = ip_address {
        let cutoff = now - chrono::TimeDelta::seconds(COMMENT_RATE_LIMIT_WINDOW_SECONDS);
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(id) FROM comment WHERE ip = $1 AND create_time >= $2")
                .bind(ip_address)
                .bind(cutoff)
                .fetch_one(&state.db_pool)
                .await
                .map_err(|error| {
                    AppError::internal(format!("Failed to check comment rate limit: {error}"))
                })?;

        if count >= COMMENT_RATE_LIMIT_MAX_PER_IP {
            return Err(AppError::TooManyRequests(
                "Too many comment submissions, please try later".to_string(),
            ));
        }
    }

    let duplicate_cutoff = now - chrono::TimeDelta::seconds(COMMENT_DUPLICATE_WINDOW_SECONDS);
    let duplicate_exists = if let Some(ip_address) = ip_address {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id::bigint
            FROM comment
            WHERE
                target_type = $1
                AND target_id = $2
                AND parent_id = $3
                AND nickname = $4
                AND content = $5
                AND create_time >= $6
                AND ip = $7
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .bind(&payload.target_type)
        .bind(payload.target_id)
        .bind(payload.parent_id)
        .bind(&payload.nickname)
        .bind(&payload.content)
        .bind(duplicate_cutoff)
        .bind(ip_address)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to check duplicate comment: {error}")))?
        .is_some()
    } else {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id::bigint
            FROM comment
            WHERE
                target_type = $1
                AND target_id = $2
                AND parent_id = $3
                AND nickname = $4
                AND content = $5
                AND create_time >= $6
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .bind(&payload.target_type)
        .bind(payload.target_id)
        .bind(payload.parent_id)
        .bind(&payload.nickname)
        .bind(&payload.content)
        .bind(duplicate_cutoff)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to check duplicate comment: {error}")))?
        .is_some()
    };

    if duplicate_exists {
        return Err(AppError::Conflict("Duplicate comment detected".to_string()));
    }

    Ok(())
}

fn looks_like_spam(content: &str) -> bool {
    if has_long_repeated_char_run(content, 40) {
        return true;
    }
    let unique_count = content.chars().collect::<HashSet<_>>().len();
    if content.chars().count() >= 30 && unique_count <= 3 {
        return true;
    }
    LINK_RE.find_iter(content).count() > COMMENT_MAX_LINKS
}

fn has_long_repeated_char_run(content: &str, min_run: usize) -> bool {
    if min_run <= 1 {
        return !content.is_empty();
    }

    let mut prev: Option<char> = None;
    let mut run_len = 0usize;

    for ch in content.chars() {
        if Some(ch) == prev {
            run_len += 1;
        } else {
            prev = Some(ch);
            run_len = 1;
        }

        if run_len >= min_run {
            return true;
        }
    }

    false
}

#[derive(sqlx::FromRow)]
struct FriendRow {
    id: i64,
    title: String,
    description: Option<String>,
    category: String,
    favicon: Option<String>,
    url: String,
    status: String,
    create_time: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize)]
struct FriendListResponse {
    data: Vec<FriendItem>,
}

#[derive(Deserialize)]
struct FriendApplyRequest {
    site_title: String,
    site_url: String,
    site_description: Option<String>,
    site_icon: Option<String>,
    contact: Option<String>,
}

#[derive(Serialize)]
struct FriendApplyResponse {
    success: bool,
    message: String,
    application_id: i64,
}

async fn get_friends(State(state): State<AppState>) -> AppResult<Json<FriendListResponse>> {
    if let Some(cached) = state
        .runtime_cache
        .get::<Vec<FriendItem>>(FRIENDS_CACHE_KEY)
        .await
    {
        return Ok(Json(FriendListResponse { data: cached }));
    }

    let rows = sqlx::query_as::<_, FriendRow>(
        r#"
        SELECT
            id::bigint AS id,
            title,
            description,
            category,
            favicon,
            url,
            status::text AS status,
            create_time
        FROM friends
        ORDER BY
            CASE
                WHEN status::text = 'ok' THEN 0
                WHEN status::text = 'missing' THEN 1
                WHEN status::text = 'blocked' THEN 2
                ELSE 3
            END,
            create_time DESC,
            id DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list friends: {error}")))?;

    let data = rows
        .into_iter()
        .map(|row| FriendItem {
            id: row.id,
            title: row.title,
            description: row.description,
            category: row.category,
            favicon: row.favicon,
            url: row.url,
            status: normalize_friend_status(&row.status),
            create_time: row.create_time,
        })
        .collect::<Vec<_>>();

    state
        .runtime_cache
        .set(FRIENDS_CACHE_KEY, data.clone(), FRIENDS_CACHE_TTL_SECONDS)
        .await;
    Ok(Json(FriendListResponse { data }))
}

async fn post_friend_apply(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<FriendApplyRequest>,
) -> AppResult<Json<FriendApplyResponse>> {
    let payload = normalize_friend_apply_payload(payload)?;
    let ip_address = resolve_client_ip(&headers);
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let exists_in_friends =
        sqlx::query_scalar::<_, i64>("SELECT id::bigint FROM friends WHERE url = $1 LIMIT 1")
            .bind(&payload.site_url)
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|error| {
                AppError::internal(format!("Failed to check existing friend: {error}"))
            })?;
    if exists_in_friends.is_some() {
        return Err(AppError::Conflict(
            "This site already exists in friend list".to_string(),
        ));
    }

    let duplicate_cutoff =
        Utc::now().naive_utc() - chrono::TimeDelta::days(FRIEND_APPLY_DUPLICATE_WINDOW_DAYS);
    let duplicate_apply = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT id::bigint
        FROM friend_apply
        WHERE
            site_url = $1
            AND create_time >= $2
            AND status::text = ANY($3)
        ORDER BY id DESC
        LIMIT 1
        "#,
    )
    .bind(&payload.site_url)
    .bind(duplicate_cutoff)
    .bind(vec!["pending", "approved"])
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| {
        AppError::internal(format!(
            "Failed to check duplicate friend application: {error}"
        ))
    })?;

    if duplicate_apply.is_some() {
        return Err(AppError::Conflict(
            "This site has already submitted an application recently".to_string(),
        ));
    }

    if let Some(ip) = ip_address.as_deref() {
        let rate_limit_cutoff =
            Utc::now().naive_utc() - chrono::TimeDelta::seconds(FRIEND_APPLY_RATE_LIMIT_SECONDS);
        let recent_ids = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id::bigint
            FROM friend_apply
            WHERE ip = $1 AND create_time >= $2
            ORDER BY id DESC
            LIMIT $3
            "#,
        )
        .bind(ip)
        .bind(rate_limit_cutoff)
        .bind(FRIEND_APPLY_RATE_LIMIT_PER_IP as i64)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to check friend apply rate limit: {error}"))
        })?;

        if recent_ids.len() >= FRIEND_APPLY_RATE_LIMIT_PER_IP {
            return Err(AppError::TooManyRequests(
                "Too many friend applications, please try later".to_string(),
            ));
        }
    }

    let application_id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO friend_apply (
            site_title,
            site_url,
            site_description,
            site_icon,
            contact,
            status,
            ip,
            user_agent
        )
        VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7)
        RETURNING id::bigint
        "#,
    )
    .bind(payload.site_title)
    .bind(payload.site_url)
    .bind(payload.site_description)
    .bind(payload.site_icon)
    .bind(payload.contact)
    .bind(ip_address.map(|value| truncate_text(value, 50)))
    .bind(user_agent.map(|value| truncate_text(value, 255)))
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to create friend application: {error}")))?;

    Ok(Json(FriendApplyResponse {
        success: true,
        message: "Friend application submitted".to_string(),
        application_id,
    }))
}

struct NormalizedFriendApplyPayload {
    site_title: String,
    site_url: String,
    site_description: Option<String>,
    site_icon: Option<String>,
    contact: Option<String>,
}

fn normalize_friend_apply_payload(
    payload: FriendApplyRequest,
) -> Result<NormalizedFriendApplyPayload, AppError> {
    let site_title = payload.site_title.trim().to_string();
    if site_title.is_empty() {
        return Err(AppError::Unprocessable(
            "site_title is required".to_string(),
        ));
    }
    if site_title.chars().count() > 255 {
        return Err(AppError::Unprocessable(
            "site_title exceeds max length".to_string(),
        ));
    }

    let site_url = payload.site_url.trim().to_string();
    if site_url.is_empty() {
        return Err(AppError::Unprocessable("site_url is required".to_string()));
    }
    if site_url.chars().count() > 500 || !is_valid_http_url(&site_url) {
        return Err(AppError::Unprocessable(
            "URL must start with http:// or https://".to_string(),
        ));
    }

    let site_description = normalize_optional_text(payload.site_description, 1000)?;
    let site_icon = normalize_optional_text(payload.site_icon, 500)?;
    if let Some(site_icon) = site_icon.as_deref() {
        if !is_valid_http_url(site_icon) {
            return Err(AppError::Unprocessable(
                "URL must start with http:// or https://".to_string(),
            ));
        }
    }
    let contact = normalize_optional_text(payload.contact, 255)?;

    Ok(NormalizedFriendApplyPayload {
        site_title,
        site_url,
        site_description,
        site_icon,
        contact,
    })
}

fn normalize_friend_status(value: &str) -> String {
    let normalized = value.trim().to_lowercase();
    if ["ok", "missing", "blocked"].contains(&normalized.as_str()) {
        return normalized;
    }
    if let Ok(parsed) = normalized.parse::<i64>() {
        return match parsed {
            1 => "ok".to_string(),
            2 | 0 => "missing".to_string(),
            3 | -1 => "blocked".to_string(),
            _ => "ok".to_string(),
        };
    }
    "ok".to_string()
}

fn is_valid_http_url(value: &str) -> bool {
    Url::parse(value)
        .ok()
        .map(|url| (url.scheme() == "http" || url.scheme() == "https") && url.host_str().is_some())
        .unwrap_or(false)
}

fn resolve_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn truncate_text(value: String, max_len: usize) -> String {
    value.chars().take(max_len).collect::<String>()
}

fn parse_liked_cookie(cookie_header: Option<&header::HeaderValue>, key: &str) -> Vec<i64> {
    let raw = cookie_header
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    let mut value = "";
    for chunk in raw.split(';') {
        let trimmed = chunk.trim();
        if let Some((cookie_key, cookie_value)) = trimmed.split_once('=') {
            if cookie_key.trim() == key {
                value = cookie_value.trim();
                break;
            }
        }
    }

    let mut seen = HashSet::<i64>::new();
    let mut result = Vec::<i64>::new();
    for chunk in value.split(',') {
        let item = chunk.trim();
        if item.is_empty() {
            continue;
        }
        let Ok(parsed) = item.parse::<i64>() else {
            continue;
        };
        if parsed <= 0 || seen.contains(&parsed) {
            continue;
        }
        seen.insert(parsed);
        result.push(parsed);
    }
    result
}

fn format_liked_cookie_value(key: &str, ids: &[i64]) -> String {
    let joined = ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("{key}={joined}; Max-Age={LIKE_COOKIE_MAX_AGE_SECONDS}; Path=/; SameSite=Lax")
}
