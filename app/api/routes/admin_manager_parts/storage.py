from __future__ import annotations

from fastapi import APIRouter, Depends, File, HTTPException, UploadFile
from sqlalchemy.exc import IntegrityError
from sqlalchemy.orm import Session

from app.core.admin_security import AdminPrincipal, require_admin_principal
from app.core.database import get_db_session
from app.schemas.admin import AdminStorageUploadResponse
from app.services.admin_service import upload_media_image

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
        item = upload_media_image(
            session,
            file_name=file_name,
            content_type=content_type,
            content=content,
            image_only=True,
        )
    except ValueError as error:
        raise HTTPException(status_code=422, detail=str(error)) from error
    except RuntimeError as error:
        raise HTTPException(status_code=500, detail=str(error)) from error
    except IntegrityError as error:
        session.rollback()
        raise HTTPException(status_code=409, detail="Image record already exists") from error

    return AdminStorageUploadResponse(
        data={
            "provider": item.provider,
            "key": item.key,
            "url": item.url,
        },
    )
