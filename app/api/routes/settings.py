from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.setting import SettingListResponse
from app.services.settings_service import list_settings

router = APIRouter(tags=["settings"])


@router.get("/setting", response_model=SettingListResponse, summary="获取所有设置")
def get_settings(session: Session = Depends(get_db_session)) -> SettingListResponse:
    data = list_settings(session)
    return SettingListResponse(data=data)
