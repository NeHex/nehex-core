from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session

from app.core.admin_security import AdminPrincipal, require_admin_principal
from app.core.database import get_db_session
from app.schemas.admin import (
    AdminAccountSettingsUpdateRequest,
    AdminSettingListResponse,
    AdminSettingsUpdateRequest,
)
from app.services.admin_service import (
    SENSITIVE_ADMIN_SETTING_KEYS,
    list_admin_settings,
    update_admin_account_settings,
    update_admin_settings,
)

router = APIRouter()


@router.get("/settings", response_model=AdminSettingListResponse, summary="List settings")
def admin_list_settings_api(
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminSettingListResponse:
    data = list_admin_settings(session=session)
    return AdminSettingListResponse(data=data)


@router.put("/settings", response_model=AdminSettingListResponse, summary="Update settings")
def admin_update_settings_api(
    payload: AdminSettingsUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminSettingListResponse:
    items_payload: list[dict] = []
    for item in payload.items:
        if item.setting_key in SENSITIVE_ADMIN_SETTING_KEYS:
            raise HTTPException(
                status_code=422,
                detail=f"Setting {item.setting_key} must be updated via /admin-api/settings/account",
            )
        items_payload.append(
            {
                "setting_key": item.setting_key,
                "setting_content": item.setting_content,
                "setting_type": item.setting_type,
                "description": item.description,
                "has_description": "description" in item.model_fields_set,
            },
        )

    try:
        data = update_admin_settings(session=session, items=items_payload)
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error

    return AdminSettingListResponse(data=data)


@router.put(
    "/settings/account",
    response_model=AdminSettingListResponse,
    summary="Update account settings",
)
def admin_update_account_settings_api(
    payload: AdminAccountSettingsUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminSettingListResponse:
    has_account = "account" in payload.model_fields_set
    has_new_password = "new_password" in payload.model_fields_set

    if not has_account and not has_new_password:
        raise HTTPException(status_code=422, detail="No account settings fields to update")

    try:
        data = update_admin_account_settings(
            session=session,
            account=payload.account if has_account else None,
            new_password=payload.new_password if has_new_password else None,
        )
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error

    return AdminSettingListResponse(data=data)
