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
    like_count: int
    lastEditTime: datetime
    tag: Optional[str] = None
    top: int
    status: int = Field(default=1, ge=0, le=1)
    content: Optional[str] = None


class ArticlePagination(BaseModel):
    page: int = Field(ge=1)
    size: int = Field(ge=1)
    total: int = Field(ge=0)
    total_pages: int = Field(ge=0)


class ArticleListResponse(BaseModel):
    data: list[ArticleItem]
    pagination: ArticlePagination


class ArticleDetailResponse(BaseModel):
    data: ArticleItem
