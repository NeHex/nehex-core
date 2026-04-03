from __future__ import annotations

from datetime import datetime
from typing import Optional

from sqlalchemy import DateTime, Integer, String, Text, func, text
from sqlalchemy.orm import Mapped, mapped_column

from app.models.base import Base


class Project(Base):
    __tablename__ = "project"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True, nullable=False)
    title: Mapped[str] = mapped_column(String(255), nullable=False)
    cover: Mapped[Optional[str]] = mapped_column(String(500), nullable=True)
    category: Mapped[Optional[str]] = mapped_column(String(120), nullable=True, index=True)
    description: Mapped[Optional[str]] = mapped_column(String(1000), nullable=True)
    content: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    tech_stack: Mapped[Optional[str]] = mapped_column(String(500), nullable=True)
    project_url: Mapped[Optional[str]] = mapped_column(String(1000), nullable=True)
    github_url: Mapped[Optional[str]] = mapped_column(String(1000), nullable=True)
    sort: Mapped[int] = mapped_column(Integer, nullable=False, server_default=text("0"), default=0)
    status: Mapped[int] = mapped_column(Integer, nullable=False, server_default=text("1"), default=1)
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
