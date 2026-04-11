from __future__ import annotations

from datetime import datetime
from typing import Optional

from sqlalchemy import DateTime, ForeignKey, Integer, String, func, text
from sqlalchemy.orm import Mapped, mapped_column

from app.models.base import Base


class MediaImage(Base):
    __tablename__ = "media_image"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True, nullable=False)
    folder_id: Mapped[Optional[int]] = mapped_column(
        Integer,
        ForeignKey("media_folder.id", ondelete="SET NULL"),
        nullable=True,
        index=True,
    )
    provider: Mapped[str] = mapped_column(String(40), nullable=False, default="local")
    storage_key: Mapped[str] = mapped_column(String(800), nullable=False)
    url: Mapped[str] = mapped_column(String(1200), nullable=False)
    file_name: Mapped[Optional[str]] = mapped_column(String(255), nullable=True)
    content_type: Mapped[Optional[str]] = mapped_column(String(120), nullable=True)
    size_bytes: Mapped[int] = mapped_column(Integer, nullable=False, server_default=text("0"), default=0)
    create_time: Mapped[datetime] = mapped_column(
        DateTime,
        nullable=False,
        server_default=func.current_timestamp(),
    )
