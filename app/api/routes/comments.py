from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException, Query, Request, Response
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.comment import CommentCreateRequest, CommentDetailResponse, CommentListResponse
from app.services.comments_service import (
    SUPPORTED_TARGET_TYPES,
    create_comment,
    like_comment,
    list_comments,
)

router = APIRouter(tags=["comment"])
LIKE_COOKIE_KEY = "comment_liked_ids"
LIKE_COOKIE_MAX_ITEMS = 400
LIKE_COOKIE_MAX_AGE = 60 * 60 * 24 * 365


def _resolve_client_ip(request: Request) -> str | None:
    forwarded_for = request.headers.get("x-forwarded-for")
    if forwarded_for:
        first_ip = forwarded_for.split(",")[0].strip()
        return first_ip[:50] if first_ip else None

    if request.client and request.client.host:
        return request.client.host[:50]

    return None


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

    # preserve order and remove duplicates
    seen: set[int] = set()
    result: list[int] = []
    for item in items:
        if item in seen:
            continue
        seen.add(item)
        result.append(item)
    return result


@router.get("/comment", response_model=CommentListResponse, summary="List comments by target")
def get_comments(
    target_type: str = Query(..., min_length=1, max_length=20),
    target_id: int = Query(..., ge=1),
    status: int = Query(1, ge=0, le=1),
    session: Session = Depends(get_db_session),
) -> CommentListResponse:
    normalized_target_type = target_type.strip().lower()
    if normalized_target_type not in SUPPORTED_TARGET_TYPES:
        raise HTTPException(status_code=422, detail="Unsupported target_type")

    data = list_comments(
        session=session,
        target_type=normalized_target_type,
        target_id=target_id,
        status=status,
    )
    return CommentListResponse(data=data)


@router.post("/comment", response_model=CommentDetailResponse, summary="Create comment")
def post_comment(
    payload: CommentCreateRequest,
    request: Request,
    session: Session = Depends(get_db_session),
) -> CommentDetailResponse:
    if payload.target_type not in SUPPORTED_TARGET_TYPES:
        raise HTTPException(status_code=422, detail="Unsupported target_type")

    try:
        data = create_comment(
            session=session,
            payload=payload,
            ip_address=_resolve_client_ip(request),
        )
    except ValueError as error:
        message = str(error)
        status_code = 422
        if message == "Duplicate comment detected":
            status_code = 409
        elif message == "Too many comment submissions, please try later":
            status_code = 429
        elif message in {"Comment contains blocked content", "Comment looks like spam"}:
            status_code = 400
        raise HTTPException(status_code=status_code, detail=message) from error

    return CommentDetailResponse(data=data)


@router.post("/comment/{comment_id}/like", response_model=CommentDetailResponse, summary="Like comment")
def post_comment_like(
    comment_id: int,
    request: Request,
    response: Response,
    session: Session = Depends(get_db_session),
) -> CommentDetailResponse:
    if comment_id <= 0:
        raise HTTPException(status_code=422, detail="Invalid comment id")

    liked_ids = _parse_liked_cookie(request.cookies.get(LIKE_COOKIE_KEY))
    if comment_id in liked_ids:
        raise HTTPException(status_code=409, detail="Already liked")

    liked_comment = like_comment(session=session, comment_id=comment_id)
    if liked_comment is None:
        raise HTTPException(status_code=404, detail="Comment not found")

    liked_ids.append(comment_id)
    liked_ids = liked_ids[-LIKE_COOKIE_MAX_ITEMS:]
    response.set_cookie(
        key=LIKE_COOKIE_KEY,
        value=",".join(str(item) for item in liked_ids),
        max_age=LIKE_COOKIE_MAX_AGE,
        samesite="lax",
        httponly=False,
    )

    return CommentDetailResponse(data=liked_comment)
