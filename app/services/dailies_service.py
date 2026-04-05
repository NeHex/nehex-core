from __future__ import annotations

from sqlalchemy import desc, select
from sqlalchemy.orm import Session

from app.core.simple_cache import cache
from app.models.daily import Daily
from app.schemas.daily import DailyItem

DAILIES_CACHE_KEY = "dailies:list"
DAILIES_CACHE_TTL_SECONDS = 20


def _map_daily_item(row: Daily) -> DailyItem:
    return DailyItem(
        id=row.id,
        title=row.title,
        content=row.content,
        create_time=row.create_time,
        weather=row.weather,
    )


def list_dailies(session: Session) -> list[DailyItem]:
    cached = cache.get(DAILIES_CACHE_KEY)
    if cached is not None:
        return [item.model_copy(deep=True) for item in cached]

    stmt = select(Daily).order_by(
        desc(Daily.create_time),
        desc(Daily.id),
    )
    result = session.execute(stmt)
    rows = result.scalars().all()
    mapped = [_map_daily_item(row) for row in rows]
    cache.set(DAILIES_CACHE_KEY, mapped, DAILIES_CACHE_TTL_SECONDS)
    return [item.model_copy(deep=True) for item in mapped]


def get_daily_by_id(session: Session, daily_id: int) -> DailyItem | None:
    stmt = select(Daily).where(Daily.id == daily_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None
    return _map_daily_item(row)
