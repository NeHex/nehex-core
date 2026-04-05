from __future__ import annotations

from typing import Optional

from sqlalchemy import desc, select
from sqlalchemy.orm import Session

from app.models.album import Album
from app.models.article import Article
from app.models.daily import Daily
from app.models.project import Project
from app.models.singlepage import SinglePage
from app.schemas.album import AlbumItem
from app.schemas.article import ArticleItem
from app.schemas.daily import DailyItem
from app.schemas.page import PageItem
from app.schemas.project import ProjectItem
from app.services.admin_service_parts.common import (
    _invalidate_album_cache,
    _invalidate_article_cache,
    _invalidate_daily_cache,
    _invalidate_page_cache,
    _invalidate_project_cache,
    _map_album_item,
    _map_article_item,
    _map_daily_item,
    _map_page_item,
    _map_project_item,
    _normalize_optional_text,
)


def list_articles_by_class(session: Session, article_class: str) -> list[ArticleItem]:
    stmt = (
        select(Article)
        .where(Article.article_class == article_class)
        .order_by(desc(Article.top), desc(Article.last_edit_time), desc(Article.id))
    )
    rows = session.execute(stmt).scalars().all()
    return [_map_article_item(row) for row in rows]


def list_pages(session: Session) -> list[PageItem]:
    stmt = (
        select(SinglePage)
        .order_by(SinglePage.sort.asc(), desc(SinglePage.update_time), desc(SinglePage.id))
    )
    rows = session.execute(stmt).scalars().all()
    return [_map_page_item(row) for row in rows]


def get_page_by_id(session: Session, page_id: int) -> Optional[PageItem]:
    stmt = select(SinglePage).where(SinglePage.id == page_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None
    return _map_page_item(row)


def create_page(
    session: Session,
    *,
    page_key: str,
    title: str,
    cover_image: Optional[str] = None,
    content: Optional[str] = None,
    sort: int = 0,
    status: int = 1,
) -> PageItem:
    row = SinglePage(
        page_key=page_key.strip().strip("/"),
        title=title.strip(),
        cover_image=_normalize_optional_text(cover_image),
        content=_normalize_optional_text(content),
        sort=int(sort),
        status=1 if int(status) > 0 else 0,
    )
    session.add(row)
    session.commit()
    session.refresh(row)
    _invalidate_page_cache()
    return _map_page_item(row)


def update_page(
    session: Session,
    page_id: int,
    *,
    page_key: Optional[str] = None,
    title: Optional[str] = None,
    cover_image: Optional[str] = None,
    content: Optional[str] = None,
    sort: Optional[int] = None,
    status: Optional[int] = None,
) -> Optional[PageItem]:
    stmt = select(SinglePage).where(SinglePage.id == page_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    if page_key is not None:
        row.page_key = page_key.strip().strip("/")
    if title is not None:
        row.title = title.strip()
    if cover_image is not None:
        row.cover_image = _normalize_optional_text(cover_image)
    if content is not None:
        row.content = _normalize_optional_text(content)
    if sort is not None:
        row.sort = int(sort)
    if status is not None:
        row.status = 1 if int(status) > 0 else 0

    session.commit()
    session.refresh(row)
    _invalidate_page_cache()
    return _map_page_item(row)


def delete_page(session: Session, page_id: int) -> bool:
    stmt = select(SinglePage).where(SinglePage.id == page_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return False

    session.delete(row)
    session.commit()
    _invalidate_page_cache()
    return True


def list_projects(session: Session) -> list[ProjectItem]:
    stmt = (
        select(Project)
        .order_by(Project.sort.asc(), desc(Project.update_time), desc(Project.id))
    )
    rows = session.execute(stmt).scalars().all()
    return [_map_project_item(row) for row in rows]


def create_project(
    session: Session,
    *,
    title: str,
    cover: Optional[str] = None,
    category: Optional[str] = None,
    description: Optional[str] = None,
    content: Optional[str] = None,
    tech_stack: Optional[str] = None,
    project_url: Optional[str] = None,
    github_url: Optional[str] = None,
    sort: int = 0,
    status: int = 1,
) -> ProjectItem:
    row = Project(
        title=title.strip(),
        cover=_normalize_optional_text(cover),
        category=_normalize_optional_text(category),
        description=_normalize_optional_text(description),
        content=_normalize_optional_text(content),
        tech_stack=_normalize_optional_text(tech_stack),
        project_url=_normalize_optional_text(project_url),
        github_url=_normalize_optional_text(github_url),
        sort=int(sort),
        status=1 if int(status) > 0 else 0,
    )
    session.add(row)
    session.commit()
    session.refresh(row)
    _invalidate_project_cache()
    return _map_project_item(row)


def update_project(
    session: Session,
    project_id: int,
    *,
    title: Optional[str] = None,
    cover: Optional[str] = None,
    category: Optional[str] = None,
    description: Optional[str] = None,
    content: Optional[str] = None,
    tech_stack: Optional[str] = None,
    project_url: Optional[str] = None,
    github_url: Optional[str] = None,
    sort: Optional[int] = None,
    status: Optional[int] = None,
) -> Optional[ProjectItem]:
    stmt = select(Project).where(Project.id == project_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    if title is not None:
        row.title = title.strip()
    if cover is not None:
        row.cover = _normalize_optional_text(cover)
    if category is not None:
        row.category = _normalize_optional_text(category)
    if description is not None:
        row.description = _normalize_optional_text(description)
    if content is not None:
        row.content = _normalize_optional_text(content)
    if tech_stack is not None:
        row.tech_stack = _normalize_optional_text(tech_stack)
    if project_url is not None:
        row.project_url = _normalize_optional_text(project_url)
    if github_url is not None:
        row.github_url = _normalize_optional_text(github_url)
    if sort is not None:
        row.sort = int(sort)
    if status is not None:
        row.status = 1 if int(status) > 0 else 0

    session.commit()
    session.refresh(row)
    _invalidate_project_cache()
    return _map_project_item(row)


def delete_project(session: Session, project_id: int) -> bool:
    stmt = select(Project).where(Project.id == project_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return False

    session.delete(row)
    session.commit()
    _invalidate_project_cache()
    return True


def create_article(
    session: Session,
    *,
    title: str,
    article_class: str,
    article_top_image: Optional[str] = None,
    read_count: int = 0,
    tag: Optional[str] = None,
    top: int = 0,
    content: Optional[str] = None,
) -> ArticleItem:
    row = Article(
        title=title.strip(),
        article_class=article_class.strip(),
        article_top_image=_normalize_optional_text(article_top_image),
        read_count=max(0, int(read_count)),
        tag=_normalize_optional_text(tag),
        top=max(0, int(top)),
        content=_normalize_optional_text(content),
    )
    session.add(row)
    session.commit()
    session.refresh(row)
    _invalidate_article_cache()
    return _map_article_item(row)


def update_article(
    session: Session,
    article_id: int,
    *,
    title: Optional[str] = None,
    article_class: Optional[str] = None,
    article_top_image: Optional[str] = None,
    read_count: Optional[int] = None,
    tag: Optional[str] = None,
    top: Optional[int] = None,
    content: Optional[str] = None,
    expected_class: Optional[str] = None,
) -> Optional[ArticleItem]:
    stmt = select(Article).where(Article.id == article_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    if expected_class is not None and row.article_class != expected_class:
        return None

    if title is not None:
        row.title = title.strip()
    if article_class is not None:
        row.article_class = article_class.strip()
    if article_top_image is not None:
        row.article_top_image = _normalize_optional_text(article_top_image)
    if read_count is not None:
        row.read_count = max(0, int(read_count))
    if tag is not None:
        row.tag = _normalize_optional_text(tag)
    if top is not None:
        row.top = max(0, int(top))
    if content is not None:
        row.content = _normalize_optional_text(content)

    session.commit()
    session.refresh(row)
    _invalidate_article_cache()
    return _map_article_item(row)


def delete_article(
    session: Session,
    article_id: int,
    *,
    expected_class: Optional[str] = None,
) -> bool:
    stmt = select(Article).where(Article.id == article_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return False

    if expected_class is not None and row.article_class != expected_class:
        return False

    session.delete(row)
    session.commit()
    _invalidate_article_cache()
    return True


def create_daily(
    session: Session,
    *,
    title: str,
    content: Optional[str] = None,
    weather: Optional[str] = None,
) -> DailyItem:
    row = Daily(
        title=title.strip(),
        content=_normalize_optional_text(content),
        weather=_normalize_optional_text(weather),
    )
    session.add(row)
    session.commit()
    session.refresh(row)
    _invalidate_daily_cache()
    return _map_daily_item(row)


def update_daily(
    session: Session,
    daily_id: int,
    *,
    title: Optional[str] = None,
    content: Optional[str] = None,
    weather: Optional[str] = None,
) -> Optional[DailyItem]:
    stmt = select(Daily).where(Daily.id == daily_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    if title is not None:
        row.title = title.strip()
    if content is not None:
        row.content = _normalize_optional_text(content)
    if weather is not None:
        row.weather = _normalize_optional_text(weather)

    session.commit()
    session.refresh(row)
    _invalidate_daily_cache()
    return _map_daily_item(row)


def delete_daily(session: Session, daily_id: int) -> bool:
    stmt = select(Daily).where(Daily.id == daily_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return False

    session.delete(row)
    session.commit()
    _invalidate_daily_cache()
    return True


def create_album(
    session: Session,
    *,
    title: str,
    album_class: str,
    cover: Optional[str] = None,
    like_count: int = 0,
    img_urls: Optional[str] = None,
) -> AlbumItem:
    row = Album(
        title=title.strip(),
        album_class=album_class.strip(),
        cover=_normalize_optional_text(cover),
        like_count=max(0, int(like_count)),
        img_urls=_normalize_optional_text(img_urls),
    )
    session.add(row)
    session.commit()
    session.refresh(row)
    _invalidate_album_cache()
    return _map_album_item(row)


def update_album(
    session: Session,
    album_id: int,
    *,
    title: Optional[str] = None,
    album_class: Optional[str] = None,
    cover: Optional[str] = None,
    like_count: Optional[int] = None,
    img_urls: Optional[str] = None,
) -> Optional[AlbumItem]:
    stmt = select(Album).where(Album.id == album_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    if title is not None:
        row.title = title.strip()
    if album_class is not None:
        row.album_class = album_class.strip()
    if cover is not None:
        row.cover = _normalize_optional_text(cover)
    if like_count is not None:
        row.like_count = max(0, int(like_count))
    if img_urls is not None:
        row.img_urls = _normalize_optional_text(img_urls)

    session.commit()
    session.refresh(row)
    _invalidate_album_cache()
    return _map_album_item(row)


def delete_album(session: Session, album_id: int) -> bool:
    stmt = select(Album).where(Album.id == album_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return False

    session.delete(row)
    session.commit()
    _invalidate_album_cache()
    return True
