from __future__ import annotations

from datetime import datetime, timedelta

from sqlalchemy import case, desc, select
from sqlalchemy.orm import Session

from app.core.simple_cache import cache
from app.models.friend import Friend
from app.models.friend_apply import FriendApply
from app.schemas.friend import FriendApplyRequest
from app.schemas.friend import FriendItem

FRIENDS_CACHE_KEY = "friends:list"
FRIENDS_CACHE_TTL_SECONDS = 30
FRIEND_APPLY_RATE_LIMIT_SECONDS = 300
FRIEND_APPLY_RATE_LIMIT_PER_IP = 5
FRIEND_APPLY_DUPLICATE_WINDOW_DAYS = 14
FRIEND_STATUS_VALUES = {"ok", "missing", "blocked"}


def _map_friend_item(row: Friend) -> FriendItem:
    return FriendItem(
        id=row.id,
        title=row.title,
        description=row.description,
        category=row.category,
        favicon=row.favicon,
        url=row.url,
        status=_normalize_friend_status(row.status),
        create_time=row.create_time,
    )


def _normalize_optional_text(value: str | None) -> str | None:
    if value is None:
        return None
    normalized = value.strip()
    return normalized or None


def _normalize_friend_status(value: object) -> str:
    if isinstance(value, int):
        return {
            1: "ok",
            2: "missing",
            3: "blocked",
            0: "missing",
            -1: "blocked",
        }.get(value, "ok")

    normalized = str(value or "").strip().lower()
    if normalized in FRIEND_STATUS_VALUES:
        return normalized

    if normalized.isdigit():
        return _normalize_friend_status(int(normalized))

    return "ok"


def list_friends(session: Session) -> list[FriendItem]:
    cached = cache.get(FRIENDS_CACHE_KEY)
    if cached is not None:
        return [item.model_copy(deep=True) for item in cached]

    status_order = case(
        (Friend.status == "ok", 0),
        (Friend.status == "missing", 1),
        (Friend.status == "blocked", 2),
        else_=3,
    )
    stmt = select(Friend).order_by(status_order, desc(Friend.create_time), desc(Friend.id))
    result = session.execute(stmt)
    rows = result.scalars().all()
    mapped = [_map_friend_item(row) for row in rows]
    cache.set(FRIENDS_CACHE_KEY, mapped, FRIENDS_CACHE_TTL_SECONDS)
    return [item.model_copy(deep=True) for item in mapped]


def create_friend_apply(
    session: Session,
    payload: FriendApplyRequest,
    *,
    ip_address: str | None = None,
    user_agent: str | None = None,
) -> int:
    now = datetime.utcnow()
    normalized_url = payload.site_url.strip()

    existing_friend_stmt = select(Friend.id).where(Friend.url == normalized_url).limit(1)
    existing_friend = session.execute(existing_friend_stmt).scalars().first()
    if existing_friend is not None:
        raise ValueError("This site already exists in friend list")

    duplicate_cutoff = now - timedelta(days=FRIEND_APPLY_DUPLICATE_WINDOW_DAYS)
    duplicate_apply_stmt = (
        select(FriendApply.id)
        .where(
            FriendApply.site_url == normalized_url,
            FriendApply.create_time >= duplicate_cutoff,
            FriendApply.status.in_(["pending", "approved"]),
        )
        .order_by(desc(FriendApply.id))
        .limit(1)
    )
    duplicate_apply = session.execute(duplicate_apply_stmt).scalars().first()
    if duplicate_apply is not None:
        raise ValueError("This site has already submitted an application recently")

    normalized_ip = _normalize_optional_text(ip_address)
    if normalized_ip:
        rate_limit_cutoff = now - timedelta(seconds=FRIEND_APPLY_RATE_LIMIT_SECONDS)
        rate_limit_stmt = (
            select(FriendApply.id)
            .where(
                FriendApply.ip == normalized_ip,
                FriendApply.create_time >= rate_limit_cutoff,
            )
            .order_by(desc(FriendApply.id))
            .limit(FRIEND_APPLY_RATE_LIMIT_PER_IP)
        )
        recent_rows = session.execute(rate_limit_stmt).scalars().all()
        if len(recent_rows) >= FRIEND_APPLY_RATE_LIMIT_PER_IP:
            raise ValueError("Too many friend applications, please try later")

    row = FriendApply(
        site_title=payload.site_title.strip(),
        site_url=normalized_url,
        site_description=_normalize_optional_text(payload.site_description),
        site_icon=_normalize_optional_text(payload.site_icon),
        contact=_normalize_optional_text(payload.contact),
        status="pending",
        ip=normalized_ip[:50] if normalized_ip else None,
        user_agent=(_normalize_optional_text(user_agent) or "")[:255] or None,
    )
    session.add(row)
    session.commit()
    session.refresh(row)
    return int(row.id)
