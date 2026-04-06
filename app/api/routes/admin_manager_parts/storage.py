from __future__ import annotations

from fastapi import APIRouter, Depends, File, HTTPException, UploadFile
from sqlalchemy.orm import Session

from app.core.admin_security import AdminPrincipal, require_admin_principal
from app.core.database import get_db_session
from app.schemas.admin import AdminStorageUploadResponse
from app.services.object_storage_service import upload_image_to_object_storage

router = APIRouter()


@router.post("/storage/upload", response_model=AdminStorageUploadResponse, summary="Upload image to object storage")
async def admin_upload_storage_image_api(
    file: UploadFile = File(...),
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminStorageUploadResponse:
    try:
        content = await file.read()
        file_name = file.filename or "image"
        content_type = file.content_type or ""
        data = upload_image_to_object_storage(
            session,
            file_name=file_name,
            content_type=content_type,
            content=content,
        )
    except ValueError as error:
        raise HTTPException(status_code=422, detail=str(error)) from error
    except RuntimeError as error:
        raise HTTPException(status_code=500, detail=str(error)) from error

    return AdminStorageUploadResponse(data=data)
