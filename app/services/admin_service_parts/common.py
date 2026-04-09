from __future__ import annotations

from typing import Optional

from app.core.simple_cache import cache
from app.models.album import Album
from app.models.article import Article
from app.models.comment import Comment
from app.models.daily import Daily
from app.models.friend import Friend
from app.models.friend_apply import FriendApply
from app.models.project import Project
from app.models.setting import Setting
from app.models.singlepage import SinglePage
from app.schemas.album import AlbumItem
from app.schemas.article import ArticleItem
from app.schemas.comment import CommentItem
from app.schemas.daily import DailyItem
from app.schemas.friend import FriendApplyItem, FriendItem
from app.schemas.page import PageItem
from app.schemas.project import ProjectItem
from app.schemas.setting import SettingItem
from app.services.settings_service import parse_setting_content

SENSITIVE_ADMIN_SETTING_KEYS = {"user_account", "user_account_password"}
FRIEND_STATUS_VALUES = {"ok", "missing", "blocked"}
FRIEND_APPLY_STATUS_VALUES = {"pending", "approved", "rejected", "blocked"}


def _invalidate_article_cache() -> None:
    cache.delete("articles:list")


def _invalidate_daily_cache() -> None:
    cache.delete("dailies:list")


def _invalidate_album_cache() -> None:
    cache.delete("albums:list")


def _invalidate_page_cache() -> None:
    cache.delete("pages:list")


def _invalidate_project_cache() -> None:
    cache.delete("projects:list")


def _invalidate_friends_cache() -> None:
    cache.delete("friends:list")


def _invalidate_comment_cache(target_type: str, target_id: int) -> None:
    cache.delete_prefix(f"comments:list:{target_type}:{target_id}:")


def _invalidate_settings_cache() -> None:
    cache.delete("settings:list")
    cache.delete("settings:list:with-theme-details")


def _normalize_optional_text(value: Optional[str]) -> Optional[str]:
    if value is None:
        return None
    normalized = value.strip()
    return normalized or None


def _map_article_item(row: Article) -> ArticleItem:
    status = row.status if row.status is not None else 1
    return ArticleItem(
        id=row.id,
        title=row.title,
        articleTopImage=row.article_top_image,
        class_=row.article_class,
        read=row.read_count,
        like_count=row.like_count,
        lastEditTime=row.last_edit_time,
        tag=row.tag,
        top=row.top,
        status=1 if int(status) > 0 else 0,
        content=row.content,
    )


def _map_daily_item(row: Daily) -> DailyItem:
    return DailyItem(
        id=row.id,
        title=row.title,
        content=row.content,
        create_time=row.create_time,
        weather=row.weather,
    )


def _map_album_item(row: Album) -> AlbumItem:
    return AlbumItem(
        id=row.id,
        title=row.title,
        cover=row.cover,
        class_=row.album_class,
        like_count=row.like_count,
        img_urls=row.img_urls,
        create_time=row.create_time,
        update_time=row.update_time,
    )


def _map_page_item(row: SinglePage) -> PageItem:
    return PageItem(
        id=row.id,
        page_key=row.page_key,
        title=row.title,
        cover_image=row.cover_image,
        content=row.content,
        sort=row.sort,
        status=row.status,
        create_time=row.create_time,
        update_time=row.update_time,
    )


def _map_project_item(row: Project) -> ProjectItem:
    return ProjectItem(
        id=row.id,
        title=row.title,
        cover=row.cover,
        category=row.category,
        description=row.description,
        content=row.content,
        tech_stack=row.tech_stack,
        project_url=row.project_url,
        github_url=row.github_url,
        sort=row.sort,
        status=row.status,
        create_time=row.create_time,
        update_time=row.update_time,
    )


def _normalize_friend_status(value: object) -> str:
    if isinstance(value, int):
        return {
            1: "ok",
            2: "missing",
            3: "blocked",
            0: "missing",
            -1: "blocked",
        }.get(value, "ok")

    normalized = str(value or "").strip().lower()
    if normalized in FRIEND_STATUS_VALUES:
        return normalized

    if normalized.isdigit():
        return _normalize_friend_status(int(normalized))

    return "ok"


def _normalize_friend_apply_status(value: object) -> str:
    normalized = str(value or "").strip().lower()
    if normalized in FRIEND_APPLY_STATUS_VALUES:
        return normalized
    return "pending"


def _map_friend_item(row: Friend) -> FriendItem:
    return FriendItem(
        id=row.id,
        title=row.title,
        description=row.description,
        category=row.category,
        favicon=row.favicon,
        url=row.url,
        status=_normalize_friend_status(row.status),
        create_time=row.create_time,
    )


def _map_friend_apply_item(row: FriendApply) -> FriendApplyItem:
    return FriendApplyItem(
        id=row.id,
        site_title=row.site_title,
        site_url=row.site_url,
        site_description=row.site_description,
        site_icon=row.site_icon,
        contact=row.contact,
        status=_normalize_friend_apply_status(row.status),
        ip=row.ip,
        user_agent=row.user_agent,
        create_time=row.create_time,
        update_time=row.update_time,
    )


def _map_comment_item(row: Comment) -> CommentItem:
    return CommentItem(
        id=row.id,
        parent_id=row.parent_id,
        target_type=row.target_type,
        target_id=row.target_id,
        content=row.content,
        nickname=row.nickname,
        email=row.email,
        website=row.website,
        like_count=row.like_count,
        status=row.status,
        is_admin=bool(row.is_admin),
        ip=row.ip,
        create_time=row.create_time,
        update_time=row.update_time,
        replies=[],
    )


def _map_setting_item(row: Setting) -> SettingItem:
    return SettingItem(
        setting_key=row.setting_key,
        setting_type=row.setting_type,
        setting_content=parse_setting_content(row.setting_type, row.setting_content),
        description=row.description,
        updated_at=row.updated_at,
        created_at=row.created_at,
    )
