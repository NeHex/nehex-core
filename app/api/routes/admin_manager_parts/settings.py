from __future__ import annotations

from fastapi import APIRouter, Depends, File, Form, HTTPException, UploadFile
from fastapi import Query
from fastapi.responses import FileResponse
from sqlalchemy.orm import Session

from app.core.admin_security import AdminPrincipal, require_admin_principal
from app.core.database import get_db_session
from app.schemas.admin import (
    AdminAccountSettingsUpdateRequest,
    AdminActionResponse,
    AdminBackupDetailResponse,
    AdminBackupListResponse,
    AdminBackupRestoreRequest,
    AdminMailLogListResponse,
    AdminMailSmtpTestRequest,
    AdminPagination,
    AdminSettingListResponse,
    AdminSettingsUpdateRequest,
)
from app.services.admin_service import (
    SENSITIVE_ADMIN_SETTING_KEYS,
    create_admin_backup,
    get_admin_backup_file_path,
    list_admin_mail_logs,
    list_admin_backups,
    list_admin_settings,
    restore_admin_backup,
    test_admin_mail_smtp,
    upload_and_restore_admin_backup_stream,
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


@router.get(
    "/backups",
    response_model=AdminBackupListResponse,
    summary="List backup archives",
)
def admin_list_backups_api(
    _: AdminPrincipal = Depends(require_admin_principal),
) -> AdminBackupListResponse:
    data = list_admin_backups()
    return AdminBackupListResponse(data=data)


@router.post(
    "/backups",
    response_model=AdminBackupDetailResponse,
    summary="Create site backup",
)
def admin_create_backup_api(
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminBackupDetailResponse:
    try:
        data = create_admin_backup(session=session)
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error
    except Exception as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=f"备份失败: {error}") from error

    return AdminBackupDetailResponse(data=data)


@router.get(
    "/backups/{filename}/download",
    summary="Download backup archive",
)
def admin_download_backup_api(
    filename: str,
    _: AdminPrincipal = Depends(require_admin_principal),
) -> FileResponse:
    try:
        backup_file = get_admin_backup_file_path(filename)
    except FileNotFoundError as error:
        raise HTTPException(status_code=404, detail=str(error)) from error
    except ValueError as error:
        raise HTTPException(status_code=422, detail=str(error)) from error

    return FileResponse(
        path=backup_file,
        media_type="application/gzip",
        filename=backup_file.name,
    )


@router.post(
    "/backups/{filename}/restore",
    response_model=AdminActionResponse,
    summary="Restore from backup archive",
)
def admin_restore_backup_api(
    filename: str,
    payload: AdminBackupRestoreRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    try:
        restore_admin_backup(
            session=session,
            filename=filename,
            confirm_overwrite=payload.confirm_overwrite,
        )
    except FileNotFoundError as error:
        session.rollback()
        raise HTTPException(status_code=404, detail=str(error)) from error
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error
    except Exception as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=f"恢复失败: {error}") from error

    return AdminActionResponse(message="备份恢复完成，现有数据已覆盖")


@router.post(
    "/backups/upload-restore",
    response_model=AdminActionResponse,
    summary="Upload backup archive and restore",
)
def admin_upload_and_restore_backup_api(
    file: UploadFile = File(...),
    confirm_overwrite: bool = Form(default=False),
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    try:
        file.file.seek(0)
        data = upload_and_restore_admin_backup_stream(
            session=session,
            file_name=file.filename or "",
            stream=file.file,
            confirm_overwrite=confirm_overwrite,
        )
    except FileNotFoundError as error:
        session.rollback()
        raise HTTPException(status_code=404, detail=str(error)) from error
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error
    except Exception as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=f"上传恢复失败: {error}") from error

    return AdminActionResponse(message=f"上传并恢复成功：{data.filename}")


@router.post(
    "/settings/mail/test",
    response_model=AdminActionResponse,
    summary="Test SMTP connectivity",
)
def admin_test_mail_smtp_api(
    payload: AdminMailSmtpTestRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    try:
        test_admin_mail_smtp(session=session, payload=payload.model_dump())
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error
    except Exception as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=f"SMTP test failed: {error}") from error

    return AdminActionResponse(message="SMTP 通信成功，测试邮件已发送")


@router.get(
    "/mail-logs",
    response_model=AdminMailLogListResponse,
    summary="List mail sending logs",
)
def admin_list_mail_logs_api(
    status: str = Query(default="all", max_length=20),
    page: int = Query(default=1, ge=1),
    size: int = Query(default=20, ge=1, le=100),
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminMailLogListResponse:
    try:
        data, normalized_page, normalized_size, total, total_pages = list_admin_mail_logs(
            session=session,
            status=status,
            page=page,
            size=size,
        )
    except ValueError as error:
        raise HTTPException(status_code=422, detail=str(error)) from error

    return AdminMailLogListResponse(
        data=data,
        pagination=AdminPagination(
            page=normalized_page,
            size=normalized_size,
            total=total,
            total_pages=total_pages,
        ),
    )
