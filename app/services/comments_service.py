from __future__ import annotations

import re
from datetime import datetime, timedelta

from sqlalchemy import and_, asc, desc, func, select, update
from sqlalchemy.orm import Session

from app.core.simple_cache import cache
from app.models.comment import Comment
from app.schemas.comment import CommentCreateRequest, CommentItem

SUPPORTED_TARGET_TYPES = {"article", "album", "singlepage"}

COMMENT_CACHE_TTL_SECONDS = 8
COMMENT_RATE_LIMIT_WINDOW_SECONDS = 45
COMMENT_RATE_LIMIT_MAX_PER_IP = 5
COMMENT_DUPLICATE_WINDOW_SECONDS = 600
COMMENT_MAX_LINKS = 8

SUSPICIOUS_PATTERNS = [
    "<script",
    "</script",
    "javascript:",
    "onerror=",
    "onload=",
    " union select ",
    " drop table ",
    " delete from ",
]


def _comment_list_cache_key(target_type: str, target_id: int, status: int) -> str:
    return f"comments:list:{target_type}:{target_id}:{status}"


def _invalidate_comment_cache(target_type: str, target_id: int) -> None:
    cache.delete_prefix(f"comments:list:{target_type}:{target_id}:")


def _normalize_optional_text(value: str | None) -> str | None:
    if value is None:
        return None
    normalized = value.strip()
    return normalized or None


def _normalize_content(value: str) -> str:
    return re.sub(r"\s+", " ", value.replace("\r\n", "\n")).strip()


def _count_links(content: str) -> int:
    return len(re.findall(r"https?://|www\\.", content, flags=re.IGNORECASE))


def _contains_suspicious_payload(content: str) -> bool:
    lowered = f" {content.lower()} "
    return any(pattern in lowered for pattern in SUSPICIOUS_PATTERNS)


def _looks_like_spam(content: str) -> bool:
    if re.search(r"(.)\1{39,}", content):
        return True

    if len(content) >= 30 and len(set(content)) <= 3:
        return True

    if _count_links(content) > COMMENT_MAX_LINKS:
        return True

    return False


def _map_comment_item(row: Comment) -> CommentItem:
    return CommentItem(
        id=row.id,
        parent_id=row.parent_id,
        target_type=row.target_type,
        target_id=row.target_id,
        content=row.content,
        nickname=row.nickname,
        email=row.email,
        website=row.website,
        like_count=row.like_count,
        status=row.status,
        ip=row.ip,
        create_time=row.create_time,
        update_time=row.update_time,
        replies=[],
    )


def _build_comment_tree(comments: list[CommentItem]) -> list[CommentItem]:
    roots: list[CommentItem] = []
    item_map = {item.id: item for item in comments}

    for item in comments:
        if item.parent_id == 0:
            roots.append(item)
            continue

        parent = item_map.get(item.parent_id)
        if parent is None:
            roots.append(item)
            continue

        parent.replies.append(item)

    def sort_items(items: list[CommentItem]) -> None:
        items.sort(key=lambda comment: (comment.create_time, comment.id))
        for comment in items:
            if comment.replies:
                sort_items(comment.replies)

    sort_items(roots)
    return roots


def _validate_comment_payload(
    session: Session,
    payload: CommentCreateRequest,
    ip_address: str | None,
) -> None:
    normalized_content = _normalize_content(payload.content)
    if _contains_suspicious_payload(normalized_content):
        raise ValueError("Comment contains blocked content")

    if _looks_like_spam(normalized_content):
        raise ValueError("Comment looks like spam")

    now = datetime.utcnow()

    if ip_address:
        rate_limit_cutoff = now - timedelta(seconds=COMMENT_RATE_LIMIT_WINDOW_SECONDS)
        rate_stmt = (
            select(func.count(Comment.id))
            .where(
                Comment.ip == ip_address,
                Comment.create_time >= rate_limit_cutoff,
            )
            .limit(1)
        )
        current_count = int(session.execute(rate_stmt).scalar() or 0)
        if current_count >= COMMENT_RATE_LIMIT_MAX_PER_IP:
            raise ValueError("Too many comment submissions, please try later")

    duplicate_cutoff = now - timedelta(seconds=COMMENT_DUPLICATE_WINDOW_SECONDS)
    duplicate_conditions = [
        Comment.target_type == payload.target_type,
        Comment.target_id == payload.target_id,
        Comment.parent_id == (payload.parent_id if payload.parent_id > 0 else 0),
        Comment.nickname == payload.nickname,
        Comment.content == normalized_content,
        Comment.create_time >= duplicate_cutoff,
    ]

    if ip_address:
        duplicate_conditions.append(Comment.ip == ip_address)

    duplicate_stmt = (
        select(Comment.id)
        .where(and_(*duplicate_conditions))
        .order_by(desc(Comment.id))
        .limit(1)
    )
    duplicate_id = session.execute(duplicate_stmt).scalars().first()
    if duplicate_id is not None:
        raise ValueError("Duplicate comment detected")


def list_comments(
    session: Session,
    target_type: str,
    target_id: int,
    status: int = 1,
) -> list[CommentItem]:
    cache_key = _comment_list_cache_key(target_type, target_id, status)
    cached = cache.get(cache_key)
    if cached is not None:
        return [item.model_copy(deep=True) for item in cached]

    stmt = (
        select(Comment)
        .where(
            Comment.target_type == target_type,
            Comment.target_id == target_id,
            Comment.status == status,
        )
        .order_by(asc(Comment.create_time), asc(Comment.id))
    )
    result = session.execute(stmt)
    rows = result.scalars().all()

    mapped = [_map_comment_item(row) for row in rows]
    tree = _build_comment_tree(mapped)
    cache.set(cache_key, tree, COMMENT_CACHE_TTL_SECONDS)
    return [item.model_copy(deep=True) for item in tree]


def create_comment(
    session: Session,
    payload: CommentCreateRequest,
    ip_address: str | None = None,
) -> CommentItem:
    parent_id = payload.parent_id if payload.parent_id > 0 else 0

    if parent_id > 0:
        parent_stmt = (
            select(Comment)
            .where(
                Comment.id == parent_id,
                Comment.target_type == payload.target_type,
                Comment.target_id == payload.target_id,
            )
            .limit(1)
        )
        parent_row = session.execute(parent_stmt).scalars().first()
        if parent_row is None:
            raise ValueError("Invalid parent_id for this target")

    normalized_content = _normalize_content(payload.content)
    payload = payload.model_copy(update={"content": normalized_content, "parent_id": parent_id})
    _validate_comment_payload(session=session, payload=payload, ip_address=ip_address)

    new_comment = Comment(
        parent_id=parent_id,
        target_type=payload.target_type,
        target_id=payload.target_id,
        content=normalized_content,
        nickname=payload.nickname.strip(),
        email=_normalize_optional_text(payload.email),
        website=_normalize_optional_text(payload.website),
        ip=_normalize_optional_text(ip_address),
    )

    session.add(new_comment)
    session.commit()
    session.refresh(new_comment)

    _invalidate_comment_cache(payload.target_type, payload.target_id)
    return _map_comment_item(new_comment)


def like_comment(session: Session, comment_id: int) -> CommentItem | None:
    stmt = (
        select(Comment)
        .where(Comment.id == comment_id, Comment.status == 1)
        .limit(1)
    )
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    session.execute(
        update(Comment)
        .where(Comment.id == comment_id)
        .values(like_count=Comment.like_count + 1),
    )
    session.commit()

    refreshed = session.execute(stmt).scalars().first()
    if refreshed is None:
        return None

    _invalidate_comment_cache(refreshed.target_type, refreshed.target_id)
    return _map_comment_item(refreshed)


def get_comment_by_id(session: Session, comment_id: int) -> CommentItem | None:
    stmt = select(Comment).where(Comment.id == comment_id).limit(1)
    result = session.execute(stmt)
    row = result.scalars().first()
    if row is None:
        return None
    return _map_comment_item(row)


def get_latest_comments(
    session: Session,
    target_type: str,
    target_id: int,
    limit: int = 50,
) -> list[CommentItem]:
    stmt = (
        select(Comment)
        .where(
            Comment.target_type == target_type,
            Comment.target_id == target_id,
            Comment.status == 1,
        )
        .order_by(desc(Comment.create_time), desc(Comment.id))
        .limit(limit)
    )
    result = session.execute(stmt)
    rows = result.scalars().all()
    mapped = [_map_comment_item(row) for row in rows]
    mapped.reverse()
    return _build_comment_tree(mapped)
