from __future__ import annotations

from datetime import datetime
from typing import Optional

from pydantic import BaseModel, ConfigDict, Field


class AlbumItem(BaseModel):
    model_config = ConfigDict(populate_by_name=True)

    id: int
    title: str
    cover: Optional[str] = None
    class_: str = Field(alias="class")
    like_count: int
    img_urls: Optional[str] = None
    create_time: datetime
    update_time: datetime


class AlbumListResponse(BaseModel):
    data: list[AlbumItem]


class AlbumDetailResponse(BaseModel):
    data: AlbumItem
