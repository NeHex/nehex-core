from __future__ import annotations

from typing import Optional

from sqlalchemy import desc, func, select
from sqlalchemy.orm import Session

from app.models.media_folder import MediaFolder
from app.models.media_image import MediaImage
from app.schemas.admin import AdminMediaFolderItem, AdminMediaImageItem
from app.services.object_storage_service import upload_file_to_object_storage, upload_image_to_object_storage


def _normalize_optional_text(value: Optional[str]) -> Optional[str]:
    if value is None:
        return None
    normalized = value.strip()
    return normalized or None


def _map_media_folder_item(folder: MediaFolder, image_count: int = 0) -> AdminMediaFolderItem:
    return AdminMediaFolderItem(
        id=int(folder.id),
        name=folder.name,
        image_count=max(0, int(image_count)),
        create_time=folder.create_time,
        update_time=folder.update_time,
    )


def _map_media_image_item(row: MediaImage) -> AdminMediaImageItem:
    media_type = detect_media_type(row.content_type, row.file_name)
    return AdminMediaImageItem(
        id=int(row.id),
        folder_id=int(row.folder_id) if row.folder_id is not None else None,
        media_type=media_type,
        provider=row.provider,
        key=row.storage_key,
        url=row.url,
        file_name=row.file_name,
        content_type=row.content_type,
        size_bytes=max(0, int(row.size_bytes)),
        create_time=row.create_time,
    )


def detect_media_type(content_type: Optional[str], file_name: Optional[str]) -> str:
    normalized_content_type = (content_type or "").strip().lower()
    if normalized_content_type.startswith("image/"):
        return "image"
    if normalized_content_type.startswith("video/"):
        return "video"
    if normalized_content_type.startswith("audio/"):
        return "audio"

    normalized_name = (file_name or "").strip().lower()
    if normalized_name.endswith((".jpg", ".jpeg", ".png", ".webp", ".gif", ".bmp", ".svg", ".avif")):
        return "image"
    if normalized_name.endswith((".mp4", ".webm", ".mov", ".mkv", ".avi", ".ogv")):
        return "video"
    if normalized_name.endswith((".mp3", ".wav", ".ogg", ".flac", ".aac", ".m4a")):
        return "audio"
    return "file"


def list_media_folders_with_counts(session: Session) -> list[AdminMediaFolderItem]:
    count_subquery = (
        select(
            MediaImage.folder_id.label("folder_id"),
            func.count(MediaImage.id).label("image_count"),
        )
        .where(MediaImage.folder_id.is_not(None))
        .group_by(MediaImage.folder_id)
        .subquery()
    )
    stmt = (
        select(MediaFolder, func.coalesce(count_subquery.c.image_count, 0))
        .outerjoin(count_subquery, count_subquery.c.folder_id == MediaFolder.id)
        .order_by(MediaFolder.name.asc(), MediaFolder.id.asc())
    )
    rows = session.execute(stmt).all()
    return [_map_media_folder_item(folder, int(image_count or 0)) for folder, image_count in rows]


def list_uncategorized_media_images(session: Session) -> list[AdminMediaImageItem]:
    stmt = (
        select(MediaImage)
        .where(MediaImage.folder_id.is_(None))
        .order_by(desc(MediaImage.create_time), desc(MediaImage.id))
    )
    rows = session.execute(stmt).scalars().all()
    return [_map_media_image_item(row) for row in rows]


def list_media_images_by_folder(session: Session, folder_id: int) -> list[AdminMediaImageItem]:
    stmt = (
        select(MediaImage)
        .where(MediaImage.folder_id == folder_id)
        .order_by(desc(MediaImage.create_time), desc(MediaImage.id))
    )
    rows = session.execute(stmt).scalars().all()
    return [_map_media_image_item(row) for row in rows]


def media_folder_exists(session: Session, folder_id: int) -> bool:
    stmt = select(MediaFolder.id).where(MediaFolder.id == folder_id).limit(1)
    return session.execute(stmt).scalar_one_or_none() is not None


def create_media_folder(session: Session, *, name: str) -> AdminMediaFolderItem:
    row = MediaFolder(
        name=name.strip(),
    )
    session.add(row)
    session.commit()
    session.refresh(row)
    return _map_media_folder_item(row, image_count=0)


def rename_media_folder(
    session: Session,
    folder_id: int,
    *,
    name: str,
) -> Optional[AdminMediaFolderItem]:
    stmt = select(MediaFolder).where(MediaFolder.id == folder_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    row.name = name.strip()
    session.commit()
    session.refresh(row)

    count_stmt = select(func.count(MediaImage.id)).where(MediaImage.folder_id == folder_id)
    count = int(session.execute(count_stmt).scalar() or 0)
    return _map_media_folder_item(row, image_count=count)


def delete_media_folder(session: Session, folder_id: int) -> Optional[int]:
    stmt = select(MediaFolder).where(MediaFolder.id == folder_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None

    image_stmt = select(MediaImage).where(MediaImage.folder_id == folder_id)
    images = session.execute(image_stmt).scalars().all()
    moved = 0
    for image in images:
        image.folder_id = None
        moved += 1

    session.delete(row)
    session.commit()
    return moved


def upload_media_image(
    session: Session,
    *,
    file_name: str,
    content_type: str,
    content: bytes,
    image_only: bool = False,
) -> AdminMediaImageItem:
    upload_func = upload_image_to_object_storage if image_only else upload_file_to_object_storage
    data = upload_func(
        session,
        file_name=file_name,
        content_type=content_type,
        content=content,
    )
    row = MediaImage(
        folder_id=None,
        provider=str(data.get("provider") or "").strip() or "local",
        storage_key=str(data.get("key") or "").strip(),
        url=str(data.get("url") or "").strip(),
        file_name=_normalize_optional_text(file_name),
        content_type=_normalize_optional_text(content_type),
        size_bytes=max(0, int(len(content))),
    )
    session.add(row)
    session.commit()
    session.refresh(row)
    return _map_media_image_item(row)


def move_media_images_to_folder(
    session: Session,
    *,
    image_ids: list[int],
    folder_id: Optional[int],
) -> int:
    normalized_ids = sorted({int(item) for item in image_ids if int(item) > 0})
    if not normalized_ids:
        return 0

    stmt = select(MediaImage).where(MediaImage.id.in_(normalized_ids))
    rows = session.execute(stmt).scalars().all()
    if not rows:
        return 0

    target_folder_id = int(folder_id) if folder_id is not None else None
    changed = 0
    for row in rows:
        if row.folder_id == target_folder_id:
            continue
        row.folder_id = target_folder_id
        changed += 1

    if changed > 0:
        session.commit()
    return changed


def delete_media_image(session: Session, image_id: int) -> bool:
    stmt = select(MediaImage).where(MediaImage.id == image_id).limit(1)
    row = session.execute(stmt).scalars().first()
    if row is None:
        return False

    session.delete(row)
    session.commit()
    return True
