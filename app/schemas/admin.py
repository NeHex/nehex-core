from __future__ import annotations

from datetime import datetime
from typing import Any
from typing import Optional

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator

from app.schemas.album import AlbumItem
from app.schemas.article import ArticleItem
from app.schemas.comment import CommentItem
from app.schemas.daily import DailyItem
from app.schemas.page import PageItem
from app.schemas.project import ProjectItem
from app.schemas.setting import SettingItem
from app.models.setting import SettingType


class AdminLoginRequest(BaseModel):
    account: str = Field(min_length=1, max_length=100)
    password: str = Field(min_length=1, max_length=300)

    @field_validator("account", "password")
    @classmethod
    def normalize_required_text(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field is required")
        return normalized


class AdminLoginData(BaseModel):
    token: Optional[str] = None
    account: str
    expires_at: datetime


class AdminLoginResponse(BaseModel):
    data: AdminLoginData


class AdminActionResponse(BaseModel):
    success: bool = True
    message: str


class AdminSettingUpdateItem(BaseModel):
    setting_key: str = Field(min_length=1, max_length=100)
    setting_content: Any = None
    setting_type: Optional[SettingType] = None
    description: Optional[str] = Field(default=None, max_length=255)

    @field_validator("setting_key")
    @classmethod
    def normalize_setting_key(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("setting_key is required")
        return normalized

    @field_validator("description")
    @classmethod
    def normalize_description(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminSettingsUpdateRequest(BaseModel):
    items: list[AdminSettingUpdateItem] = Field(min_length=1, max_length=200)


class AdminAccountSettingsUpdateRequest(BaseModel):
    account: Optional[str] = Field(default=None, min_length=1, max_length=100)
    new_password: Optional[str] = Field(default=None, min_length=1, max_length=300)
    confirm_password: Optional[str] = Field(default=None, min_length=1, max_length=300)

    @field_validator("account", "new_password", "confirm_password")
    @classmethod
    def normalize_optional_text_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None

    @model_validator(mode="after")
    def validate_password_update(self) -> "AdminAccountSettingsUpdateRequest":
        if self.new_password is None:
            return self

        if self.confirm_password is None:
            raise ValueError("confirm_password is required when new_password is provided")

        if self.new_password != self.confirm_password:
            raise ValueError("new_password and confirm_password do not match")
        return self


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


class AdminArticleDetailResponse(BaseModel):
    data: ArticleItem


class AdminArticleListResponse(BaseModel):
    data: list[ArticleItem]


class AdminDailyDetailResponse(BaseModel):
    data: DailyItem


class AdminAlbumDetailResponse(BaseModel):
    data: AlbumItem


class AdminCommentDetailResponse(BaseModel):
    data: CommentItem


class AdminCommentListResponse(BaseModel):
    data: list[CommentItem]


class AdminPageDetailResponse(BaseModel):
    data: PageItem


class AdminPageListResponse(BaseModel):
    data: list[PageItem]


class AdminProjectDetailResponse(BaseModel):
    data: ProjectItem


class AdminProjectListResponse(BaseModel):
    data: list[ProjectItem]


class AdminSettingListResponse(BaseModel):
    data: list[SettingItem]
