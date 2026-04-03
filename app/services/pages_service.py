from __future__ import annotations

from sqlalchemy import desc, select
from sqlalchemy.orm import Session

from app.core.simple_cache import cache
from app.models.singlepage import SinglePage
from app.schemas.page import PageItem

PAGES_CACHE_KEY = "pages:list"
PAGES_CACHE_TTL_SECONDS = 20


def _map_page_item(row: SinglePage) -> PageItem:
    return PageItem(
        id=row.id,
        page_key=row.page_key,
        title=row.title,
        cover_image=row.cover_image,
        content=row.content,
        sort=row.sort,
        status=row.status,
        create_time=row.create_time,
        update_time=row.update_time,
    )


def list_pages(session: Session) -> list[PageItem]:
    cached = cache.get(PAGES_CACHE_KEY)
    if cached is not None:
        return [item.model_copy(deep=True) for item in cached]

    stmt = (
        select(SinglePage)
        .where(SinglePage.status == 1)
        .order_by(SinglePage.sort.asc(), desc(SinglePage.update_time), desc(SinglePage.id))
    )
    rows = session.execute(stmt).scalars().all()
    mapped = [_map_page_item(row) for row in rows]
    cache.set(PAGES_CACHE_KEY, mapped, PAGES_CACHE_TTL_SECONDS)
    return [item.model_copy(deep=True) for item in mapped]


def get_page_by_key(session: Session, page_key: str) -> PageItem | None:
    normalized_key = page_key.strip().strip("/")
    if not normalized_key:
        return None

    stmt = (
        select(SinglePage)
        .where(SinglePage.page_key == normalized_key, SinglePage.status == 1)
        .limit(1)
    )
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None
    return _map_page_item(row)
