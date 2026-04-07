from __future__ import annotations

from datetime import datetime

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


class AdminBackupItem(BaseModel):
    filename: str
    size_bytes: int = Field(ge=0)
    created_at: datetime
    updated_at: datetime


class AdminBackupDetailResponse(BaseModel):
    data: AdminBackupItem


class AdminBackupListResponse(BaseModel):
    data: list[AdminBackupItem]


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


class AdminMailLogItem(BaseModel):
    id: int
    category: str
    template_key: str
    to_email: str
    subject: str
    body: str
    status: str
    error_message: str | None = None
    trigger_comment_id: int | None = None
    created_at: datetime
    sent_at: datetime | None = None


class AdminMailLogListResponse(BaseModel):
    data: list[AdminMailLogItem]
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


class AdminDashboardSeries(BaseModel):
    labels: list[str]
    values: list[int]
    total: int = Field(ge=0)


class AdminDashboardPeriodMetrics(BaseModel):
    day: AdminDashboardSeries
    week: AdminDashboardSeries
    month: AdminDashboardSeries
    year: AdminDashboardSeries


class AdminDashboardSiteTotals(BaseModel):
    text_count: int = Field(ge=0)
    article_count: int = Field(ge=0)
    comment_count: int = Field(ge=0)
    album_count: int = Field(ge=0)


class AdminDashboardData(BaseModel):
    visit_ip: AdminDashboardPeriodMetrics
    api_calls: AdminDashboardPeriodMetrics
    site_totals: AdminDashboardSiteTotals


class AdminDashboardResponse(BaseModel):
    data: AdminDashboardData


class AdminStorageUploadData(BaseModel):
    provider: str
    key: str
    url: str


class AdminStorageUploadResponse(BaseModel):
    data: AdminStorageUploadData
