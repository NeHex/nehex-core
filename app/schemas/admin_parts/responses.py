from __future__ import annotations

from pydantic import BaseModel, Field

from app.schemas.album import AlbumItem
from app.schemas.article import ArticleItem
from app.schemas.comment import CommentItem
from app.schemas.daily import DailyItem
from app.schemas.friend import FriendApplyItem, FriendItem
from app.schemas.page import PageItem
from app.schemas.project import ProjectItem
from app.schemas.setting import SettingItem


class AdminPagination(BaseModel):
    page: int = Field(ge=1)
    size: int = Field(ge=1)
    total: int = Field(ge=0)
    total_pages: int = Field(ge=0)


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
    pagination: AdminPagination


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


class AdminFriendDetailResponse(BaseModel):
    data: FriendItem


class AdminFriendListResponse(BaseModel):
    data: list[FriendItem]


class AdminFriendApplyDetailResponse(BaseModel):
    data: FriendApplyItem


class AdminFriendApplyListResponse(BaseModel):
    data: list[FriendApplyItem]
