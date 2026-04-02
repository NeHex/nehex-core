from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.friend import FriendListResponse
from app.services.friends_service import list_friends

router = APIRouter(tags=["friend"])


@router.get("/friend", response_model=FriendListResponse, summary="获取全部友链")
def get_friends(session: Session = Depends(get_db_session)) -> FriendListResponse:
    data = list_friends(session)
    return FriendListResponse(data=data)
