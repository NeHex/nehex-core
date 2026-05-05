use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use chrono::{NaiveDateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::warn;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

use super::ws_content_updates::{self, ContentUpdateEvent};

const MAX_SYNC_CHANGES_LIMIT: i64 = 500;
const DEFAULT_SYNC_CHANGES_LIMIT: i64 = 200;
const KNOWN_RESOURCES: &[&str] = &[
    "article", "daily", "album", "project", "friend", "setting", "page", "movie", "comment",
];

#[derive(Serialize)]
pub struct SyncVersionResponse {
    latest_seq: i64,
    versions: HashMap<String, i64>,
}

#[derive(Deserialize)]
pub struct SyncChangesQuery {
    since: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub struct SyncChangesResponse {
    latest_seq: i64,
    changes: Vec<ContentUpdateEvent>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/sync/version", get(get_sync_version))
        .route("/api/sync/changes", get(get_sync_changes))
}

pub async fn get_sync_version(
    State(state): State<AppState>,
) -> AppResult<Json<SyncVersionResponse>> {
    let latest_seq = query_latest_seq(&state).await?;

    let rows = sqlx::query(
        r#"
        SELECT resource, MAX(seq)::bigint AS version
        FROM content_change_log
        GROUP BY resource
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to query sync versions: {error}")))?;

    let mut versions = HashMap::<String, i64>::new();
    for resource in KNOWN_RESOURCES {
        versions.insert((*resource).to_string(), 0);
    }

    for row in rows {
        let resource = row.try_get::<String, _>("resource").unwrap_or_default();
        if resource.trim().is_empty() {
            continue;
        }
        let version = row.try_get::<i64, _>("version").unwrap_or(0);
        versions.insert(resource, version.max(0));
    }

    Ok(Json(SyncVersionResponse {
        latest_seq,
        versions,
    }))
}

pub async fn get_sync_changes(
    State(state): State<AppState>,
    Query(query): Query<SyncChangesQuery>,
) -> AppResult<Json<SyncChangesResponse>> {
    let since = query.since.unwrap_or(0).max(0);
    let limit = query
        .limit
        .unwrap_or(DEFAULT_SYNC_CHANGES_LIMIT)
        .clamp(1, MAX_SYNC_CHANGES_LIMIT);

    let rows = sqlx::query(
        r#"
        SELECT
            seq::bigint AS seq,
            event_type,
            resource,
            action,
            ids::text AS ids_json,
            updated_at
        FROM content_change_log
        WHERE seq > $1
        ORDER BY seq ASC
        LIMIT $2
        "#,
    )
    .bind(since)
    .bind(limit)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to query sync changes: {error}")))?;

    let changes = rows
        .iter()
        .map(map_row_to_event)
        .collect::<Result<Vec<_>, _>>()?;

    let latest_seq = query_latest_seq(&state).await?;

    Ok(Json(SyncChangesResponse {
        latest_seq,
        changes,
    }))
}

pub async fn record_content_change_best_effort<I, T>(
    state: &AppState,
    resource: &str,
    action: &str,
    ids: I,
) where
    I: IntoIterator<Item = T>,
    T: ToString,
{
    if let Err(error) = record_content_change(state, resource, action, ids).await {
        warn!(
            "[content-sync] failed to record change event resource={} action={}: {}",
            resource, action, error
        );
    }
}

pub async fn record_content_change<I, T>(
    state: &AppState,
    resource: &str,
    action: &str,
    ids: I,
) -> AppResult<ContentUpdateEvent>
where
    I: IntoIterator<Item = T>,
    T: ToString,
{
    let normalized_resource = resource.trim().to_lowercase();
    let normalized_action = action.trim().to_lowercase();

    if normalized_resource.is_empty() || normalized_action.is_empty() {
        return Err(AppError::internal(
            "content change resource/action cannot be empty".to_string(),
        ));
    }

    let normalized_ids = ids
        .into_iter()
        .map(|item| item.to_string())
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();

    let row = sqlx::query(
        r#"
        INSERT INTO content_change_log (event_type, resource, action, ids, updated_at)
        VALUES ('content.updated', $1, $2, $3::jsonb, CURRENT_TIMESTAMP)
        RETURNING
            seq::bigint AS seq,
            event_type,
            resource,
            action,
            ids::text AS ids_json,
            updated_at
        "#,
    )
    .bind(&normalized_resource)
    .bind(&normalized_action)
    .bind(serde_json::to_string(&normalized_ids).map_err(|error| {
        AppError::internal(format!("Failed to serialize change ids: {error}"))
    })?)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to append content change log: {error}")))?;

    let event = map_row_to_event(&row)?;
    ws_content_updates::emit_content_update_event(state, event.clone()).await;

    Ok(event)
}

fn map_row_to_event(row: &sqlx::postgres::PgRow) -> AppResult<ContentUpdateEvent> {
    let seq = row.try_get::<i64, _>("seq").unwrap_or(0).max(0);
    let event_type = row
        .try_get::<String, _>("event_type")
        .unwrap_or_else(|_| "content.updated".to_string());
    let resource = row.try_get::<String, _>("resource").unwrap_or_default();
    let action = row.try_get::<String, _>("action").unwrap_or_default();
    let ids_json = row
        .try_get::<String, _>("ids_json")
        .unwrap_or_else(|_| "[]".to_string());
    let updated_at_raw = row
        .try_get::<NaiveDateTime, _>("updated_at")
        .unwrap_or_else(|_| Utc::now().naive_utc());

    let ids_value = serde_json::from_str::<serde_json::Value>(&ids_json).map_err(|error| {
        AppError::internal(format!(
            "Failed to decode content change ids payload `{ids_json}`: {error}"
        ))
    })?;
    let ids = match ids_value {
        serde_json::Value::Array(items) => items
            .into_iter()
            .filter_map(|item| match item {
                serde_json::Value::String(value) => {
                    let normalized = value.trim().to_string();
                    if normalized.is_empty() {
                        None
                    } else {
                        Some(normalized)
                    }
                }
                serde_json::Value::Number(value) => Some(value.to_string()),
                serde_json::Value::Bool(value) => Some(value.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    Ok(ContentUpdateEvent {
        event_type,
        seq,
        resource,
        action,
        ids,
        updated_at: format_utc_iso8601(updated_at_raw),
    })
}

async fn query_latest_seq(state: &AppState) -> AppResult<i64> {
    let latest = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(seq), 0)::bigint FROM content_change_log",
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to query latest sync seq: {error}")))?;

    Ok(latest.max(0))
}

fn format_utc_iso8601(value: NaiveDateTime) -> String {
    chrono::DateTime::<Utc>::from_naive_utc_and_offset(value, Utc)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}
