from __future__ import annotations

from typing import Optional

from sqlalchemy import String, cast, delete, desc, func, or_, select
from sqlalchemy.orm import Session

from app.models.comment import Comment
from app.schemas.comment import CommentItem
from app.services.mail_service import send_comment_notification_mails
from app.services.admin_service_parts.common import (
    _invalidate_comment_cache,
    _map_comment_item,
    _normalize_optional_text,
)


def list_admin_comments(
    session: Session,
    keyword: Optional[str] = None,
    *,
    page: int = 1,
    size: int = 20,
) -> tuple[list[CommentItem], int, int, int, int]:
    normalized_page = max(1, int(page))
    normalized_size = max(1, min(100, int(size)))
    offset = (normalized_page - 1) * normalized_size
    stmt = select(Comment)
    count_stmt = select(func.count(Comment.id))

    normalized_keyword = (keyword or "").strip()
    if normalized_keyword:
        like_pattern = f"%{normalized_keyword}%"
        search_condition = or_(
            Comment.content.like(like_pattern),
            Comment.nickname.like(like_pattern),
            Comment.target_type.like(like_pattern),
            cast(Comment.target_id, String).like(like_pattern),
        )
        stmt = stmt.where(search_condition)
        count_stmt = count_stmt.where(search_condition)

    total = int(session.execute(count_stmt).scalar() or 0)
    if total <= 0:
        return [], normalized_page, normalized_size, 0, 0

    stmt = stmt.order_by(desc(Comment.create_time), desc(Comment.id)).offset(offset).limit(
        normalized_size,
    )
    rows = session.execute(stmt).scalars().all()
    total_pages = (total + normalized_size - 1) // normalized_size
    return (
        [_map_comment_item(row) for row in rows],
        normalized_page,
        normalized_size,
        total,
        total_pages,
    )


def create_admin_comment(
    session: Session,
    *,
    parent_id: int,
    target_type: str,
    target_id: int,
    content: str,
    nickname: str,
    email: Optional[str] = None,
    website: Optional[str] = None,
    status: int = 1,
) -> CommentItem:
    normalized_parent_id = max(0, int(parent_id))
    parent_comment: Comment | None = None
    if normalized_parent_id > 0:
        parent_comment = session.get(Comment, normalized_parent_id)

    row = Comment(
        parent_id=normalized_parent_id,
        target_type=target_type.strip().lower(),
        target_id=int(target_id),
        content=content.strip(),
        nickname=nickname.strip(),
        email=_normalize_optional_text(email),
        website=_normalize_optional_text(website),
        status=1 if int(status) > 0 else 0,
    )
    session.add(row)
    session.commit()
    session.refresh(row)
    try:
        send_comment_notification_mails(
            session=session,
            comment=row,
            parent_comment=parent_comment,
        )
    except Exception:
        # Mail notifications should not block admin comment operations.
        session.rollback()
    _invalidate_comment_cache(row.target_type, row.target_id)
    return _map_comment_item(row)


def update_admin_comment(
    session: Session,
    comment_id: int,
    *,
    parent_id: Optional[int] = None,
    content: Optional[str] = None,
    nickname: Optional[str] = None,
    email: Optional[str] = None,
    website: Optional[str] = None,
    status: Optional[int] = None,
) -> Optional[CommentItem]:
    stmt = select(Comment).where(Comment.id == comment_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    if parent_id is not None:
        row.parent_id = max(0, int(parent_id))
    if content is not None:
        row.content = content.strip()
    if nickname is not None:
        row.nickname = nickname.strip()
    if email is not None:
        row.email = _normalize_optional_text(email)
    if website is not None:
        row.website = _normalize_optional_text(website)
    if status is not None:
        row.status = 1 if int(status) > 0 else 0

    session.commit()
    session.refresh(row)
    _invalidate_comment_cache(row.target_type, row.target_id)
    return _map_comment_item(row)


def _collect_comment_descendant_ids(session: Session, root_id: int) -> list[int]:
    all_ids: list[int] = [root_id]
    frontier: list[int] = [root_id]

    while frontier:
        stmt = select(Comment.id).where(Comment.parent_id.in_(frontier))
        children = list(session.execute(stmt).scalars().all())
        if not children:
            break
        all_ids.extend(children)
        frontier = children

    return all_ids


def delete_admin_comment(session: Session, comment_id: int) -> bool:
    stmt = select(Comment).where(Comment.id == comment_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return False

    delete_ids = _collect_comment_descendant_ids(session, comment_id)
    if delete_ids:
        session.execute(delete(Comment).where(Comment.id.in_(delete_ids)))
    session.commit()
    _invalidate_comment_cache(row.target_type, row.target_id)
    return True
