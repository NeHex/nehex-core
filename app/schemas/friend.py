from __future__ import annotations

from datetime import datetime
from typing import Literal, Optional

from pydantic import BaseModel, ConfigDict

FriendStatus = Literal["ok", "missing", "blocked"]


class FriendItem(BaseModel):
    model_config = ConfigDict(populate_by_name=True)

    id: int
    title: str
    description: Optional[str] = None
    category: str
    favicon: Optional[str] = None
    url: str
    status: FriendStatus
    create_time: datetime


class FriendListResponse(BaseModel):
    data: list[FriendItem]
