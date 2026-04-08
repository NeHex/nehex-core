from fastapi import APIRouter, Depends, HTTPException, Query, Request, Response
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.article import ArticleDetailResponse, ArticleListResponse, ArticlePagination
from app.services.articles_service import (
    get_article_by_id,
    increase_article_like_count,
    increase_article_read_count,
    list_articles,
)

router = APIRouter(tags=["article"])
LIKE_COOKIE_KEY = "article_liked_ids"
LIKE_COOKIE_MAX_ITEMS = 400
LIKE_COOKIE_MAX_AGE = 60 * 60 * 24 * 365


def _parse_liked_cookie(raw_cookie: str | None) -> list[int]:
    if not raw_cookie:
        return []

    items: list[int] = []
    for chunk in raw_cookie.split(","):
        value = chunk.strip()
        if not value or not value.isdigit():
            continue
        parsed = int(value)
        if parsed <= 0:
            continue
        items.append(parsed)

    seen: set[int] = set()
    result: list[int] = []
    for item in items:
        if item in seen:
            continue
        seen.add(item)
        result.append(item)
    return result


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


@router.post("/article/{article_id}/read", response_model=ArticleDetailResponse, summary="增加文章阅读数")
def post_article_read(
    article_id: int,
    session: Session = Depends(get_db_session),
) -> ArticleDetailResponse:
    if article_id <= 0:
        raise HTTPException(status_code=422, detail="Invalid article id")

    article = increase_article_read_count(session=session, article_id=article_id)
    if article is None:
        raise HTTPException(status_code=404, detail="Article not found")
    return ArticleDetailResponse(data=article)


@router.post("/article/{article_id}/like", response_model=ArticleDetailResponse, summary="文章点赞")
def post_article_like(
    article_id: int,
    request: Request,
    response: Response,
    session: Session = Depends(get_db_session),
) -> ArticleDetailResponse:
    if article_id <= 0:
        raise HTTPException(status_code=422, detail="Invalid article id")

    liked_ids = _parse_liked_cookie(request.cookies.get(LIKE_COOKIE_KEY))
    if article_id in liked_ids:
        raise HTTPException(status_code=409, detail="Already liked")

    article = increase_article_like_count(session=session, article_id=article_id)
    if article is None:
        raise HTTPException(status_code=404, detail="Article not found")

    liked_ids.append(article_id)
    liked_ids = liked_ids[-LIKE_COOKIE_MAX_ITEMS:]
    response.set_cookie(
        key=LIKE_COOKIE_KEY,
        value=",".join(str(item) for item in liked_ids),
        max_age=LIKE_COOKIE_MAX_AGE,
        samesite="lax",
        httponly=False,
    )
    return ArticleDetailResponse(data=article)
