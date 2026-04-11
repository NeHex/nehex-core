from __future__ import annotations

from fastapi import APIRouter, Depends, File, HTTPException, UploadFile
from sqlalchemy.exc import IntegrityError
from sqlalchemy.orm import Session

from app.core.admin_security import AdminPrincipal, require_admin_principal
from app.core.database import get_db_session
from app.schemas.admin import (
    AdminActionResponse,
    AdminMediaFolderCreateRequest,
    AdminMediaFolderDetailResponse,
    AdminMediaFolderRenameRequest,
    AdminMediaImageDetailResponse,
    AdminMediaImageListResponse,
    AdminMediaImageMoveRequest,
    AdminMediaLibraryData,
    AdminMediaLibraryResponse,
)
from app.services.admin_service import (
    create_media_folder,
    delete_media_folder,
    delete_media_image,
    list_media_folders_with_counts,
    list_media_images_by_folder,
    list_uncategorized_media_images,
    media_folder_exists,
    move_media_images_to_folder,
    rename_media_folder,
    upload_media_image,
)

router = APIRouter()


@router.get("/media/library", response_model=AdminMediaLibraryResponse, summary="Get media library overview")
def admin_get_media_library(
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminMediaLibraryResponse:
    folders = list_media_folders_with_counts(session)
    uncategorized = list_uncategorized_media_images(session)
    return AdminMediaLibraryResponse(data=AdminMediaLibraryData(folders=folders, uncategorized=uncategorized))


@router.get(
    "/media/folders/{folder_id}/images",
    response_model=AdminMediaImageListResponse,
    summary="Get images in a media folder",
)
def admin_get_media_folder_images(
    folder_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminMediaImageListResponse:
    if not media_folder_exists(session, folder_id):
        raise HTTPException(status_code=404, detail="Media folder not found")
    data = list_media_images_by_folder(session, folder_id)
    return AdminMediaImageListResponse(data=data)


@router.post("/media/folders", response_model=AdminMediaFolderDetailResponse, summary="Create media folder")
def admin_create_media_folder(
    payload: AdminMediaFolderCreateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminMediaFolderDetailResponse:
    try:
        item = create_media_folder(session, name=payload.name)
    except IntegrityError as error:
        session.rollback()
        raise HTTPException(status_code=409, detail="Folder name already exists") from error
    return AdminMediaFolderDetailResponse(data=item)


@router.put("/media/folders/{folder_id}", response_model=AdminMediaFolderDetailResponse, summary="Rename media folder")
def admin_rename_media_folder(
    folder_id: int,
    payload: AdminMediaFolderRenameRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminMediaFolderDetailResponse:
    try:
        item = rename_media_folder(session, folder_id, name=payload.name)
    except IntegrityError as error:
        session.rollback()
        raise HTTPException(status_code=409, detail="Folder name already exists") from error
    if item is None:
        raise HTTPException(status_code=404, detail="Media folder not found")
    return AdminMediaFolderDetailResponse(data=item)


@router.delete("/media/folders/{folder_id}", response_model=AdminActionResponse, summary="Delete media folder")
def admin_delete_media_folder(
    folder_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    moved = delete_media_folder(session, folder_id)
    if moved is None:
        raise HTTPException(status_code=404, detail="Media folder not found")
    return AdminActionResponse(message=f"Folder deleted and moved {moved} asset(s) to uncategorized")


@router.post("/media/images/upload", response_model=AdminMediaImageDetailResponse, summary="Upload media asset")
async def admin_upload_media_image(
    file: UploadFile = File(...),
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminMediaImageDetailResponse:
    try:
        content = await file.read()
        file_name = file.filename or "image"
        content_type = file.content_type or ""
        item = upload_media_image(
            session,
            file_name=file_name,
            content_type=content_type,
            content=content,
        )
    except ValueError as error:
        raise HTTPException(status_code=422, detail=str(error)) from error
    except RuntimeError as error:
        raise HTTPException(status_code=500, detail=str(error)) from error
    except IntegrityError as error:
        session.rollback()
        raise HTTPException(status_code=409, detail="Image record already exists") from error
    return AdminMediaImageDetailResponse(data=item)


@router.post("/media/images/move", response_model=AdminActionResponse, summary="Move media assets to folder")
def admin_move_media_images(
    payload: AdminMediaImageMoveRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    if payload.folder_id is not None and not media_folder_exists(session, payload.folder_id):
        raise HTTPException(status_code=404, detail="Media folder not found")

    changed = move_media_images_to_folder(
        session,
        image_ids=payload.ids,
        folder_id=payload.folder_id,
    )
    return AdminActionResponse(message=f"Moved {changed} asset(s)")


@router.delete("/media/images/{image_id}", response_model=AdminActionResponse, summary="Delete media image")
def admin_delete_media_image(
    image_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    deleted = delete_media_image(session, image_id)
    if not deleted:
        raise HTTPException(status_code=404, detail="Media image not found")
    return AdminActionResponse(message="Media image deleted")
