from __future__ import annotations

from sqlalchemy import desc, func, select, update
from sqlalchemy.orm import Session

from app.models.article import Article
from app.schemas.article import ArticleItem

DEFAULT_ARTICLE_PAGE_SIZE = 20
MAX_ARTICLE_PAGE_SIZE = 100
PUBLISHED_ARTICLE_STATUS = 1


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


def _normalize_pagination(page: int, size: int) -> tuple[int, int]:
    normalized_page = max(1, int(page))
    normalized_size = max(1, min(MAX_ARTICLE_PAGE_SIZE, int(size)))
    return normalized_page, normalized_size


def list_articles(
    session: Session,
    *,
    page: int = 1,
    size: int = DEFAULT_ARTICLE_PAGE_SIZE,
    include_unpublished: bool = False,
) -> tuple[list[ArticleItem], int, int, int, int]:
    normalized_page, normalized_size = _normalize_pagination(page, size)
    offset = (normalized_page - 1) * normalized_size

    total_stmt = select(func.count(Article.id))
    if not include_unpublished:
        total_stmt = total_stmt.where(Article.status == PUBLISHED_ARTICLE_STATUS)
    total = int(session.execute(total_stmt).scalar() or 0)
    if total <= 0:
        return [], normalized_page, normalized_size, 0, 0

    stmt = select(Article)
    if not include_unpublished:
        stmt = stmt.where(Article.status == PUBLISHED_ARTICLE_STATUS)

    stmt = stmt.order_by(
        desc(Article.top),
        desc(Article.last_edit_time),
        desc(Article.id),
    ).offset(offset).limit(normalized_size)
    result = session.execute(stmt)
    rows = result.scalars().all()

    mapped = [_map_article_item(row) for row in rows]
    total_pages = (total + normalized_size - 1) // normalized_size
    return mapped, normalized_page, normalized_size, total, total_pages


def get_article_by_id(
    session: Session,
    article_id: int,
    *,
    include_unpublished: bool = False,
) -> ArticleItem | None:
    stmt = select(Article).where(Article.id == article_id)
    if not include_unpublished:
        stmt = stmt.where(Article.status == PUBLISHED_ARTICLE_STATUS)
    stmt = stmt.limit(1)
    result = session.execute(stmt)
    row = result.scalars().first()
    if row is None:
        return None
    return _map_article_item(row)


def increase_article_read_count(session: Session, article_id: int) -> ArticleItem | None:
    stmt = (
        select(Article)
        .where(
            Article.id == article_id,
            Article.status == PUBLISHED_ARTICLE_STATUS,
        )
        .limit(1)
    )
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    session.execute(
        update(Article)
        .where(
            Article.id == article_id,
            Article.status == PUBLISHED_ARTICLE_STATUS,
        )
        .values(read_count=Article.read_count + 1),
    )
    session.commit()

    refreshed = session.execute(stmt).scalars().first()
    if refreshed is None:
        return None
    return _map_article_item(refreshed)


def increase_article_like_count(session: Session, article_id: int) -> ArticleItem | None:
    stmt = (
        select(Article)
        .where(
            Article.id == article_id,
            Article.status == PUBLISHED_ARTICLE_STATUS,
        )
        .limit(1)
    )
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    session.execute(
        update(Article)
        .where(
            Article.id == article_id,
            Article.status == PUBLISHED_ARTICLE_STATUS,
        )
        .values(like_count=Article.like_count + 1),
    )
    session.commit()

    refreshed = session.execute(stmt).scalars().first()
    if refreshed is None:
        return None
    return _map_article_item(refreshed)
