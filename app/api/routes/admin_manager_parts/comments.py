from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy.orm import Session

from app.core.admin_security import AdminPrincipal, require_admin_principal
from app.core.database import get_db_session
from app.schemas.admin import (
    AdminActionResponse,
    AdminCommentCreateRequest,
    AdminCommentDetailResponse,
    AdminCommentListResponse,
    AdminCommentUpdateRequest,
    AdminPagination,
)
from app.services.admin_service import (
    create_admin_comment,
    delete_admin_comment,
    list_admin_comments,
    update_admin_comment,
)

router = APIRouter()


@router.post("/comments", response_model=AdminCommentDetailResponse, summary="Create comment")
def admin_create_comment(
    payload: AdminCommentCreateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminCommentDetailResponse:
    item = create_admin_comment(
        session=session,
        parent_id=payload.parent_id,
        target_type=payload.target_type,
        target_id=payload.target_id,
        content=payload.content,
        nickname=payload.nickname,
        email=payload.email,
        website=payload.website,
        status=payload.status,
    )
    return AdminCommentDetailResponse(data=item)


@router.get("/comments", response_model=AdminCommentListResponse, summary="List comments")
def admin_list_comments_api(
    keyword: str = Query(default="", max_length=200),
    page: int = Query(default=1, ge=1),
    size: int = Query(default=20, ge=1, le=100),
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminCommentListResponse:
    data, normalized_page, normalized_size, total, total_pages = list_admin_comments(
        session=session,
        keyword=keyword,
        page=page,
        size=size,
    )
    return AdminCommentListResponse(
        data=data,
        pagination=AdminPagination(
            page=normalized_page,
            size=normalized_size,
            total=total,
            total_pages=total_pages,
        ),
    )


@router.put("/comments/{comment_id}", response_model=AdminCommentDetailResponse, summary="Update comment")
def admin_update_comment(
    comment_id: int,
    payload: AdminCommentUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminCommentDetailResponse:
    data = payload.model_dump(exclude_unset=True)
    item = update_admin_comment(
        session=session,
        comment_id=comment_id,
        parent_id=data.get("parent_id"),
        content=data.get("content"),
        nickname=data.get("nickname"),
        email=data.get("email"),
        website=data.get("website"),
        status=data.get("status"),
    )
    if item is None:
        raise HTTPException(status_code=404, detail="Comment not found")
    return AdminCommentDetailResponse(data=item)


@router.delete("/comments/{comment_id}", response_model=AdminActionResponse, summary="Delete comment")
def admin_delete_comment(
    comment_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    deleted = delete_admin_comment(session=session, comment_id=comment_id)
    if not deleted:
        raise HTTPException(status_code=404, detail="Comment not found")
    return AdminActionResponse(message="Comment deleted")
