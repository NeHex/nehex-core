from __future__ import annotations

from datetime import datetime
from typing import Optional

from pydantic import BaseModel


class PageItem(BaseModel):
    id: int
    page_key: str
    title: str
    cover_image: Optional[str] = None
    content: Optional[str] = None
    sort: int
    status: int
    create_time: datetime
    update_time: datetime


class PageListResponse(BaseModel):
    data: list[PageItem]


class PageDetailResponse(BaseModel):
    data: PageItem
