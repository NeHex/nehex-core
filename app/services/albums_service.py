from __future__ import annotations

from sqlalchemy import desc, select
from sqlalchemy.orm import Session

from app.core.simple_cache import cache
from app.models.album import Album
from app.schemas.album import AlbumItem

ALBUMS_CACHE_KEY = "albums:list"
ALBUMS_CACHE_TTL_SECONDS = 20


def _map_album_item(row: Album) -> AlbumItem:
    return AlbumItem(
        id=row.id,
        title=row.title,
        cover=row.cover,
        class_=row.album_class,
        like_count=row.like_count,
        img_urls=row.img_urls,
        create_time=row.create_time,
        update_time=row.update_time,
    )


def list_albums(session: Session) -> list[AlbumItem]:
    cached = cache.get(ALBUMS_CACHE_KEY)
    if cached is not None:
        return [item.model_copy(deep=True) for item in cached]

    stmt = select(Album).order_by(
        desc(Album.update_time),
        desc(Album.id),
    )
    result = session.execute(stmt)
    rows = result.scalars().all()
    mapped = [_map_album_item(row) for row in rows]
    cache.set(ALBUMS_CACHE_KEY, mapped, ALBUMS_CACHE_TTL_SECONDS)
    return [item.model_copy(deep=True) for item in mapped]


def get_album_by_id(session: Session, album_id: int) -> AlbumItem | None:
    stmt = select(Album).where(Album.id == album_id).limit(1)
    result = session.execute(stmt)
    row = result.scalars().first()
    if row is None:
        return None
    return _map_album_item(row)
