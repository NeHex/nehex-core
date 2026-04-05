from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.page import PageDetailResponse, PageListResponse
from app.services.pages_service import get_page_by_key, list_pages

router = APIRouter(tags=["page"])


@router.get("/page", response_model=PageListResponse, summary="获取全部独立页")
def get_pages(session: Session = Depends(get_db_session)) -> PageListResponse:
    data = list_pages(session)
    return PageListResponse(data=data)


@router.get("/page/{page_key}", response_model=PageDetailResponse, summary="获取独立页详情")
def get_page_detail(
    page_key: str,
    session: Session = Depends(get_db_session),
) -> PageDetailResponse:
    page = get_page_by_key(session, page_key)
    if page is None:
        raise HTTPException(status_code=404, detail="Page not found")
    return PageDetailResponse(data=page)
