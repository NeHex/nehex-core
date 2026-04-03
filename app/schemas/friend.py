from __future__ import annotations

from datetime import datetime
from urllib.parse import urlparse
from typing import Literal, Optional

from pydantic import BaseModel, ConfigDict, Field, field_validator

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


class FriendApplyRequest(BaseModel):
    site_title: str = Field(min_length=1, max_length=255)
    site_url: str = Field(min_length=1, max_length=500)
    site_description: Optional[str] = Field(default=None, max_length=1000)
    site_icon: Optional[str] = Field(default=None, max_length=500)
    contact: Optional[str] = Field(default=None, max_length=255)

    @field_validator("site_title")
    @classmethod
    def normalize_site_title(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("site_title is required")
        return normalized

    @field_validator("site_url")
    @classmethod
    def normalize_site_url(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("site_url is required")

        parsed = urlparse(normalized)
        if parsed.scheme not in {"http", "https"} or not parsed.netloc:
            raise ValueError("URL must start with http:// or https://")
        return normalized

    @field_validator("site_icon")
    @classmethod
    def normalize_site_icon(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None

        normalized = value.strip()
        if not normalized:
            return None

        parsed = urlparse(normalized)
        if parsed.scheme not in {"http", "https"} or not parsed.netloc:
            raise ValueError("URL must start with http:// or https://")
        return normalized

    @field_validator("site_description", "contact")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class FriendApplyResponse(BaseModel):
    success: bool = True
    message: str
    application_id: int
