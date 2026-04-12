use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, Method},
};

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

use super::admin_auth;

const DASHBOARD_CACHE_KEY: &str = "admin:dashboard:overview:v1";
const DASHBOARD_CACHE_TTL_SECONDS: u64 = 60;

#[derive(Clone, Copy)]
enum Unit {
    Hour,
    Day,
    Month,
}

#[derive(Clone)]
struct PeriodSpec {
    start: NaiveDateTime,
    labels: Vec<String>,
    unit: Unit,
}

#[derive(Serialize, Deserialize, Clone)]
struct DashboardSeries {
    labels: Vec<String>,
    values: Vec<i64>,
    total: i64,
}

#[derive(Serialize, Deserialize, Clone)]
struct DashboardPeriodMetrics {
    day: DashboardSeries,
    week: DashboardSeries,
    month: DashboardSeries,
    year: DashboardSeries,
}

#[derive(Serialize, Deserialize, Clone)]
struct DashboardSiteTotals {
    text_count: i64,
    article_count: i64,
    comment_count: i64,
    album_count: i64,
    friend_count: i64,
}

#[derive(Serialize, Deserialize, Clone)]
struct DashboardData {
    visit_ip: DashboardPeriodMetrics,
    api_calls: DashboardPeriodMetrics,
    site_totals: DashboardSiteTotals,
}

#[derive(Serialize)]
pub struct DashboardResponse {
    data: DashboardData,
}

pub async fn admin_dashboard_overview(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> AppResult<Json<DashboardResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    if let Some(cached) = state
        .runtime_cache
        .get::<DashboardData>(DASHBOARD_CACHE_KEY)
        .await
    {
        return Ok(Json(DashboardResponse { data: cached }));
    }

    let now = Utc::now()
        .naive_utc()
        .with_nanosecond(0)
        .unwrap_or_else(|| Utc::now().naive_utc());
    let specs = build_period_specs(now)?;
    let data = match build_dashboard_data(&state, &specs).await {
        Ok(data) => data,
        Err(error) => {
            warn!("Dashboard query degraded to empty payload: {error}");
            empty_dashboard_data(&specs)
        }
    };
    state
        .runtime_cache
        .set(
            DASHBOARD_CACHE_KEY,
            data.clone(),
            DASHBOARD_CACHE_TTL_SECONDS,
        )
        .await;
    Ok(Json(DashboardResponse { data }))
}

async fn build_dashboard_data(state: &AppState, specs: &Periods) -> AppResult<DashboardData> {
    let visit_ip = DashboardPeriodMetrics {
        day: build_visit_series(state, &specs.day).await?,
        week: build_visit_series(state, &specs.week).await?,
        month: build_visit_series(state, &specs.month).await?,
        year: build_visit_series(state, &specs.year).await?,
    };

    let api_calls = DashboardPeriodMetrics {
        day: build_api_count_series(state, &specs.day).await?,
        week: build_api_count_series(state, &specs.week).await?,
        month: build_api_count_series(state, &specs.month).await?,
        year: build_api_count_series(state, &specs.year).await?,
    };

    let site_totals = build_site_totals(state).await?;

    Ok(DashboardData {
        visit_ip,
        api_calls,
        site_totals,
    })
}

fn empty_dashboard_data(specs: &Periods) -> DashboardData {
    DashboardData {
        visit_ip: DashboardPeriodMetrics {
            day: empty_series(&specs.day),
            week: empty_series(&specs.week),
            month: empty_series(&specs.month),
            year: empty_series(&specs.year),
        },
        api_calls: DashboardPeriodMetrics {
            day: empty_series(&specs.day),
            week: empty_series(&specs.week),
            month: empty_series(&specs.month),
            year: empty_series(&specs.year),
        },
        site_totals: DashboardSiteTotals {
            text_count: 0,
            article_count: 0,
            comment_count: 0,
            album_count: 0,
            friend_count: 0,
        },
    }
}

struct Periods {
    day: PeriodSpec,
    week: PeriodSpec,
    month: PeriodSpec,
    year: PeriodSpec,
}

fn build_period_specs(now: NaiveDateTime) -> AppResult<Periods> {
    let hour_base = now
        .with_minute(0)
        .and_then(|v| v.with_second(0))
        .and_then(|v| v.with_nanosecond(0))
        .ok_or_else(|| AppError::internal("Failed to normalize hour-base timestamp"))?;
    let day_start = hour_base - Duration::hours(23);

    let week_start_date = now.date() - Duration::days(6);
    let week_start = week_start_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::internal("Failed to build week start timestamp"))?;

    let month_start_date = now.date() - Duration::days(29);
    let month_start = month_start_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::internal("Failed to build month start timestamp"))?;

    let year_month_start = NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .ok_or_else(|| AppError::internal("Failed to build year month base"))?;
    let year_start = shift_months(year_month_start, -11)?;

    Ok(Periods {
        day: PeriodSpec {
            start: day_start,
            labels: (0..24)
                .map(|index| {
                    (day_start + Duration::hours(index))
                        .format("%H:00")
                        .to_string()
                })
                .collect(),
            unit: Unit::Hour,
        },
        week: PeriodSpec {
            start: week_start,
            labels: (0..7)
                .map(|index| {
                    (week_start + Duration::days(index))
                        .format("%m-%d")
                        .to_string()
                })
                .collect(),
            unit: Unit::Day,
        },
        month: PeriodSpec {
            start: month_start,
            labels: (0..30)
                .map(|index| {
                    (month_start + Duration::days(index))
                        .format("%m-%d")
                        .to_string()
                })
                .collect(),
            unit: Unit::Day,
        },
        year: PeriodSpec {
            start: year_start,
            labels: (0..12)
                .filter_map(|index| shift_months(year_start, index).ok())
                .map(|value| value.format("%Y-%m").to_string())
                .collect(),
            unit: Unit::Month,
        },
    })
}

fn shift_months(value: NaiveDateTime, offset: i32) -> AppResult<NaiveDateTime> {
    let total_months = value.year() * 12 + (value.month() as i32 - 1) + offset;
    let year = total_months.div_euclid(12);
    let month = total_months.rem_euclid(12) + 1;
    NaiveDate::from_ymd_opt(year, month as u32, 1)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .ok_or_else(|| AppError::internal("Failed to shift months"))
}

fn bucket_index(spec: &PeriodSpec, timestamp: NaiveDateTime) -> Option<usize> {
    if timestamp < spec.start {
        return None;
    }

    let size = spec.labels.len();
    if size == 0 {
        return None;
    }

    let index = match spec.unit {
        Unit::Hour => {
            let delta = timestamp - spec.start;
            (delta.num_seconds() / 3600) as i64
        }
        Unit::Day => (timestamp.date() - spec.start.date()).num_days(),
        Unit::Month => {
            ((timestamp.year() - spec.start.year()) * 12
                + (timestamp.month() as i32 - spec.start.month() as i32)) as i64
        }
    };

    if index < 0 || index >= size as i64 {
        return None;
    }

    Some(index as usize)
}

fn build_bucketed_series(
    spec: &PeriodSpec,
    buckets: &[(NaiveDateTime, i64)],
    total: i64,
) -> DashboardSeries {
    let mut values = vec![0_i64; spec.labels.len()];

    for (timestamp, value) in buckets {
        if let Some(index) = bucket_index(spec, *timestamp) {
            values[index] = (*value).max(0);
        }
    }

    DashboardSeries {
        labels: spec.labels.clone(),
        total: total.max(0),
        values,
    }
}

fn empty_series(spec: &PeriodSpec) -> DashboardSeries {
    DashboardSeries {
        labels: spec.labels.clone(),
        values: vec![0; spec.labels.len()],
        total: 0,
    }
}

async fn build_visit_series(state: &AppState, spec: &PeriodSpec) -> AppResult<DashboardSeries> {
    let granularity = pg_date_trunc_granularity(spec.unit);
    let bucket_sql = format!(
        r#"
        SELECT bucket, COUNT(DISTINCT ip)::bigint AS value
        FROM (
            SELECT date_trunc('{granularity}', create_time) AS bucket, ip
            FROM comment
            WHERE create_time >= $1 AND ip IS NOT NULL AND ip <> ''
            UNION ALL
            SELECT date_trunc('{granularity}', create_time) AS bucket, ip
            FROM friend_apply
            WHERE create_time >= $1 AND ip IS NOT NULL AND ip <> ''
        ) t
        GROUP BY bucket
        ORDER BY bucket ASC
        "#
    );
    let buckets = sqlx::query_as::<_, (NaiveDateTime, i64)>(&bucket_sql)
        .bind(spec.start)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to aggregate visit buckets: {error}"))
        })?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(DISTINCT ip)::bigint
        FROM (
            SELECT ip FROM comment WHERE create_time >= $1 AND ip IS NOT NULL AND ip <> ''
            UNION
            SELECT ip FROM friend_apply WHERE create_time >= $1 AND ip IS NOT NULL AND ip <> ''
        ) t
        "#,
    )
    .bind(spec.start)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to aggregate visit totals: {error}")))?;

    Ok(build_bucketed_series(spec, &buckets, total))
}

async fn build_api_count_series(state: &AppState, spec: &PeriodSpec) -> AppResult<DashboardSeries> {
    let granularity = pg_date_trunc_granularity(spec.unit);
    let bucket_sql = format!(
        r#"
        SELECT bucket, COUNT(*)::bigint AS value
        FROM (
            SELECT date_trunc('{granularity}', create_time) AS bucket FROM comment WHERE create_time >= $1
            UNION ALL
            SELECT date_trunc('{granularity}', create_time) AS bucket FROM friend_apply WHERE create_time >= $1
            UNION ALL
            SELECT date_trunc('{granularity}', create_time) AS bucket FROM daily WHERE create_time >= $1
            UNION ALL
            SELECT date_trunc('{granularity}', create_time) AS bucket FROM album WHERE create_time >= $1
            UNION ALL
            SELECT date_trunc('{granularity}', create_time) AS bucket FROM project WHERE create_time >= $1
            UNION ALL
            SELECT date_trunc('{granularity}', create_time) AS bucket FROM singlepage WHERE create_time >= $1
            UNION ALL
            SELECT date_trunc('{granularity}', create_time) AS bucket FROM friends WHERE create_time >= $1
            UNION ALL
            SELECT date_trunc('{granularity}', "lastEditTime") AS bucket FROM article WHERE "lastEditTime" >= $1
        ) t
        GROUP BY bucket
        ORDER BY bucket ASC
        "#
    );

    let buckets = sqlx::query_as::<_, (NaiveDateTime, i64)>(&bucket_sql)
        .bind(spec.start)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to aggregate api buckets: {error}")))?;

    let total = buckets
        .iter()
        .map(|(_, value)| (*value).max(0))
        .sum::<i64>();
    Ok(build_bucketed_series(spec, &buckets, total))
}

fn pg_date_trunc_granularity(unit: Unit) -> &'static str {
    match unit {
        Unit::Hour => "hour",
        Unit::Day => "day",
        Unit::Month => "month",
    }
}

async fn sum_text_length(state: &AppState, sql: &str) -> AppResult<i64> {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(&state.db_pool)
        .await
        .map(|value| value.max(0))
        .map_err(|error| AppError::internal(format!("Failed to sum text length: {error}")))
}

async fn count_table_rows(state: &AppState, table: &str) -> AppResult<i64> {
    let sql = format!("SELECT COUNT(*)::bigint FROM {table}");
    sqlx::query_scalar::<_, i64>(&sql)
        .fetch_one(&state.db_pool)
        .await
        .map(|value| value.max(0))
        .map_err(|error| AppError::internal(format!("Failed to count table `{table}`: {error}")))
}

async fn build_site_totals(state: &AppState) -> AppResult<DashboardSiteTotals> {
    let mut text_count = 0_i64;
    text_count +=
        sum_text_length(state, "SELECT COALESCE(SUM(char_length(COALESCE(title, '')) + char_length(COALESCE(content, ''))), 0)::bigint FROM article").await?;
    text_count +=
        sum_text_length(state, "SELECT COALESCE(SUM(char_length(COALESCE(title, '')) + char_length(COALESCE(content, ''))), 0)::bigint FROM daily").await?;
    text_count +=
        sum_text_length(state, "SELECT COALESCE(SUM(char_length(COALESCE(title, '')) + char_length(COALESCE(content, ''))), 0)::bigint FROM singlepage").await?;
    text_count +=
        sum_text_length(state, "SELECT COALESCE(SUM(char_length(COALESCE(title, '')) + char_length(COALESCE(description, '')) + char_length(COALESCE(content, ''))), 0)::bigint FROM project").await?;
    text_count += sum_text_length(
        state,
        "SELECT COALESCE(SUM(char_length(COALESCE(content, ''))), 0)::bigint FROM comment",
    )
    .await?;

    Ok(DashboardSiteTotals {
        text_count: text_count.max(0),
        article_count: count_table_rows(state, "article").await?,
        comment_count: count_table_rows(state, "comment").await?,
        album_count: count_table_rows(state, "album").await?,
        friend_count: count_table_rows(state, "friends").await?,
    })
}
