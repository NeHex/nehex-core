from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.article import ArticleDetailResponse, ArticleListResponse
from app.services.articles_service import get_article_by_id, list_articles

router = APIRouter(tags=["article"])


@router.get("/article", response_model=ArticleListResponse, summary="获取全部文章")
def get_articles(session: Session = Depends(get_db_session)) -> ArticleListResponse:
    data = list_articles(session)
    return ArticleListResponse(data=data)


@router.get("/article/{article_id}", response_model=ArticleDetailResponse, summary="获取文章详情")
def get_article_detail(
    article_id: int,
    session: Session = Depends(get_db_session),
) -> ArticleDetailResponse:
    article = get_article_by_id(session, article_id)
    if article is None:
        raise HTTPException(status_code=404, detail="Article not found")
    return ArticleDetailResponse(data=article)
