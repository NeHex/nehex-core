from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.article import ArticleDetailResponse, ArticleListResponse, ArticlePagination
from app.services.articles_service import get_article_by_id, list_articles

router = APIRouter(tags=["article"])


@router.get("/article", response_model=ArticleListResponse, summary="获取文章列表")
def get_articles(
    page: int = Query(default=1, ge=1),
    size: int = Query(default=20, ge=1, le=100),
    session: Session = Depends(get_db_session),
) -> ArticleListResponse:
    data, normalized_page, normalized_size, total, total_pages = list_articles(
        session,
        page=page,
        size=size,
    )
    return ArticleListResponse(
        data=data,
        pagination=ArticlePagination(
            page=normalized_page,
            size=normalized_size,
            total=total,
            total_pages=total_pages,
        ),
    )


@router.get("/article/{article_id}", response_model=ArticleDetailResponse, summary="获取文章详情")
def get_article_detail(
    article_id: int,
    session: Session = Depends(get_db_session),
) -> ArticleDetailResponse:
    article = get_article_by_id(session, article_id)
    if article is None:
        raise HTTPException(status_code=404, detail="Article not found")
    return ArticleDetailResponse(data=article)
