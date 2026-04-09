from __future__ import annotations

from datetime import datetime
from typing import Optional

from pydantic import BaseModel, Field, field_validator


class CommentItem(BaseModel):
    id: int
    parent_id: int
    target_type: str
    target_id: int
    content: str
    nickname: str
    email: Optional[str] = None
    website: Optional[str] = None
    like_count: int = 0
    status: int = 1
    is_admin: bool = False
    ip: Optional[str] = None
    create_time: datetime
    update_time: datetime
    replies: list["CommentItem"] = Field(default_factory=list)


class CommentListResponse(BaseModel):
    data: list[CommentItem]


class CommentDetailResponse(BaseModel):
    data: CommentItem


class CommentCreateRequest(BaseModel):
    parent_id: int = Field(default=0, ge=0)
    target_type: str = Field(min_length=1, max_length=20)
    target_id: int = Field(ge=1)
    content: str = Field(min_length=1, max_length=1200)
    nickname: str = Field(min_length=1, max_length=100)
    email: Optional[str] = Field(default=None, max_length=255)
    website: Optional[str] = Field(default=None, max_length=255)

    @field_validator("target_type")
    @classmethod
    def normalize_target_type(cls, value: str) -> str:
        return value.strip().lower()

    @field_validator("content")
    @classmethod
    def normalize_content(cls, value: str) -> str:
        normalized = value.replace("\r\n", "\n").strip()
        if not normalized:
            raise ValueError("content is required")
        return normalized

    @field_validator("nickname")
    @classmethod
    def normalize_nickname(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("nickname is required")
        return normalized

    @field_validator("email", "website")
    @classmethod
    def normalize_optional_field(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None
