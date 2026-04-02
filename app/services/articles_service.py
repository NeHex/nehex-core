from __future__ import annotations

from sqlalchemy import desc, select
from sqlalchemy.orm import Session

from app.core.simple_cache import cache
from app.models.article import Article
from app.schemas.article import ArticleItem

ARTICLES_CACHE_KEY = "articles:list"
ARTICLES_CACHE_TTL_SECONDS = 20


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


def list_articles(session: Session) -> list[ArticleItem]:
    cached = cache.get(ARTICLES_CACHE_KEY)
    if cached is not None:
        return [item.model_copy(deep=True) for item in cached]

    stmt = select(Article).order_by(
        desc(Article.top),
        desc(Article.last_edit_time),
        desc(Article.id),
    )
    result = session.execute(stmt)
    rows = result.scalars().all()

    mapped = [_map_article_item(row) for row in rows]
    cache.set(ARTICLES_CACHE_KEY, mapped, ARTICLES_CACHE_TTL_SECONDS)
    return [item.model_copy(deep=True) for item in mapped]


def get_article_by_id(session: Session, article_id: int) -> ArticleItem | None:
    stmt = select(Article).where(Article.id == article_id).limit(1)
    result = session.execute(stmt)
    row = result.scalars().first()
    if row is None:
        return None
    return _map_article_item(row)
