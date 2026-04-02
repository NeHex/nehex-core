from __future__ import annotations

from datetime import datetime
from typing import Optional

from pydantic import BaseModel, ConfigDict, Field


class ArticleItem(BaseModel):
    model_config = ConfigDict(populate_by_name=True)

    id: int
    title: str
    articleTopImage: Optional[str] = None
    class_: str = Field(alias="class")
    read: int
    lastEditTime: datetime
    tag: Optional[str] = None
    top: int
    content: Optional[str] = None


class ArticleListResponse(BaseModel):
    data: list[ArticleItem]


class ArticleDetailResponse(BaseModel):
    data: ArticleItem
