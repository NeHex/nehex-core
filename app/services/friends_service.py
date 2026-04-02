from __future__ import annotations

from sqlalchemy import case, desc, select
from sqlalchemy.orm import Session

from app.core.simple_cache import cache
from app.models.friend import Friend
from app.schemas.friend import FriendItem

FRIENDS_CACHE_KEY = "friends:list"
FRIENDS_CACHE_TTL_SECONDS = 30


def _map_friend_item(row: Friend) -> FriendItem:
    return FriendItem(
        id=row.id,
        title=row.title,
        description=row.description,
        category=row.category,
        favicon=row.favicon,
        url=row.url,
        status=row.status,
        create_time=row.create_time,
    )


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
