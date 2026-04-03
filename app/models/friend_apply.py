from __future__ import annotations

from datetime import datetime
from typing import Optional

from sqlalchemy import DateTime, Enum, Integer, String, func, text
from sqlalchemy.orm import Mapped, mapped_column

from app.models.base import Base


class FriendApply(Base):
    __tablename__ = "friend_apply"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True, nullable=False)
    site_title: Mapped[str] = mapped_column(String(255), nullable=False)
    site_url: Mapped[str] = mapped_column(String(500), nullable=False, index=True)
    site_description: Mapped[Optional[str]] = mapped_column(String(1000), nullable=True)
    site_icon: Mapped[Optional[str]] = mapped_column(String(500), nullable=True)
    contact: Mapped[Optional[str]] = mapped_column(String(255), nullable=True)
    status: Mapped[str] = mapped_column(
        Enum("pending", "approved", "rejected", "blocked", name="friend_apply_status"),
        nullable=False,
        server_default=text("'pending'"),
        default="pending",
    )
    ip: Mapped[Optional[str]] = mapped_column(String(50), nullable=True)
    user_agent: Mapped[Optional[str]] = mapped_column(String(255), nullable=True)
    create_time: Mapped[datetime] = mapped_column(
        DateTime,
        nullable=False,
        server_default=func.current_timestamp(),
    )
    update_time: Mapped[datetime] = mapped_column(
        DateTime,
        nullable=False,
        server_default=func.current_timestamp(),
        onupdate=func.current_timestamp(),
    )
