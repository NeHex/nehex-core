from __future__ import annotations

from typing import Optional

from sqlalchemy import delete, desc, select
from sqlalchemy.orm import Session

from app.core.simple_cache import cache
from app.models.album import Album
from app.models.article import Article
from app.models.comment import Comment
from app.models.daily import Daily
from app.models.project import Project
from app.models.setting import Setting
from app.models.singlepage import SinglePage
from app.schemas.album import AlbumItem
from app.schemas.article import ArticleItem
from app.schemas.comment import CommentItem
from app.schemas.daily import DailyItem
from app.schemas.page import PageItem
from app.schemas.project import ProjectItem


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


def _invalidate_comment_cache(target_type: str, target_id: int) -> None:
    cache.delete_prefix(f"comments:list:{target_type}:{target_id}:")


def _normalize_optional_text(value: Optional[str]) -> Optional[str]:
    if value is None:
        return None
    normalized = value.strip()
    return normalized or None


def _map_article_item(row: Article) -> ArticleItem:
    return ArticleItem(
        id=row.id,
        title=row.title,
        articleTopImage=row.article_top_image,
        class_=row.article_class,
        read=row.read_count,
        lastEditTime=row.last_edit_time,
        tag=row.tag,
        top=row.top,
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
        ip=row.ip,
        create_time=row.create_time,
        update_time=row.update_time,
        replies=[],
    )


def get_admin_credentials(session: Session) -> tuple[str, str]:
    stmt = select(Setting).where(
        Setting.setting_key.in_(["user_account", "user_account_password"]),
    )
    rows = session.execute(stmt).scalars().all()
    setting_map = {row.setting_key: row.setting_content or "" for row in rows}
    account = str(setting_map.get("user_account", "")).strip()
    password_hash = str(setting_map.get("user_account_password", "")).strip().lower()
    return account, password_hash


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


def create_admin_comment(
    session: Session,
    *,
    parent_id: int,
    target_type: str,
    target_id: int,
    content: str,
    nickname: str,
    email: Optional[str] = None,
    website: Optional[str] = None,
    status: int = 1,
) -> CommentItem:
    row = Comment(
        parent_id=max(0, int(parent_id)),
        target_type=target_type.strip().lower(),
        target_id=int(target_id),
        content=content.strip(),
        nickname=nickname.strip(),
        email=_normalize_optional_text(email),
        website=_normalize_optional_text(website),
        status=1 if int(status) > 0 else 0,
    )
    session.add(row)
    session.commit()
    session.refresh(row)
    _invalidate_comment_cache(row.target_type, row.target_id)
    return _map_comment_item(row)


def update_admin_comment(
    session: Session,
    comment_id: int,
    *,
    parent_id: Optional[int] = None,
    content: Optional[str] = None,
    nickname: Optional[str] = None,
    email: Optional[str] = None,
    website: Optional[str] = None,
    status: Optional[int] = None,
) -> Optional[CommentItem]:
    stmt = select(Comment).where(Comment.id == comment_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    if parent_id is not None:
        row.parent_id = max(0, int(parent_id))
    if content is not None:
        row.content = content.strip()
    if nickname is not None:
        row.nickname = nickname.strip()
    if email is not None:
        row.email = _normalize_optional_text(email)
    if website is not None:
        row.website = _normalize_optional_text(website)
    if status is not None:
        row.status = 1 if int(status) > 0 else 0

    session.commit()
    session.refresh(row)
    _invalidate_comment_cache(row.target_type, row.target_id)
    return _map_comment_item(row)


def _collect_comment_descendant_ids(session: Session, root_id: int) -> list[int]:
    all_ids: list[int] = [root_id]
    frontier: list[int] = [root_id]

    while frontier:
        stmt = select(Comment.id).where(Comment.parent_id.in_(frontier))
        children = list(session.execute(stmt).scalars().all())
        if not children:
            break
        all_ids.extend(children)
        frontier = children

    return all_ids


def delete_admin_comment(session: Session, comment_id: int) -> bool:
    stmt = select(Comment).where(Comment.id == comment_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return False

    delete_ids = _collect_comment_descendant_ids(session, comment_id)
    if delete_ids:
        session.execute(delete(Comment).where(Comment.id.in_(delete_ids)))
    session.commit()
    _invalidate_comment_cache(row.target_type, row.target_id)
    return True
