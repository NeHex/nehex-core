from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.setting import (
    SettingListResponse,
    SiteOwnerProfileData,
    SiteOwnerProfileResponse,
    ThemeSettingResponse,
)
from app.services.settings_service import get_site_owner_profile, list_settings, list_theme_settings

router = APIRouter(tags=["settings"])


@router.get("/setting", response_model=SettingListResponse, summary="获取所有设置")
def get_settings(session: Session = Depends(get_db_session)) -> SettingListResponse:
    data = list_settings(session)
    return SettingListResponse(data=data)


@router.get("/setting/theme", response_model=ThemeSettingResponse, summary="获取主题设置")
def get_theme_settings(session: Session = Depends(get_db_session)) -> ThemeSettingResponse:
    data = list_theme_settings(session)
    return ThemeSettingResponse(data=data)


@router.get("/setting/site-owner", response_model=SiteOwnerProfileResponse, summary="获取站长资料")
def get_site_owner(session: Session = Depends(get_db_session)) -> SiteOwnerProfileResponse:
    data = get_site_owner_profile(session)
    return SiteOwnerProfileResponse(data=SiteOwnerProfileData(**data))
