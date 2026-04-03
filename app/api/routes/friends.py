from fastapi import APIRouter, Depends, HTTPException, Request, status
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.friend import FriendApplyRequest, FriendApplyResponse, FriendListResponse
from app.services.friends_service import create_friend_apply, list_friends

router = APIRouter(tags=["friend"])


@router.get("/friend", response_model=FriendListResponse, summary="获取全部友链")
def get_friends(session: Session = Depends(get_db_session)) -> FriendListResponse:
    data = list_friends(session)
    return FriendListResponse(data=data)


def _resolve_client_ip(request: Request) -> str | None:
    forwarded_for = request.headers.get("x-forwarded-for")
    if forwarded_for:
        first_ip = forwarded_for.split(",")[0].strip()
        return first_ip[:50] if first_ip else None

    if request.client and request.client.host:
        return request.client.host[:50]

    return None


@router.post("/friend/apply", response_model=FriendApplyResponse, summary="提交友链申请")
def post_friend_apply(
    payload: FriendApplyRequest,
    request: Request,
    session: Session = Depends(get_db_session),
) -> FriendApplyResponse:
    try:
        application_id = create_friend_apply(
            session=session,
            payload=payload,
            ip_address=_resolve_client_ip(request),
            user_agent=request.headers.get("user-agent"),
        )
    except ValueError as error:
        message = str(error)
        status_code = status.HTTP_422_UNPROCESSABLE_ENTITY
        if message == "Too many friend applications, please try later":
            status_code = status.HTTP_429_TOO_MANY_REQUESTS
        elif "already" in message:
            status_code = status.HTTP_409_CONFLICT
        raise HTTPException(status_code=status_code, detail=message) from error

    return FriendApplyResponse(
        message="Friend application submitted",
        application_id=application_id,
    )


@router.post("/friend-apply", response_model=FriendApplyResponse, summary="提交友链申请（兼容路径）")
def post_friend_apply_legacy(
    payload: FriendApplyRequest,
    request: Request,
    session: Session = Depends(get_db_session),
) -> FriendApplyResponse:
    return post_friend_apply(payload=payload, request=request, session=session)
