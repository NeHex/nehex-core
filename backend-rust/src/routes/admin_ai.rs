use std::{collections::HashMap, time::Duration};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, Method},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

use super::admin_auth;

const AI_ENABLED_KEY: &str = "ai_enabled";
const AI_PROVIDER_KEY: &str = "ai_provider";
const AI_BASE_URL_KEY: &str = "ai_base_url";
const AI_API_KEY_KEY: &str = "ai_api_key";
const AI_MODEL_KEY: &str = "ai_model";
const AI_ARTICLE_SUMMARY_ENABLED_KEY: &str = "ai_article_summary_enabled";
const AI_ARTICLE_SUMMARY_PROMPT_KEY: &str = "ai_article_summary_prompt";
const AI_LEGACY_SYSTEM_PROMPT_KEY: &str = "ai_system_prompt";
const AI_REQUEST_TIMEOUT_SECONDS: u64 = 45;
const DEFAULT_ARTICLE_SUMMARY_PROMPT: &str =
    "这是我写的一篇文章，希望你可以彻底读懂然后总结出来，总结内容最好控制在50字左右，文章内容: {article_content}";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";

#[derive(Deserialize)]
pub struct AdminArticleSummaryGenerateRequest {
    article_content: String,
}

#[derive(Serialize)]
pub struct AdminArticleSummaryGenerateResponse {
    data: AdminArticleSummaryGenerateData,
}

#[derive(Serialize)]
pub struct AdminArticleSummaryGenerateData {
    summary: String,
}

struct AiRuntimeSettings {
    enabled: bool,
    provider: String,
    base_url: String,
    api_key: String,
    model: String,
    article_summary_enabled: bool,
    article_summary_prompt: String,
}

pub async fn admin_generate_article_summary(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<AdminArticleSummaryGenerateRequest>,
) -> AppResult<Json<AdminArticleSummaryGenerateResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let article_content = payload.article_content.trim().to_string();
    if article_content.is_empty() {
        return Err(AppError::Unprocessable("文章内容不能为空".to_string()));
    }

    let settings = load_ai_runtime_settings(&state).await?;
    if !settings.enabled {
        return Err(AppError::Unprocessable("AI 功能未启用".to_string()));
    }
    if !settings.article_summary_enabled {
        return Err(AppError::Unprocessable("AI文章总结功能未启用".to_string()));
    }
    if settings.base_url.trim().is_empty() {
        return Err(AppError::Unprocessable("AI_BASE_URL 未配置".to_string()));
    }
    if settings.api_key.trim().is_empty() {
        return Err(AppError::Unprocessable("AI_API_KEY 未配置".to_string()));
    }
    if settings.model.trim().is_empty() {
        return Err(AppError::Unprocessable("AI_MODEL 未配置".to_string()));
    }

    let prompt = settings
        .article_summary_prompt
        .replace("{article_content}", &article_content);

    let summary = request_summary_from_provider(&settings, &prompt).await?;
    if summary.trim().is_empty() {
        return Err(AppError::internal("AI 返回内容为空".to_string()));
    }

    Ok(Json(AdminArticleSummaryGenerateResponse {
        data: AdminArticleSummaryGenerateData {
            summary: summary.trim().to_string(),
        },
    }))
}

async fn load_ai_runtime_settings(state: &AppState) -> AppResult<AiRuntimeSettings> {
    let keys = vec![
        AI_ENABLED_KEY.to_string(),
        AI_PROVIDER_KEY.to_string(),
        AI_BASE_URL_KEY.to_string(),
        AI_API_KEY_KEY.to_string(),
        AI_MODEL_KEY.to_string(),
        AI_ARTICLE_SUMMARY_ENABLED_KEY.to_string(),
        AI_ARTICLE_SUMMARY_PROMPT_KEY.to_string(),
        AI_LEGACY_SYSTEM_PROMPT_KEY.to_string(),
    ];

    let rows = sqlx::query(
        r#"
        SELECT setting_key, setting_type::text AS setting_type, setting_content
        FROM settings
        WHERE setting_key = ANY($1)
        "#,
    )
    .bind(keys)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to load ai settings: {error}")))?;

    let mut map = HashMap::<String, (String, String)>::new();
    for row in rows {
        let key = row.try_get::<String, _>("setting_key").unwrap_or_default();
        let setting_type = row
            .try_get::<String, _>("setting_type")
            .unwrap_or_else(|_| "string".to_string())
            .to_lowercase();
        let setting_content = row
            .try_get::<Option<String>, _>("setting_content")
            .ok()
            .flatten()
            .unwrap_or_default();
        map.insert(key, (setting_type, setting_content));
    }

    let enabled = read_bool_setting(&map, AI_ENABLED_KEY, false);
    let provider = read_string_setting(&map, AI_PROVIDER_KEY, "openai");
    let base_url = read_string_setting(&map, AI_BASE_URL_KEY, "");
    let api_key = read_string_setting(&map, AI_API_KEY_KEY, "");
    let model = read_string_setting(&map, AI_MODEL_KEY, "");
    let article_summary_enabled =
        read_bool_setting(&map, AI_ARTICLE_SUMMARY_ENABLED_KEY, false);
    let article_summary_prompt = {
        let text = read_string_setting(&map, AI_ARTICLE_SUMMARY_PROMPT_KEY, "");
        if text.is_empty() {
            let legacy = read_string_setting(&map, AI_LEGACY_SYSTEM_PROMPT_KEY, "");
            if legacy.is_empty() {
                DEFAULT_ARTICLE_SUMMARY_PROMPT.to_string()
            } else {
                legacy
            }
        } else {
            text
        }
    };

    Ok(AiRuntimeSettings {
        enabled,
        provider,
        base_url,
        api_key,
        model,
        article_summary_enabled,
        article_summary_prompt,
    })
}

fn read_string_setting(
    settings: &HashMap<String, (String, String)>,
    key: &str,
    fallback: &str,
) -> String {
    settings
        .get(key)
        .map(|(_, content)| content.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn read_bool_setting(
    settings: &HashMap<String, (String, String)>,
    key: &str,
    fallback: bool,
) -> bool {
    let Some((setting_type, content)) = settings.get(key) else {
        return fallback;
    };

    if setting_type == "boolean" {
        return matches!(
            content.trim().to_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        );
    }

    matches!(
        content.trim().to_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

async fn request_summary_from_provider(
    settings: &AiRuntimeSettings,
    prompt: &str,
) -> AppResult<String> {
    let provider = settings.provider.trim().to_lowercase();
    let use_anthropic = provider == "anthropic"
        || settings
            .base_url
            .to_lowercase()
            .contains("anthropic");

    if use_anthropic {
        request_anthropic_summary(settings, prompt).await
    } else {
        request_openai_compatible_summary(settings, prompt).await
    }
}

async fn request_openai_compatible_summary(
    settings: &AiRuntimeSettings,
    prompt: &str,
) -> AppResult<String> {
    let endpoint = build_openai_chat_endpoint(&settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(AI_REQUEST_TIMEOUT_SECONDS))
        .build()
        .map_err(|error| AppError::internal(format!("Failed to build AI client: {error}")))?;

    let provider = settings.provider.trim().to_lowercase();
    let use_azure_api_key = provider == "azure-openai"
        || settings
            .base_url
            .to_lowercase()
            .contains("openai.azure.com");

    let mut request = client
        .post(endpoint)
        .header("Content-Type", "application/json");
    request = if use_azure_api_key {
        request.header("api-key", settings.api_key.as_str())
    } else {
        request.header("Authorization", format!("Bearer {}", settings.api_key))
    };

    let response = request
        .json(&serde_json::json!({
            "model": settings.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt,
                }
            ],
            "stream": false
        }))
        .send()
        .await
        .map_err(|error| AppError::internal(format!("Failed to request AI summary: {error}")))?;

    let status = response.status();
    let payload = response
        .json::<Value>()
        .await
        .map_err(|error| AppError::internal(format!("Failed to decode AI response: {error}")))?;

    if !status.is_success() {
        let message = payload
            .get("error")
            .and_then(|error| error.get("message").or_else(|| error.get("detail")))
            .and_then(Value::as_str)
            .unwrap_or("AI 接口请求失败");
        return Err(AppError::ServiceUnavailable(message.to_string()));
    }

    extract_openai_text(&payload)
        .ok_or_else(|| AppError::internal("AI 响应中未找到总结内容".to_string()))
}

async fn request_anthropic_summary(
    settings: &AiRuntimeSettings,
    prompt: &str,
) -> AppResult<String> {
    let endpoint = build_anthropic_messages_endpoint(&settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(AI_REQUEST_TIMEOUT_SECONDS))
        .build()
        .map_err(|error| AppError::internal(format!("Failed to build AI client: {error}")))?;

    let response = client
        .post(endpoint)
        .header("x-api-key", settings.api_key.as_str())
        .header("anthropic-version", ANTHROPIC_API_VERSION)
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": settings.model,
            "max_tokens": 512,
            "messages": [
                {
                    "role": "user",
                    "content": prompt,
                }
            ]
        }))
        .send()
        .await
        .map_err(|error| AppError::internal(format!("Failed to request AI summary: {error}")))?;

    let status = response.status();
    let payload = response
        .json::<Value>()
        .await
        .map_err(|error| AppError::internal(format!("Failed to decode AI response: {error}")))?;

    if !status.is_success() {
        let message = payload
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Anthropic 接口请求失败");
        return Err(AppError::ServiceUnavailable(message.to_string()));
    }

    extract_anthropic_text(&payload)
        .ok_or_else(|| AppError::internal("AI 响应中未找到总结内容".to_string()))
}

fn build_openai_chat_endpoint(base_url: &str) -> String {
    let normalized = base_url.trim().trim_end_matches('/').to_string();
    if normalized.ends_with("/chat/completions") {
        return normalized;
    }
    format!("{normalized}/chat/completions")
}

fn build_anthropic_messages_endpoint(base_url: &str) -> String {
    let normalized = base_url.trim().trim_end_matches('/').to_string();
    if normalized.ends_with("/v1/messages") {
        return normalized;
    }
    if normalized.ends_with("/v1") {
        return format!("{normalized}/messages");
    }
    format!("{normalized}/v1/messages")
}

fn extract_openai_text(payload: &Value) -> Option<String> {
    let first_choice = payload.get("choices")?.as_array()?.first()?;
    let message = first_choice.get("message")?;
    let content = message.get("content")?;

    match content {
        Value::String(text) => Some(text.trim().to_string()),
        Value::Array(parts) => parts.iter().find_map(|part| {
            part.get("text")
                .and_then(Value::as_str)
                .map(|value| value.trim().to_string())
        }),
        _ => None,
    }
}

fn extract_anthropic_text(payload: &Value) -> Option<String> {
    let content = payload.get("content")?.as_array()?;
    content.iter().find_map(|item| {
        item.get("text")
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
    })
}
