from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.daily import DailyListResponse
from app.services.dailies_service import list_dailies

router = APIRouter(tags=["daily"])


@router.get("/daily", response_model=DailyListResponse, summary="获取全部日常记录")
def get_dailies(session: Session = Depends(get_db_session)) -> DailyListResponse:
    data = list_dailies(session)
    return DailyListResponse(data=data)
