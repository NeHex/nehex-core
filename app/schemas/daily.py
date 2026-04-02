from __future__ import annotations

from datetime import datetime
from typing import Optional

from pydantic import BaseModel


class DailyItem(BaseModel):
    id: int
    title: str
    content: Optional[str] = None
    create_time: datetime
    weather: Optional[str] = None


class DailyListResponse(BaseModel):
    data: list[DailyItem]
