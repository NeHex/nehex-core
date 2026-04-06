from __future__ import annotations

from typing import Optional
from urllib.parse import urlparse

from pydantic import BaseModel, ConfigDict, Field, field_validator

from app.schemas.friend import FriendApplyStatus, FriendStatus


def _normalize_http_url(value: str, *, field_name: str, allow_empty: bool = False) -> Optional[str]:
    normalized = value.strip()
    if not normalized:
        if allow_empty:
            return None
        raise ValueError(f"{field_name} is required")

    parsed = urlparse(normalized)
    if not parsed.scheme:
        normalized = f"https://{normalized.lstrip('/')}"
        parsed = urlparse(normalized)

    if parsed.scheme not in {"http", "https"} or not parsed.netloc:
        raise ValueError("URL must start with http:// or https://")
    return normalized


class AdminArticleCreateRequest(BaseModel):
    model_config = ConfigDict(populate_by_name=True)

    title: str = Field(min_length=1, max_length=255)
    articleTopImage: Optional[str] = Field(default=None, max_length=500)
    class_: str = Field(alias="class", min_length=1, max_length=100)
    read: int = Field(default=0, ge=0)
    tag: Optional[str] = Field(default=None, max_length=255)
    top: int = Field(default=0, ge=0)
    content: Optional[str] = None

    @field_validator("title", "class_")
    @classmethod
    def normalize_required_fields(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field is required")
        return normalized

    @field_validator("articleTopImage", "tag", "content")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminArticleUpdateRequest(BaseModel):
    model_config = ConfigDict(populate_by_name=True)

    title: Optional[str] = Field(default=None, min_length=1, max_length=255)
    articleTopImage: Optional[str] = Field(default=None, max_length=500)
    class_: Optional[str] = Field(default=None, alias="class", min_length=1, max_length=100)
    read: Optional[int] = Field(default=None, ge=0)
    tag: Optional[str] = Field(default=None, max_length=255)
    top: Optional[int] = Field(default=None, ge=0)
    content: Optional[str] = None

    @field_validator("title", "class_")
    @classmethod
    def normalize_optional_required_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field cannot be empty")
        return normalized

    @field_validator("articleTopImage", "tag", "content")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminSimpleArticleCreateRequest(BaseModel):
    title: str = Field(min_length=1, max_length=255)
    articleTopImage: Optional[str] = Field(default=None, max_length=500)
    tag: Optional[str] = Field(default=None, max_length=255)
    top: int = Field(default=0, ge=0)
    content: Optional[str] = None

    @field_validator("title")
    @classmethod
    def normalize_title(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("title is required")
        return normalized

    @field_validator("articleTopImage", "tag", "content")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminSimpleArticleUpdateRequest(BaseModel):
    title: Optional[str] = Field(default=None, min_length=1, max_length=255)
    articleTopImage: Optional[str] = Field(default=None, max_length=500)
    tag: Optional[str] = Field(default=None, max_length=255)
    top: Optional[int] = Field(default=None, ge=0)
    content: Optional[str] = None

    @field_validator("title")
    @classmethod
    def normalize_optional_title(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        if not normalized:
            raise ValueError("title cannot be empty")
        return normalized

    @field_validator("articleTopImage", "tag", "content")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminPageCreateRequest(BaseModel):
    page_key: str = Field(min_length=1, max_length=120)
    title: str = Field(min_length=1, max_length=255)
    cover_image: Optional[str] = Field(default=None, max_length=500)
    content: Optional[str] = None
    sort: int = Field(default=0)
    status: int = Field(default=1, ge=0, le=1)

    @field_validator("page_key", "title")
    @classmethod
    def normalize_required_fields(cls, value: str) -> str:
        normalized = value.strip().strip("/")
        if not normalized:
            raise ValueError("Field is required")
        return normalized

    @field_validator("cover_image", "content")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminPageUpdateRequest(BaseModel):
    page_key: Optional[str] = Field(default=None, min_length=1, max_length=120)
    title: Optional[str] = Field(default=None, min_length=1, max_length=255)
    cover_image: Optional[str] = Field(default=None, max_length=500)
    content: Optional[str] = None
    sort: Optional[int] = None
    status: Optional[int] = Field(default=None, ge=0, le=1)

    @field_validator("page_key", "title")
    @classmethod
    def normalize_optional_required_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip().strip("/")
        if not normalized:
            raise ValueError("Field cannot be empty")
        return normalized

    @field_validator("cover_image", "content")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminProjectCreateRequest(BaseModel):
    title: str = Field(min_length=1, max_length=255)
    cover: Optional[str] = Field(default=None, max_length=500)
    category: Optional[str] = Field(default=None, max_length=120)
    description: Optional[str] = Field(default=None, max_length=1000)
    content: Optional[str] = None
    tech_stack: Optional[str] = Field(default=None, max_length=500)
    project_url: Optional[str] = Field(default=None, max_length=1000)
    github_url: Optional[str] = Field(default=None, max_length=1000)
    sort: int = Field(default=0)
    status: int = Field(default=1, ge=0, le=1)

    @field_validator("title")
    @classmethod
    def normalize_title(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("title is required")
        return normalized

    @field_validator(
        "cover",
        "category",
        "description",
        "content",
        "tech_stack",
        "project_url",
        "github_url",
    )
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminProjectUpdateRequest(BaseModel):
    title: Optional[str] = Field(default=None, min_length=1, max_length=255)
    cover: Optional[str] = Field(default=None, max_length=500)
    category: Optional[str] = Field(default=None, max_length=120)
    description: Optional[str] = Field(default=None, max_length=1000)
    content: Optional[str] = None
    tech_stack: Optional[str] = Field(default=None, max_length=500)
    project_url: Optional[str] = Field(default=None, max_length=1000)
    github_url: Optional[str] = Field(default=None, max_length=1000)
    sort: Optional[int] = None
    status: Optional[int] = Field(default=None, ge=0, le=1)

    @field_validator("title")
    @classmethod
    def normalize_optional_title(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        if not normalized:
            raise ValueError("title cannot be empty")
        return normalized

    @field_validator(
        "cover",
        "category",
        "description",
        "content",
        "tech_stack",
        "project_url",
        "github_url",
    )
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminFriendCreateRequest(BaseModel):
    title: str = Field(min_length=1, max_length=255)
    description: Optional[str] = Field(default=None, max_length=500)
    category: str = Field(default="default", min_length=1, max_length=100)
    favicon: Optional[str] = Field(default=None, max_length=500)
    url: str = Field(min_length=1, max_length=500)
    status: FriendStatus = "ok"
    overwrite_existing: bool = False

    @field_validator("title", "category")
    @classmethod
    def normalize_required_fields(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field is required")
        return normalized

    @field_validator("url")
    @classmethod
    def normalize_url(cls, value: str) -> str:
        normalized = _normalize_http_url(value, field_name="url", allow_empty=False)
        if normalized is None:
            raise ValueError("url is required")
        return normalized

    @field_validator("favicon")
    @classmethod
    def normalize_favicon(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        return _normalize_http_url(value, field_name="favicon", allow_empty=True)

    @field_validator("description")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminFriendUpdateRequest(BaseModel):
    title: Optional[str] = Field(default=None, min_length=1, max_length=255)
    description: Optional[str] = Field(default=None, max_length=500)
    category: Optional[str] = Field(default=None, min_length=1, max_length=100)
    favicon: Optional[str] = Field(default=None, max_length=500)
    url: Optional[str] = Field(default=None, min_length=1, max_length=500)
    status: Optional[FriendStatus] = None

    @field_validator("title", "category")
    @classmethod
    def normalize_optional_required_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field cannot be empty")
        return normalized

    @field_validator("url")
    @classmethod
    def normalize_optional_url(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = _normalize_http_url(value, field_name="url", allow_empty=False)
        if normalized is None:
            raise ValueError("url cannot be empty")
        return normalized

    @field_validator("favicon")
    @classmethod
    def normalize_optional_favicon(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        return _normalize_http_url(value, field_name="favicon", allow_empty=True)

    @field_validator("description")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminFriendApplyStatusUpdateRequest(BaseModel):
    status: FriendApplyStatus
    create_friend: bool = False
    friend_category: Optional[str] = Field(default=None, max_length=100)

    @field_validator("friend_category")
    @classmethod
    def normalize_friend_category(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminDailyCreateRequest(BaseModel):
    title: str = Field(min_length=1, max_length=255)
    content: Optional[str] = None
    weather: Optional[str] = Field(default=None, max_length=50)

    @field_validator("title")
    @classmethod
    def normalize_title(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("title is required")
        return normalized

    @field_validator("content", "weather")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminDailyUpdateRequest(BaseModel):
    title: Optional[str] = Field(default=None, min_length=1, max_length=255)
    content: Optional[str] = None
    weather: Optional[str] = Field(default=None, max_length=50)

    @field_validator("title")
    @classmethod
    def normalize_optional_title(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        if not normalized:
            raise ValueError("title cannot be empty")
        return normalized

    @field_validator("content", "weather")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminAlbumCreateRequest(BaseModel):
    model_config = ConfigDict(populate_by_name=True)

    title: str = Field(min_length=1, max_length=255)
    cover: Optional[str] = Field(default=None, max_length=500)
    class_: str = Field(alias="class", min_length=1, max_length=100)
    like_count: int = Field(default=0, ge=0)
    img_urls: Optional[str] = None

    @field_validator("title", "class_")
    @classmethod
    def normalize_required_fields(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field is required")
        return normalized

    @field_validator("cover", "img_urls")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminAlbumUpdateRequest(BaseModel):
    model_config = ConfigDict(populate_by_name=True)

    title: Optional[str] = Field(default=None, min_length=1, max_length=255)
    cover: Optional[str] = Field(default=None, max_length=500)
    class_: Optional[str] = Field(default=None, alias="class", min_length=1, max_length=100)
    like_count: Optional[int] = Field(default=None, ge=0)
    img_urls: Optional[str] = None

    @field_validator("title", "class_")
    @classmethod
    def normalize_optional_required_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field cannot be empty")
        return normalized

    @field_validator("cover", "img_urls")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminCommentCreateRequest(BaseModel):
    parent_id: int = Field(default=0, ge=0)
    target_type: str = Field(min_length=1, max_length=20)
    target_id: int = Field(ge=1)
    content: str = Field(min_length=1, max_length=1200)
    nickname: str = Field(min_length=1, max_length=100)
    email: Optional[str] = Field(default=None, max_length=255)
    website: Optional[str] = Field(default=None, max_length=255)
    status: int = Field(default=1, ge=0, le=1)

    @field_validator("target_type")
    @classmethod
    def normalize_target_type(cls, value: str) -> str:
        normalized = value.strip().lower()
        if not normalized:
            raise ValueError("target_type is required")
        return normalized

    @field_validator("content", "nickname")
    @classmethod
    def normalize_required_fields(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field is required")
        return normalized

    @field_validator("email", "website")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminCommentUpdateRequest(BaseModel):
    parent_id: Optional[int] = Field(default=None, ge=0)
    content: Optional[str] = Field(default=None, min_length=1, max_length=1200)
    nickname: Optional[str] = Field(default=None, min_length=1, max_length=100)
    email: Optional[str] = Field(default=None, max_length=255)
    website: Optional[str] = Field(default=None, max_length=255)
    status: Optional[int] = Field(default=None, ge=0, le=1)

    @field_validator("content", "nickname")
    @classmethod
    def normalize_optional_required_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field cannot be empty")
        return normalized

    @field_validator("email", "website")
    @classmethod
    def normalize_optional_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None
