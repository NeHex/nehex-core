from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy.exc import IntegrityError
from sqlalchemy.orm import Session

from app.core.admin_security import AdminPrincipal, require_admin_principal
from app.core.database import get_db_session
from app.schemas.admin import (
    AdminActionResponse,
    AdminFriendApplyDetailResponse,
    AdminFriendApplyListResponse,
    AdminFriendApplyStatusUpdateRequest,
    AdminFriendCreateRequest,
    AdminFriendDetailResponse,
    AdminFriendListResponse,
    AdminFriendUpdateRequest,
)
from app.services.admin_service import (
    create_admin_friend,
    delete_admin_friend,
    list_admin_friend_applies,
    list_admin_friends,
    update_admin_friend,
    update_admin_friend_apply_status,
)

router = APIRouter()


@router.get("/friends", response_model=AdminFriendListResponse, summary="List friends")
def admin_list_friends_api(
    keyword: str = Query(default="", max_length=200),
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminFriendListResponse:
    data = list_admin_friends(session=session, keyword=keyword)
    return AdminFriendListResponse(data=data)


@router.post("/friends", response_model=AdminFriendDetailResponse, summary="Create friend")
def admin_create_friend_api(
    payload: AdminFriendCreateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminFriendDetailResponse:
    try:
        item = create_admin_friend(
            session=session,
            title=payload.title,
            description=payload.description,
            category=payload.category,
            favicon=payload.favicon,
            url=payload.url,
            status=payload.status,
        )
    except IntegrityError as error:
        session.rollback()
        raise HTTPException(status_code=409, detail="Friend URL already exists") from error
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error
    return AdminFriendDetailResponse(data=item)


@router.put("/friends/{friend_id}", response_model=AdminFriendDetailResponse, summary="Update friend")
def admin_update_friend_api(
    friend_id: int,
    payload: AdminFriendUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminFriendDetailResponse:
    data = payload.model_dump(exclude_unset=True)
    try:
        item = update_admin_friend(
            session=session,
            friend_id=friend_id,
            title=data.get("title"),
            description=data.get("description"),
            category=data.get("category"),
            favicon=data.get("favicon"),
            url=data.get("url"),
            status=data.get("status"),
        )
    except IntegrityError as error:
        session.rollback()
        raise HTTPException(status_code=409, detail="Friend URL already exists") from error
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error

    if item is None:
        raise HTTPException(status_code=404, detail="Friend not found")
    return AdminFriendDetailResponse(data=item)


@router.delete("/friends/{friend_id}", response_model=AdminActionResponse, summary="Delete friend")
def admin_delete_friend_api(
    friend_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    deleted = delete_admin_friend(session=session, friend_id=friend_id)
    if not deleted:
        raise HTTPException(status_code=404, detail="Friend not found")
    return AdminActionResponse(message="Friend deleted")


@router.get(
    "/friend-applies",
    response_model=AdminFriendApplyListResponse,
    summary="List friend applications",
)
def admin_list_friend_applies_api(
    status_filter: str = Query(default="", max_length=20, alias="status"),
    keyword: str = Query(default="", max_length=200),
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminFriendApplyListResponse:
    try:
        data = list_admin_friend_applies(
            session=session,
            status=status_filter,
            keyword=keyword,
        )
    except (ValueError, TypeError) as error:
        raise HTTPException(status_code=422, detail=str(error)) from error
    return AdminFriendApplyListResponse(data=data)


@router.put(
    "/friend-applies/{apply_id}/status",
    response_model=AdminFriendApplyDetailResponse,
    summary="Update friend application status",
)
def admin_update_friend_apply_status_api(
    apply_id: int,
    payload: AdminFriendApplyStatusUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminFriendApplyDetailResponse:
    try:
        item = update_admin_friend_apply_status(
            session=session,
            apply_id=apply_id,
            status=payload.status,
            create_friend=payload.create_friend,
            friend_category=payload.friend_category,
        )
    except IntegrityError as error:
        session.rollback()
        raise HTTPException(status_code=409, detail="Friend URL already exists") from error
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error

    if item is None:
        raise HTTPException(status_code=404, detail="Friend application not found")
    return AdminFriendApplyDetailResponse(data=item)
