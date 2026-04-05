from __future__ import annotations

from datetime import datetime
from typing import Any
from typing import Optional

from pydantic import BaseModel, Field

from app.models.setting import SettingType


class SettingItem(BaseModel):
    setting_key: str
    setting_type: SettingType
    setting_content: Any = Field(default=None)
    description: Optional[str] = None
    updated_at: datetime
    created_at: datetime


class SettingListResponse(BaseModel):
    data: list[SettingItem]


class ThemeSettingData(BaseModel):
    active_profile: str
    profiles: dict[str, dict[str, Any]]
    current: dict[str, Any]


class ThemeSettingResponse(BaseModel):
    data: ThemeSettingData
