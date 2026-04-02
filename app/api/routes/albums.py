from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.album import AlbumDetailResponse, AlbumListResponse
from app.services.albums_service import get_album_by_id, list_albums

router = APIRouter(tags=["album"])


@router.get("/album", response_model=AlbumListResponse, summary="获取全部相册")
def get_albums(session: Session = Depends(get_db_session)) -> AlbumListResponse:
    data = list_albums(session)
    return AlbumListResponse(data=data)


@router.get("/album/{album_id}", response_model=AlbumDetailResponse, summary="获取相册详情")
def get_album_detail(
    album_id: int,
    session: Session = Depends(get_db_session),
) -> AlbumDetailResponse:
    album = get_album_by_id(session, album_id)
    if album is None:
        raise HTTPException(status_code=404, detail="Album not found")
    return AlbumDetailResponse(data=album)
