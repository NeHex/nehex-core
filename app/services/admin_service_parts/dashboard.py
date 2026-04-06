from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, time, timedelta, timezone
from typing import Any, Literal

from sqlalchemy import func, select
from sqlalchemy.exc import SQLAlchemyError
from sqlalchemy.orm import Session

from app.core.simple_cache import cache
from app.models.album import Album
from app.models.article import Article
from app.models.comment import Comment
from app.models.daily import Daily
from app.models.friend import Friend
from app.models.friend_apply import FriendApply
from app.models.project import Project
from app.models.singlepage import SinglePage

DashboardPeriodKey = Literal["day", "week", "month", "year"]

DASHBOARD_CACHE_KEY = "admin:dashboard:overview:v1"
DASHBOARD_CACHE_TTL_SECONDS = 60


@dataclass(frozen=True)
class PeriodSpec:
    key: DashboardPeriodKey
    start: datetime
    labels: list[str]
    unit: Literal["hour", "day", "month"]


def _normalize_timestamp(value: datetime | None) -> datetime | None:
    if value is None:
        return None
    if value.tzinfo is not None:
        return value.astimezone(timezone.utc).replace(tzinfo=None, microsecond=0)
    return value.replace(microsecond=0)


def _month_start(value: datetime) -> datetime:
    return value.replace(day=1, hour=0, minute=0, second=0, microsecond=0)


def _shift_months(value: datetime, offset: int) -> datetime:
    total_months = (value.year * 12) + (value.month - 1) + offset
    year = total_months // 12
    month = (total_months % 12) + 1
    return datetime(year, month, 1)


def _build_period_specs(now: datetime) -> dict[DashboardPeriodKey, PeriodSpec]:
    hour_base = now.replace(minute=0, second=0, microsecond=0)
    day_start = hour_base - timedelta(hours=23)

    week_start_date = now.date() - timedelta(days=6)
    week_start = datetime.combine(week_start_date, time.min)

    month_start_date = now.date() - timedelta(days=29)
    month_start = datetime.combine(month_start_date, time.min)

    year_start = _shift_months(_month_start(now), -11)

    return {
        "day": PeriodSpec(
            key="day",
            start=day_start,
            labels=[(day_start + timedelta(hours=index)).strftime("%H:00") for index in range(24)],
            unit="hour",
        ),
        "week": PeriodSpec(
            key="week",
            start=week_start,
            labels=[(week_start + timedelta(days=index)).strftime("%m-%d") for index in range(7)],
            unit="day",
        ),
        "month": PeriodSpec(
            key="month",
            start=month_start,
            labels=[(month_start + timedelta(days=index)).strftime("%m-%d") for index in range(30)],
            unit="day",
        ),
        "year": PeriodSpec(
            key="year",
            start=year_start,
            labels=[_shift_months(year_start, index).strftime("%Y-%m") for index in range(12)],
            unit="month",
        ),
    }


def _bucket_index(spec: PeriodSpec, timestamp: datetime) -> int | None:
    if timestamp < spec.start:
        return None

    size = len(spec.labels)
    if size <= 0:
        return None

    if spec.unit == "hour":
        delta = timestamp - spec.start
        index = int(delta.total_seconds() // 3600)
    elif spec.unit == "day":
        index = (timestamp.date() - spec.start.date()).days
    else:
        index = (timestamp.year - spec.start.year) * 12 + (timestamp.month - spec.start.month)

    if index < 0 or index >= size:
        return None
    return index


def _empty_series(spec: PeriodSpec) -> dict[str, Any]:
    return {
        "labels": list(spec.labels),
        "values": [0 for _ in spec.labels],
        "total": 0,
    }


def _earliest_start(specs: dict[DashboardPeriodKey, PeriodSpec]) -> datetime:
    return min(spec.start for spec in specs.values())


def _fetch_visit_events(session: Session, earliest_start: datetime) -> list[tuple[datetime, str]]:
    events: list[tuple[datetime, str]] = []

    for statement in (
        select(Comment.create_time, Comment.ip).where(
            Comment.create_time >= earliest_start,
            Comment.ip.is_not(None),
            Comment.ip != "",
        ),
        select(FriendApply.create_time, FriendApply.ip).where(
            FriendApply.create_time >= earliest_start,
            FriendApply.ip.is_not(None),
            FriendApply.ip != "",
        ),
    ):
        rows = session.execute(statement).all()
        for raw_timestamp, raw_ip in rows:
            normalized_timestamp = _normalize_timestamp(raw_timestamp)
            normalized_ip = str(raw_ip or "").strip()
            if normalized_timestamp is None or not normalized_ip:
                continue
            events.append((normalized_timestamp, normalized_ip))

    return events


def _fetch_api_call_events(session: Session, earliest_start: datetime) -> list[datetime]:
    events: list[datetime] = []

    for column in (
        Comment.create_time,
        FriendApply.create_time,
        Daily.create_time,
        Album.create_time,
        Project.create_time,
        SinglePage.create_time,
        Friend.create_time,
        Article.last_edit_time,
    ):
        rows = session.execute(select(column).where(column >= earliest_start)).scalars().all()
        for raw_timestamp in rows:
            normalized_timestamp = _normalize_timestamp(raw_timestamp)
            if normalized_timestamp is None:
                continue
            events.append(normalized_timestamp)

    return events


def _build_visit_period_series(spec: PeriodSpec, events: list[tuple[datetime, str]]) -> dict[str, Any]:
    bucket_unique_ips: list[set[str]] = [set() for _ in spec.labels]
    period_unique_ips: set[str] = set()

    for timestamp, ip in events:
        index = _bucket_index(spec, timestamp)
        if index is None:
            continue
        bucket_unique_ips[index].add(ip)
        period_unique_ips.add(ip)

    return {
        "labels": list(spec.labels),
        "values": [len(item) for item in bucket_unique_ips],
        "total": len(period_unique_ips),
    }


def _build_count_period_series(spec: PeriodSpec, events: list[datetime]) -> dict[str, Any]:
    values = [0 for _ in spec.labels]

    for timestamp in events:
        index = _bucket_index(spec, timestamp)
        if index is None:
            continue
        values[index] += 1

    return {
        "labels": list(spec.labels),
        "values": values,
        "total": int(sum(values)),
    }


def _sum_text_length(session: Session, model: Any, *columns: Any) -> int:
    if not columns:
        return 0

    total_expr = None
    for column in columns:
        part = func.char_length(func.coalesce(column, ""))
        total_expr = part if total_expr is None else total_expr + part

    value = session.execute(
        select(func.coalesce(func.sum(total_expr), 0)).select_from(model),
    ).scalar_one()
    return int(value or 0)


def _build_site_totals(session: Session) -> dict[str, int]:
    text_count = 0
    text_count += _sum_text_length(session, Article, Article.title, Article.content)
    text_count += _sum_text_length(session, Daily, Daily.title, Daily.content)
    text_count += _sum_text_length(session, SinglePage, SinglePage.title, SinglePage.content)
    text_count += _sum_text_length(session, Project, Project.title, Project.description, Project.content)
    text_count += _sum_text_length(session, Comment, Comment.content)

    article_count = int(session.execute(select(func.count(Article.id))).scalar_one() or 0)
    comment_count = int(session.execute(select(func.count(Comment.id))).scalar_one() or 0)
    album_count = int(session.execute(select(func.count(Album.id))).scalar_one() or 0)

    return {
        "text_count": max(0, text_count),
        "article_count": max(0, article_count),
        "comment_count": max(0, comment_count),
        "album_count": max(0, album_count),
    }


def get_admin_dashboard_data(session: Session) -> dict[str, Any]:
    cached = cache.get(DASHBOARD_CACHE_KEY)
    if isinstance(cached, dict):
        return cached

    now = _normalize_timestamp(datetime.utcnow()) or datetime.utcnow().replace(microsecond=0)
    specs = _build_period_specs(now)
    base_payload = {
        period: _empty_series(spec)
        for period, spec in specs.items()
    }

    try:
        earliest_start = _earliest_start(specs)
        visit_events = _fetch_visit_events(session, earliest_start)
        api_call_events = _fetch_api_call_events(session, earliest_start)

        visit_payload = {
            period: _build_visit_period_series(spec, visit_events)
            for period, spec in specs.items()
        }
        api_call_payload = {
            period: _build_count_period_series(spec, api_call_events)
            for period, spec in specs.items()
        }
        site_totals = _build_site_totals(session)
    except SQLAlchemyError:
        visit_payload = dict(base_payload)
        api_call_payload = dict(base_payload)
        site_totals = {
            "text_count": 0,
            "article_count": 0,
            "comment_count": 0,
            "album_count": 0,
        }

    payload = {
        "visit_ip": visit_payload,
        "api_calls": api_call_payload,
        "site_totals": site_totals,
    }
    cache.set(DASHBOARD_CACHE_KEY, payload, DASHBOARD_CACHE_TTL_SECONDS)
    return payload
