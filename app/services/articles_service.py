from __future__ import annotations

from sqlalchemy import desc, select
from sqlalchemy.orm import Session

from app.models.article import Article
from app.schemas.article import ArticleItem


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
    stmt = select(Article).order_by(
        desc(Article.top),
        desc(Article.last_edit_time),
        desc(Article.id),
    )
    result = session.execute(stmt)
    rows = result.scalars().all()

    return [_map_article_item(row) for row in rows]


def get_article_by_id(session: Session, article_id: int) -> ArticleItem | None:
    stmt = select(Article).where(Article.id == article_id).limit(1)
    result = session.execute(stmt)
    row = result.scalars().first()
    if row is None:
        return None
    return _map_article_item(row)
