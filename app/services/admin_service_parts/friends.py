from __future__ import annotations

from typing import Optional

from sqlalchemy import String, case, cast, desc, or_, select
from sqlalchemy.orm import Session

from app.models.friend import Friend
from app.models.friend_apply import FriendApply
from app.schemas.friend import FriendApplyItem, FriendItem
from app.services.admin_service_parts.common import (
    FRIEND_APPLY_STATUS_VALUES,
    _invalidate_friends_cache,
    _map_friend_apply_item,
    _map_friend_item,
    _normalize_friend_apply_status,
    _normalize_friend_status,
    _normalize_optional_text,
)


def list_admin_friends(session: Session, keyword: Optional[str] = None) -> list[FriendItem]:
    stmt = select(Friend)

    normalized_keyword = (keyword or "").strip()
    if normalized_keyword:
        like_pattern = f"%{normalized_keyword}%"
        stmt = stmt.where(
            or_(
                Friend.title.like(like_pattern),
                Friend.description.like(like_pattern),
                Friend.category.like(like_pattern),
                Friend.url.like(like_pattern),
                Friend.status.like(like_pattern),
                cast(Friend.id, String).like(like_pattern),
            ),
        )

    status_order = case(
        (Friend.status == "ok", 0),
        (Friend.status == "missing", 1),
        (Friend.status == "blocked", 2),
        else_=3,
    )
    stmt = stmt.order_by(status_order, desc(Friend.create_time), desc(Friend.id))
    rows = session.execute(stmt).scalars().all()
    return [_map_friend_item(row) for row in rows]


def create_admin_friend(
    session: Session,
    *,
    title: str,
    description: Optional[str] = None,
    category: str = "default",
    favicon: Optional[str] = None,
    url: str,
    status: str = "ok",
) -> FriendItem:
    normalized_url = url.strip()
    if not normalized_url:
        raise ValueError("url is required")

    existing = session.execute(
        select(Friend.id).where(Friend.url == normalized_url).limit(1),
    ).scalars().first()
    if existing is not None:
        raise ValueError("Friend URL already exists")

    row = Friend(
        title=title.strip(),
        description=_normalize_optional_text(description),
        category=_normalize_optional_text(category) or "default",
        favicon=_normalize_optional_text(favicon),
        url=normalized_url,
        status=_normalize_friend_status(status),
    )
    session.add(row)
    session.commit()
    session.refresh(row)
    _invalidate_friends_cache()
    return _map_friend_item(row)


def update_admin_friend(
    session: Session,
    friend_id: int,
    *,
    title: Optional[str] = None,
    description: Optional[str] = None,
    category: Optional[str] = None,
    favicon: Optional[str] = None,
    url: Optional[str] = None,
    status: Optional[str] = None,
) -> Optional[FriendItem]:
    row = session.execute(
        select(Friend).where(Friend.id == friend_id).limit(1),
    ).scalars().first()
    if row is None:
        return None

    if title is not None:
        row.title = title.strip()
    if description is not None:
        row.description = _normalize_optional_text(description)
    if category is not None:
        row.category = _normalize_optional_text(category) or "default"
    if favicon is not None:
        row.favicon = _normalize_optional_text(favicon)
    if url is not None:
        normalized_url = url.strip()
        if not normalized_url:
            raise ValueError("url cannot be empty")
        existing = session.execute(
            select(Friend.id).where(
                Friend.url == normalized_url,
                Friend.id != friend_id,
            ).limit(1),
        ).scalars().first()
        if existing is not None:
            raise ValueError("Friend URL already exists")
        row.url = normalized_url
    if status is not None:
        row.status = _normalize_friend_status(status)

    session.commit()
    session.refresh(row)
    _invalidate_friends_cache()
    return _map_friend_item(row)


def delete_admin_friend(session: Session, friend_id: int) -> bool:
    row = session.execute(
        select(Friend).where(Friend.id == friend_id).limit(1),
    ).scalars().first()
    if row is None:
        return False

    session.delete(row)
    session.commit()
    _invalidate_friends_cache()
    return True


def list_admin_friend_applies(
    session: Session,
    *,
    status: Optional[str] = None,
    keyword: Optional[str] = None,
) -> list[FriendApplyItem]:
    stmt = select(FriendApply)

    normalized_status = (status or "").strip().lower()
    if normalized_status:
        if normalized_status not in FRIEND_APPLY_STATUS_VALUES:
            raise ValueError("Invalid friend apply status")
        stmt = stmt.where(FriendApply.status == normalized_status)

    normalized_keyword = (keyword or "").strip()
    if normalized_keyword:
        like_pattern = f"%{normalized_keyword}%"
        stmt = stmt.where(
            or_(
                FriendApply.site_title.like(like_pattern),
                FriendApply.site_url.like(like_pattern),
                FriendApply.site_description.like(like_pattern),
                FriendApply.contact.like(like_pattern),
                FriendApply.status.like(like_pattern),
                FriendApply.ip.like(like_pattern),
                cast(FriendApply.id, String).like(like_pattern),
            ),
        )

    status_order = case(
        (FriendApply.status == "pending", 0),
        (FriendApply.status == "approved", 1),
        (FriendApply.status == "rejected", 2),
        (FriendApply.status == "blocked", 3),
        else_=4,
    )
    stmt = stmt.order_by(status_order, desc(FriendApply.create_time), desc(FriendApply.id))
    rows = session.execute(stmt).scalars().all()
    return [_map_friend_apply_item(row) for row in rows]


def update_admin_friend_apply_status(
    session: Session,
    apply_id: int,
    *,
    status: str,
    create_friend: bool = False,
    friend_category: Optional[str] = None,
) -> Optional[FriendApplyItem]:
    row = session.execute(
        select(FriendApply).where(FriendApply.id == apply_id).limit(1),
    ).scalars().first()
    if row is None:
        return None

    normalized_status = _normalize_friend_apply_status(status)
    row.status = normalized_status

    should_create_friend = normalized_status == "approved" and bool(create_friend)
    if should_create_friend:
        normalized_url = (row.site_url or "").strip()
        if normalized_url:
            existing_friend_id = session.execute(
                select(Friend.id).where(Friend.url == normalized_url).limit(1),
            ).scalars().first()
            if existing_friend_id is None:
                normalized_category = _normalize_optional_text(friend_category) or "friend_apply"
                normalized_title = _normalize_optional_text(row.site_title) or normalized_url
                normalized_description = _normalize_optional_text(row.site_description)
                normalized_icon = _normalize_optional_text(row.site_icon)

                friend_row = Friend(
                    title=normalized_title[:255],
                    description=(normalized_description[:500] if normalized_description else None),
                    category=normalized_category[:100],
                    favicon=(normalized_icon[:500] if normalized_icon else None),
                    url=normalized_url[:500],
                    status="ok",
                )
                session.add(friend_row)

    session.commit()
    session.refresh(row)
    if should_create_friend:
        _invalidate_friends_cache()
    return _map_friend_apply_item(row)
