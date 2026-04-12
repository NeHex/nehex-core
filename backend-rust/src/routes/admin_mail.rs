use std::{
    collections::HashMap,
    sync::RwLock as StdRwLock,
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, Method},
};
use chrono::{NaiveDateTime, Utc};
use lettre::{
    Message, SmtpTransport, Transport,
    message::Mailbox,
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::warn;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

use super::admin_auth;

const MAIL_STATUS_FILTERS: &[&str] = &["all", "success", "failed"];
const MAIL_SMTP_SECURITY_VALUES: &[&str] = &["none", "starttls", "ssl"];
const SITE_TITLE_SETTING_KEY: &str = "site_title";
const SITE_URL_SETTING_KEY: &str = "site_url";

const MAIL_SETTING_SMTP_HOST: &str = "mail_smtp_host";
const MAIL_SETTING_SMTP_PORT: &str = "mail_smtp_port";
const MAIL_SETTING_SMTP_SECURITY: &str = "mail_smtp_security";
const MAIL_SETTING_SMTP_USERNAME: &str = "mail_smtp_username";
const MAIL_SETTING_SMTP_PASSWORD: &str = "mail_smtp_password";
const MAIL_SETTING_SMTP_FROM_EMAIL: &str = "mail_smtp_from_email";
const MAIL_SETTING_SMTP_FROM_NAME: &str = "mail_smtp_from_name";
const MAIL_SETTING_SMTP_TIMEOUT_SECONDS: &str = "mail_smtp_timeout_seconds";
const MAIL_SETTING_NOTIFY_ADMIN_EMAIL: &str = "mail_notify_admin_email";
const MAIL_SETTING_NOTIFY_NEW_COMMENT_ENABLED: &str = "mail_notify_new_comment_enabled";
const MAIL_SETTING_NOTIFY_REPLY_ENABLED: &str = "mail_notify_reply_enabled";
const MAIL_SETTING_REPLY_SUBJECT_TEMPLATE: &str = "mail_reply_subject_template";
const MAIL_SETTING_REPLY_BODY_TEMPLATE: &str = "mail_reply_body_template";
const MAIL_SETTING_NEW_COMMENT_SUBJECT_TEMPLATE: &str = "mail_new_comment_subject_template";
const MAIL_SETTING_NEW_COMMENT_BODY_TEMPLATE: &str = "mail_new_comment_body_template";

const DEFAULT_REPLY_SUBJECT_TEMPLATE: &str = "[{{site_title}}] 你的评论有新回复";
const DEFAULT_REPLY_BODY_TEMPLATE: &str = "你好，{{parent_nickname}}：\n\n你在 {{site_title}} 的评论收到了新回复。\n\n原评论内容：\n{{parent_content}}\n\n回复者：{{reply_nickname}}\n回复内容：\n{{reply_content}}\n\n评论位置：{{target_type}} #{{target_id}}\n直达链接：{{comment_url}}\n回复时间：{{reply_time}}\n";
const DEFAULT_NEW_COMMENT_SUBJECT_TEMPLATE: &str = "[{{site_title}}] 收到新评论提醒";
const DEFAULT_NEW_COMMENT_BODY_TEMPLATE: &str = "{{site_title}} 收到了一条新评论。\n\n评论者：{{comment_nickname}}\n评论者邮箱：{{comment_email}}\n评论位置：{{target_type}} #{{target_id}}\n直达链接：{{comment_url}}\n评论时间：{{comment_time}}\n\n评论内容：\n{{comment_content}}\n";

static TEMPLATE_VAR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{\s*([a-zA-Z0-9_]+)\s*\}\}").expect("template variable regex should compile")
});
const MAIL_NOTIFICATION_SETTINGS_CACHE_TTL_SECONDS: u64 = 30;

#[derive(Clone)]
struct MailSettingsCacheEntry {
    expires_at: Instant,
    value: MailNotificationSettings,
}

static MAIL_NOTIFICATION_SETTINGS_CACHE: Lazy<StdRwLock<Option<MailSettingsCacheEntry>>> =
    Lazy::new(|| StdRwLock::new(None));

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

#[derive(Serialize)]
struct AdminMailLogItem {
    id: i64,
    category: String,
    template_key: String,
    to_email: String,
    subject: String,
    body: String,
    status: String,
    error_message: Option<String>,
    trigger_comment_id: Option<i64>,
    created_at: NaiveDateTime,
    sent_at: Option<NaiveDateTime>,
}

#[derive(Serialize)]
pub struct AdminMailLogListResponse {
    data: Vec<AdminMailLogItem>,
    pagination: AdminPagination,
}

#[derive(Deserialize)]
pub struct AdminMailLogListQuery {
    status: Option<String>,
    page: Option<i64>,
    size: Option<i64>,
}

#[derive(Deserialize)]
pub struct AdminMailSmtpTestRequest {
    smtp_host: String,
    #[serde(default = "default_smtp_port")]
    smtp_port: i64,
    #[serde(default = "default_smtp_security")]
    smtp_security: String,
    smtp_username: Option<String>,
    smtp_password: Option<String>,
    smtp_from_email: Option<String>,
    smtp_from_name: Option<String>,
    #[serde(default = "default_smtp_timeout_seconds")]
    smtp_timeout_seconds: i64,
    test_email: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct SmtpRuntimeConfig {
    host: String,
    port: u16,
    security: String,
    username: String,
    password: String,
    from_email: String,
    from_name: String,
    timeout_seconds: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct MailNotificationSettings {
    smtp: SmtpRuntimeConfig,
    site_title: String,
    site_url: String,
    notify_admin_email: String,
    notify_new_comment_enabled: bool,
    notify_reply_enabled: bool,
    reply_subject_template: String,
    reply_body_template: String,
    new_comment_subject_template: String,
    new_comment_body_template: String,
}

pub(crate) struct MailNoticeCommentContext {
    pub id: i64,
    pub parent_id: i64,
    pub target_type: String,
    pub target_id: i64,
    pub nickname: String,
    pub email: Option<String>,
    pub content: String,
    pub create_time: NaiveDateTime,
}

pub(crate) fn spawn_comment_notification_mails(state: AppState, comment: MailNoticeCommentContext) {
    tokio::spawn(async move {
        send_comment_notification_mails_best_effort(&state, &comment).await;
    });
}

pub(crate) fn invalidate_mail_settings_cache() {
    if let Ok(mut guard) = MAIL_NOTIFICATION_SETTINGS_CACHE.write() {
        *guard = None;
    }
}

fn default_smtp_port() -> i64 {
    465
}

fn default_smtp_security() -> String {
    "ssl".to_string()
}

fn default_smtp_timeout_seconds() -> i64 {
    12
}

pub async fn admin_test_mail_smtp(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<AdminMailSmtpTestRequest>,
) -> AppResult<Json<AdminActionResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let recipient = normalize_required_text(payload.test_email.clone(), "test_email")?;
    let smtp = build_runtime_smtp_config(&payload)?;
    let smtp_host = smtp.host.clone();

    let subject = "NeHex 邮件通信测试".to_string();
    let body = format!(
        "这是一封 NeHex 后台发出的 SMTP 测试邮件。\n\n时间：{}\n服务器：{}:{} ({})\n",
        Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        smtp_host,
        smtp.port,
        smtp.security
    );

    let send_result = send_smtp_mail_async(
        smtp.clone(),
        recipient.clone(),
        subject.clone(),
        body.clone(),
    )
    .await;
    match send_result {
        Ok(_) => {
            persist_mail_log(
                &state,
                "smtp_test",
                "smtp_test",
                &recipient,
                &subject,
                &body,
                "success",
                None,
                None,
            )
            .await;
            Ok(Json(AdminActionResponse {
                success: true,
                message: "SMTP 通信成功，测试邮件已发送".to_string(),
            }))
        }
        Err(error_message) => {
            persist_mail_log(
                &state,
                "smtp_test",
                "smtp_test",
                &recipient,
                &subject,
                &body,
                "failed",
                Some(&error_message),
                None,
            )
            .await;
            Err(AppError::Unprocessable(format!(
                "SMTP test failed: {error_message}"
            )))
        }
    }
}

pub async fn admin_list_mail_logs(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Query(query): Query<AdminMailLogListQuery>,
) -> AppResult<Json<AdminMailLogListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let status = normalize_optional_text(query.status)
        .unwrap_or_else(|| "all".to_string())
        .to_lowercase();
    if !MAIL_STATUS_FILTERS.contains(&status.as_str()) {
        return Err(AppError::Unprocessable(
            "Invalid mail status filter".to_string(),
        ));
    }

    let page = query.page.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * size;

    let total = if status == "all" {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(id)::bigint FROM mail_log")
            .fetch_one(&state.db_pool)
            .await
            .map_err(|error| AppError::internal(format!("Failed to count mail logs: {error}")))?
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(id)::bigint FROM mail_log WHERE status = $1::mail_log_status",
        )
        .bind(&status)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to count mail logs: {error}")))?
    };

    if total <= 0 {
        return Ok(Json(AdminMailLogListResponse {
            data: Vec::new(),
            pagination: AdminPagination {
                page,
                size,
                total: 0,
                total_pages: 0,
            },
        }));
    }

    let rows = if status == "all" {
        sqlx::query(
            r#"
            SELECT
                id::bigint AS id,
                category,
                template_key,
                to_email,
                subject,
                body,
                status::text AS status,
                error_message,
                trigger_comment_id::bigint AS trigger_comment_id,
                created_at,
                sent_at
            FROM mail_log
            ORDER BY created_at DESC, id DESC
            OFFSET $1
            LIMIT $2
            "#,
        )
        .bind(offset)
        .bind(size)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list mail logs: {error}")))?
    } else {
        sqlx::query(
            r#"
            SELECT
                id::bigint AS id,
                category,
                template_key,
                to_email,
                subject,
                body,
                status::text AS status,
                error_message,
                trigger_comment_id::bigint AS trigger_comment_id,
                created_at,
                sent_at
            FROM mail_log
            WHERE status = $1::mail_log_status
            ORDER BY created_at DESC, id DESC
            OFFSET $2
            LIMIT $3
            "#,
        )
        .bind(&status)
        .bind(offset)
        .bind(size)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list mail logs: {error}")))?
    };

    let data = rows
        .into_iter()
        .map(|row| AdminMailLogItem {
            id: row.try_get::<i64, _>("id").unwrap_or_default(),
            category: row.try_get::<String, _>("category").unwrap_or_default(),
            template_key: row.try_get::<String, _>("template_key").unwrap_or_default(),
            to_email: row.try_get::<String, _>("to_email").unwrap_or_default(),
            subject: row.try_get::<String, _>("subject").unwrap_or_default(),
            body: row.try_get::<String, _>("body").unwrap_or_default(),
            status: row
                .try_get::<String, _>("status")
                .unwrap_or_else(|_| "failed".to_string()),
            error_message: row
                .try_get::<Option<String>, _>("error_message")
                .ok()
                .flatten(),
            trigger_comment_id: row
                .try_get::<Option<i64>, _>("trigger_comment_id")
                .ok()
                .flatten(),
            created_at: row
                .try_get::<NaiveDateTime, _>("created_at")
                .unwrap_or_else(|_| Utc::now().naive_utc()),
            sent_at: row
                .try_get::<Option<NaiveDateTime>, _>("sent_at")
                .ok()
                .flatten(),
        })
        .collect::<Vec<_>>();

    let total_pages = (total + size - 1) / size;
    Ok(Json(AdminMailLogListResponse {
        data,
        pagination: AdminPagination {
            page,
            size,
            total,
            total_pages,
        },
    }))
}

pub(crate) async fn send_comment_notification_mails_best_effort(
    state: &AppState,
    comment: &MailNoticeCommentContext,
) {
    let settings = match load_mail_notification_settings(state).await {
        Ok(settings) => settings,
        Err(error) => {
            warn!("Skip comment mail notification due to settings error: {error}");
            return;
        }
    };

    let target_path = match build_target_path(state, &comment.target_type, comment.target_id).await
    {
        Ok(path) => path,
        Err(error) => {
            warn!("Failed to build comment target path: {error}");
            "/".to_string()
        }
    };
    let target_url = join_site_url(&settings.site_url, &target_path);
    let comment_url = with_comment_anchor(&target_url, comment.id);
    let parent_comment_url = if comment.parent_id > 0 {
        with_comment_anchor(&target_url, comment.parent_id)
    } else {
        String::new()
    };

    let mut base_context = HashMap::<&str, String>::new();
    base_context.insert("site_title", settings.site_title.clone());
    base_context.insert("target_type", comment.target_type.clone());
    base_context.insert("target_id", comment.target_id.to_string());
    base_context.insert("target_url", target_url.clone());
    base_context.insert("comment_url", comment_url.clone());
    base_context.insert("parent_comment_url", parent_comment_url);

    if settings.notify_new_comment_enabled && !settings.notify_admin_email.trim().is_empty() {
        let mut context = base_context.clone();
        context.insert("comment_nickname", comment.nickname.clone());
        context.insert(
            "comment_email",
            normalize_optional_text(comment.email.clone()).unwrap_or_else(|| "-".to_string()),
        );
        context.insert("comment_content", comment.content.clone());
        context.insert(
            "comment_time",
            comment.create_time.format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        let subject = render_template(&settings.new_comment_subject_template, &context);
        let mut body = render_template(&settings.new_comment_body_template, &context);
        body = append_direct_link_if_missing(&body, &comment_url);

        send_and_record_comment_notice(
            state,
            &settings.smtp,
            "new_comment_notice",
            "new_comment",
            &settings.notify_admin_email,
            &subject,
            &body,
            Some(comment.id),
        )
        .await;
    }

    if !settings.notify_reply_enabled || comment.parent_id <= 0 {
        return;
    }

    let parent_comment = match load_parent_comment(state, comment.parent_id).await {
        Ok(Some(parent)) => parent,
        Ok(None) => return,
        Err(error) => {
            warn!("Failed to load parent comment for notification: {error}");
            return;
        }
    };
    let recipient = normalize_optional_text(parent_comment.email.clone()).unwrap_or_default();
    if recipient.is_empty() {
        return;
    }

    let mut context = base_context;
    context.insert("parent_nickname", parent_comment.nickname);
    context.insert("parent_content", parent_comment.content);
    context.insert(
        "parent_comment_url",
        with_comment_anchor(&target_url, parent_comment.id),
    );
    context.insert("reply_nickname", comment.nickname.clone());
    context.insert("reply_content", comment.content.clone());
    context.insert(
        "reply_time",
        comment.create_time.format("%Y-%m-%d %H:%M:%S").to_string(),
    );

    let subject = render_template(&settings.reply_subject_template, &context);
    let mut body = render_template(&settings.reply_body_template, &context);
    body = append_direct_link_if_missing(&body, &comment_url);

    send_and_record_comment_notice(
        state,
        &settings.smtp,
        "reply_notice",
        "reply",
        &recipient,
        &subject,
        &body,
        Some(comment.id),
    )
    .await;
}

fn build_runtime_smtp_config(payload: &AdminMailSmtpTestRequest) -> AppResult<SmtpRuntimeConfig> {
    let host = normalize_required_text(payload.smtp_host.clone(), "smtp_host")?;
    let security =
        normalize_required_text(payload.smtp_security.clone(), "smtp_security")?.to_lowercase();
    if !MAIL_SMTP_SECURITY_VALUES.contains(&security.as_str()) {
        return Err(AppError::Unprocessable(
            "smtp_security must be one of: none/starttls/ssl".to_string(),
        ));
    }

    let port = payload.smtp_port.clamp(1, 65535) as u16;
    let timeout_seconds = payload.smtp_timeout_seconds.clamp(3, 120) as u64;
    let username = normalize_optional_text(payload.smtp_username.clone()).unwrap_or_default();
    let password = normalize_optional_text(payload.smtp_password.clone()).unwrap_or_default();
    let from_email = normalize_optional_text(payload.smtp_from_email.clone())
        .or_else(|| (!username.trim().is_empty()).then_some(username.clone()))
        .unwrap_or_default();
    let from_name = normalize_optional_text(payload.smtp_from_name.clone()).unwrap_or_default();

    if from_email.trim().is_empty() {
        return Err(AppError::Unprocessable(
            "SMTP sender email is required".to_string(),
        ));
    }

    Ok(SmtpRuntimeConfig {
        host,
        port,
        security,
        username,
        password,
        from_email,
        from_name,
        timeout_seconds,
    })
}

fn send_smtp_mail(
    config: &SmtpRuntimeConfig,
    to_email: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    let recipient = normalize_optional_text(Some(to_email.to_string())).unwrap_or_default();
    if recipient.is_empty() {
        return Err("Recipient email is required".to_string());
    }

    let from_address = config
        .from_email
        .parse()
        .map_err(|error| format!("Invalid from email: {error}"))?;
    let to_address = recipient
        .parse()
        .map_err(|error| format!("Invalid recipient email: {error}"))?;

    let from_mailbox = if config.from_name.trim().is_empty() {
        Mailbox::new(None, from_address)
    } else {
        Mailbox::new(Some(config.from_name.clone()), from_address)
    };

    let message = Message::builder()
        .from(from_mailbox)
        .to(Mailbox::new(None, to_address))
        .subject(subject.trim())
        .body(body.to_string())
        .map_err(|error| format!("Failed to build email message: {error}"))?;

    let mut builder = match config.security.as_str() {
        "none" => SmtpTransport::builder_dangerous(&config.host).tls(Tls::None),
        "starttls" => {
            let tls = TlsParameters::new(config.host.clone())
                .map_err(|error| format!("Failed to configure STARTTLS: {error}"))?;
            SmtpTransport::builder_dangerous(&config.host).tls(Tls::Required(tls))
        }
        _ => SmtpTransport::relay(&config.host)
            .map_err(|error| format!("Failed to configure SMTP relay: {error}"))?,
    };

    builder = builder
        .port(config.port)
        .timeout(Some(Duration::from_secs(config.timeout_seconds)));

    if !config.username.trim().is_empty() {
        builder = builder.credentials(Credentials::new(
            config.username.clone(),
            config.password.clone(),
        ));
    }

    let transport = builder.build();
    transport
        .send(&message)
        .map_err(|error| format!("SMTP send failed: {error}"))?;

    Ok(())
}

async fn send_smtp_mail_async(
    config: SmtpRuntimeConfig,
    to_email: String,
    subject: String,
    body: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || send_smtp_mail(&config, &to_email, &subject, &body))
        .await
        .map_err(|error| format!("SMTP task join failed: {error}"))?
}

async fn persist_mail_log(
    state: &AppState,
    category: &str,
    template_key: &str,
    to_email: &str,
    subject: &str,
    body: &str,
    status: &str,
    error_message: Option<&str>,
    trigger_comment_id: Option<i64>,
) {
    let normalized_error = error_message
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| {
            if value.len() > 4000 {
                value[..4000].to_string()
            } else {
                value.to_string()
            }
        });

    let sent_at = if status == "success" {
        Some(Utc::now().naive_utc())
    } else {
        None
    };

    let result = sqlx::query(
        r#"
        INSERT INTO mail_log (
            category,
            template_key,
            to_email,
            subject,
            body,
            status,
            error_message,
            trigger_comment_id,
            sent_at
        )
        VALUES ($1, $2, $3, $4, $5, $6::mail_log_status, $7, $8, $9)
        "#,
    )
    .bind(category)
    .bind(template_key)
    .bind(to_email)
    .bind(subject)
    .bind(body)
    .bind(status)
    .bind(normalized_error)
    .bind(trigger_comment_id)
    .bind(sent_at)
    .execute(&state.db_pool)
    .await;

    if let Err(error) = result {
        warn!("Failed to persist mail log: {error}");
    }
}

async fn send_and_record_comment_notice(
    state: &AppState,
    smtp: &SmtpRuntimeConfig,
    category: &str,
    template_key: &str,
    to_email: &str,
    subject: &str,
    body: &str,
    trigger_comment_id: Option<i64>,
) {
    match send_smtp_mail_async(
        smtp.clone(),
        to_email.to_string(),
        subject.to_string(),
        body.to_string(),
    )
    .await
    {
        Ok(_) => {
            persist_mail_log(
                state,
                category,
                template_key,
                to_email,
                subject,
                body,
                "success",
                None,
                trigger_comment_id,
            )
            .await;
        }
        Err(error_message) => {
            persist_mail_log(
                state,
                category,
                template_key,
                to_email,
                subject,
                body,
                "failed",
                Some(&error_message),
                trigger_comment_id,
            )
            .await;
        }
    }
}

async fn load_mail_notification_settings(state: &AppState) -> AppResult<MailNotificationSettings> {
    let now = Instant::now();
    if let Ok(guard) = MAIL_NOTIFICATION_SETTINGS_CACHE.read() {
        if let Some(entry) = guard.as_ref() {
            if entry.expires_at > now {
                return Ok(entry.value.clone());
            }
        }
    }

    let keys = vec![
        SITE_TITLE_SETTING_KEY,
        SITE_URL_SETTING_KEY,
        MAIL_SETTING_SMTP_HOST,
        MAIL_SETTING_SMTP_PORT,
        MAIL_SETTING_SMTP_SECURITY,
        MAIL_SETTING_SMTP_USERNAME,
        MAIL_SETTING_SMTP_PASSWORD,
        MAIL_SETTING_SMTP_FROM_EMAIL,
        MAIL_SETTING_SMTP_FROM_NAME,
        MAIL_SETTING_SMTP_TIMEOUT_SECONDS,
        MAIL_SETTING_NOTIFY_ADMIN_EMAIL,
        MAIL_SETTING_NOTIFY_NEW_COMMENT_ENABLED,
        MAIL_SETTING_NOTIFY_REPLY_ENABLED,
        MAIL_SETTING_REPLY_SUBJECT_TEMPLATE,
        MAIL_SETTING_REPLY_BODY_TEMPLATE,
        MAIL_SETTING_NEW_COMMENT_SUBJECT_TEMPLATE,
        MAIL_SETTING_NEW_COMMENT_BODY_TEMPLATE,
    ];

    let rows = sqlx::query(
        "SELECT setting_key, setting_content FROM settings WHERE setting_key = ANY($1)",
    )
    .bind(keys)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to load mail settings: {error}")))?;

    let mut setting_map = HashMap::<String, String>::new();
    for row in rows {
        let key = row.try_get::<String, _>("setting_key").unwrap_or_default();
        let value = row
            .try_get::<Option<String>, _>("setting_content")
            .ok()
            .flatten()
            .unwrap_or_default();
        setting_map.insert(key, value);
    }

    let smtp = build_runtime_smtp_config_from_settings(&setting_map);
    let site_title = normalize_optional_text(setting_map.get(SITE_TITLE_SETTING_KEY).cloned())
        .unwrap_or_else(|| "NeHex".to_string());

    let settings = MailNotificationSettings {
        smtp,
        site_title,
        site_url: normalize_site_url(setting_map.get(SITE_URL_SETTING_KEY).cloned()),
        notify_admin_email: normalize_optional_text(
            setting_map.get(MAIL_SETTING_NOTIFY_ADMIN_EMAIL).cloned(),
        )
        .unwrap_or_default(),
        notify_new_comment_enabled: parse_boolean_setting(
            setting_map
                .get(MAIL_SETTING_NOTIFY_NEW_COMMENT_ENABLED)
                .map(|value| value.as_str()),
            false,
        ),
        notify_reply_enabled: parse_boolean_setting(
            setting_map
                .get(MAIL_SETTING_NOTIFY_REPLY_ENABLED)
                .map(|value| value.as_str()),
            false,
        ),
        reply_subject_template: normalize_optional_text(
            setting_map
                .get(MAIL_SETTING_REPLY_SUBJECT_TEMPLATE)
                .cloned(),
        )
        .unwrap_or_else(|| DEFAULT_REPLY_SUBJECT_TEMPLATE.to_string()),
        reply_body_template: normalize_optional_text(
            setting_map.get(MAIL_SETTING_REPLY_BODY_TEMPLATE).cloned(),
        )
        .unwrap_or_else(|| DEFAULT_REPLY_BODY_TEMPLATE.to_string()),
        new_comment_subject_template: normalize_optional_text(
            setting_map
                .get(MAIL_SETTING_NEW_COMMENT_SUBJECT_TEMPLATE)
                .cloned(),
        )
        .unwrap_or_else(|| DEFAULT_NEW_COMMENT_SUBJECT_TEMPLATE.to_string()),
        new_comment_body_template: normalize_optional_text(
            setting_map
                .get(MAIL_SETTING_NEW_COMMENT_BODY_TEMPLATE)
                .cloned(),
        )
        .unwrap_or_else(|| DEFAULT_NEW_COMMENT_BODY_TEMPLATE.to_string()),
    };

    if let Ok(mut guard) = MAIL_NOTIFICATION_SETTINGS_CACHE.write() {
        *guard = Some(MailSettingsCacheEntry {
            expires_at: now + Duration::from_secs(MAIL_NOTIFICATION_SETTINGS_CACHE_TTL_SECONDS),
            value: settings.clone(),
        });
    }

    Ok(settings)
}

fn build_runtime_smtp_config_from_settings(
    setting_map: &HashMap<String, String>,
) -> SmtpRuntimeConfig {
    let host = normalize_optional_text(setting_map.get(MAIL_SETTING_SMTP_HOST).cloned())
        .unwrap_or_default();
    let port = parse_int_setting(
        setting_map
            .get(MAIL_SETTING_SMTP_PORT)
            .map(|value| value.as_str()),
        465,
        1,
        65535,
    ) as u16;
    let security = normalize_smtp_security(
        setting_map
            .get(MAIL_SETTING_SMTP_SECURITY)
            .map(|value| value.as_str()),
    );
    let username = normalize_optional_text(setting_map.get(MAIL_SETTING_SMTP_USERNAME).cloned())
        .unwrap_or_default();
    let password = normalize_optional_text(setting_map.get(MAIL_SETTING_SMTP_PASSWORD).cloned())
        .unwrap_or_default();
    let from_email =
        normalize_optional_text(setting_map.get(MAIL_SETTING_SMTP_FROM_EMAIL).cloned())
            .or_else(|| (!username.trim().is_empty()).then_some(username.clone()))
            .unwrap_or_default();
    let from_name = normalize_optional_text(setting_map.get(MAIL_SETTING_SMTP_FROM_NAME).cloned())
        .unwrap_or_default();
    let timeout_seconds = parse_int_setting(
        setting_map
            .get(MAIL_SETTING_SMTP_TIMEOUT_SECONDS)
            .map(|value| value.as_str()),
        12,
        3,
        120,
    ) as u64;

    SmtpRuntimeConfig {
        host,
        port,
        security,
        username,
        password,
        from_email,
        from_name,
        timeout_seconds,
    }
}

#[derive(sqlx::FromRow)]
struct ParentCommentRow {
    id: i64,
    nickname: String,
    email: Option<String>,
    content: String,
}

async fn load_parent_comment(
    state: &AppState,
    comment_id: i64,
) -> AppResult<Option<ParentCommentRow>> {
    let row = sqlx::query_as::<_, ParentCommentRow>(
        "SELECT id::bigint AS id, nickname, email, content FROM comment WHERE id = $1 LIMIT 1",
    )
    .bind(comment_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to query parent comment: {error}")))?;
    Ok(row)
}

async fn build_target_path(
    state: &AppState,
    target_type: &str,
    target_id: i64,
) -> AppResult<String> {
    let normalized_type = target_type.trim().to_lowercase();
    let normalized_id = target_id.max(1);

    match normalized_type.as_str() {
        "article" => Ok(format!("/article/{normalized_id}")),
        "album" => Ok(format!("/album/{normalized_id}")),
        "friend_page" => Ok("/friends".to_string()),
        "singlepage" => {
            let page_key = sqlx::query_scalar::<_, String>(
                "SELECT page_key FROM singlepage WHERE id = $1 LIMIT 1",
            )
            .bind(normalized_id)
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|error| {
                AppError::internal(format!("Failed to resolve singlepage path: {error}"))
            })?;
            if let Some(page_key) = page_key {
                let normalized = page_key.trim().trim_matches('/').to_string();
                if !normalized.is_empty() {
                    return Ok(format!("/{normalized}"));
                }
            }
            Ok(format!("/page/{normalized_id}"))
        }
        _ => Ok("/".to_string()),
    }
}

fn join_site_url(site_url: &str, path: &str) -> String {
    let normalized_path = if path.trim().is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path.trim().trim_start_matches('/'))
    };

    if site_url.trim().is_empty() {
        normalized_path
    } else {
        format!(
            "{}{}",
            site_url.trim().trim_end_matches('/'),
            normalized_path
        )
    }
}

fn with_comment_anchor(target_url: &str, comment_id: i64) -> String {
    if target_url.trim().is_empty() {
        String::new()
    } else {
        format!("{}#comment-{}", target_url, comment_id.max(1))
    }
}

fn append_direct_link_if_missing(body: &str, comment_url: &str) -> String {
    let normalized_body = body.to_string();
    let normalized_comment_url = comment_url.trim().to_string();
    if normalized_comment_url.is_empty() || normalized_body.contains(&normalized_comment_url) {
        return normalized_body;
    }

    let suffix = format!("直达链接：{normalized_comment_url}");
    if normalized_body.trim().is_empty() {
        return suffix;
    }
    format!("{}\n\n{suffix}\n", normalized_body.trim_end())
}

fn render_template(template: &str, context: &HashMap<&str, String>) -> String {
    TEMPLATE_VAR_PATTERN
        .replace_all(template, |captures: &regex::Captures<'_>| {
            captures
                .get(1)
                .and_then(|matched| context.get(matched.as_str()))
                .cloned()
                .unwrap_or_default()
        })
        .to_string()
}

fn parse_boolean_setting(value: Option<&str>, default: bool) -> bool {
    let Some(value) = value else {
        return default;
    };
    let normalized = value.trim().to_lowercase();
    if normalized.is_empty() {
        return default;
    }
    matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
}

fn parse_int_setting(value: Option<&str>, default: i64, min: i64, max: i64) -> i64 {
    let parsed = value
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(default);
    parsed.clamp(min, max)
}

fn normalize_site_url(value: Option<String>) -> String {
    let normalized = value
        .unwrap_or_default()
        .trim()
        .trim_end_matches('/')
        .to_string();
    if normalized.is_empty() {
        return String::new();
    }
    if normalized.starts_with("http://") || normalized.starts_with("https://") {
        return normalized;
    }
    format!("https://{}", normalized.trim_start_matches('/'))
}

fn normalize_smtp_security(value: Option<&str>) -> String {
    let normalized = value.unwrap_or_default().trim().to_lowercase();
    if MAIL_SMTP_SECURITY_VALUES.contains(&normalized.as_str()) {
        normalized
    } else {
        "ssl".to_string()
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    let normalized = value?.trim().to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_required_text(value: String, field_name: &str) -> AppResult<String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Err(AppError::Unprocessable(format!("{field_name} is required")));
    }
    Ok(normalized)
}
