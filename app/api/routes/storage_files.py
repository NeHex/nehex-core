from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import FileResponse
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.services.object_storage_service import resolve_local_storage_file_path

router = APIRouter(tags=["storage"])


@router.get("/storage/{file_path:path}", include_in_schema=False)
def get_local_storage_file(
    file_path: str,
    session: Session = Depends(get_db_session),
) -> FileResponse:
    target = resolve_local_storage_file_path(session, file_path)
    if target is None:
        raise HTTPException(status_code=404, detail="Storage file not found")
    return FileResponse(target)
